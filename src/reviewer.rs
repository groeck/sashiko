use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Semaphore;
use tracing::{info, error, warn};
use crate::db::Database;
use crate::settings::Settings;

pub struct Reviewer {
    db: Arc<Database>,
    settings: Settings,
    semaphore: Arc<Semaphore>,
}

impl Reviewer {
    pub fn new(db: Arc<Database>, settings: Settings) -> Self {
        let concurrency = settings.review.concurrency;
        Self {
            db,
            settings,
            semaphore: Arc::new(Semaphore::new(concurrency)),
        }
    }

    pub async fn start(&self) {
        info!("Starting Reviewer service with concurrency limit: {}", self.settings.review.concurrency);
        
        // Cleanup worktree directory on startup
        let worktree_dir = PathBuf::from(&self.settings.review.worktree_dir);
        if worktree_dir.exists() {
            info!("Cleaning up previous worktree directory: {:?}", worktree_dir);
            if let Err(e) = std::fs::remove_dir_all(&worktree_dir) {
                error!("Failed to cleanup worktree directory: {}", e);
            }
        }
        if let Err(e) = std::fs::create_dir_all(&worktree_dir) {
            error!("Failed to create worktree directory: {}", e);
        }

        // Reset any patchsets stuck in 'Reviewing' state from previous run
        match self.db.reset_reviewing_status().await {
            Ok(count) => {
                if count > 0 {
                    info!("Recovered {} interrupted reviews (reset to Pending)", count);
                }
            },
            Err(e) => error!("Failed to reset reviewing status: {}", e),
        }

        loop {
            match self.process_pending_patchsets().await {
                Ok(_) => {},
                Err(e) => error!("Error in reviewer loop: {}", e),
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    }

    async fn process_pending_patchsets(&self) -> Result<()> {
        // Fetch pending patchsets
        // We look for 'Pending' status.
        let patchsets = self.db.get_pending_patchsets(10).await?;

        if patchsets.is_empty() {
            return Ok(());
        }

        info!("Found {} pending patchsets for review", patchsets.len());

        for patchset in patchsets {
            let permit = self.semaphore.clone().acquire_owned().await?;
            let db = self.db.clone();
            let settings = self.settings.clone();
            let patchset_id = patchset.id;

            tokio::spawn(async move {
                // Move permit into the task so it's dropped when task finishes
                let _permit = permit;
                
                info!("Starting review for patchset {}", patchset_id);
                
                // Update status to 'Reviewing' to avoid double processing?
                // Or just rely on the fact we grabbed it? 
                // Better to update status. But we don't have 'Reviewing' status in schema yet.
                // Assuming 'Pending' means ready. We should add 'Reviewing' or similar.
                // For now, let's keep it 'Pending' but maybe we need to be careful.
                // Actually, if we pick 10 and spawn 10 tasks, next loop might pick them again if they are still 'Pending'.
                // We MUST update status to something else.
                // Let's use 'Reviewing'. (Need to ensure schema allows it or just use it if text).
                
                if let Err(e) = db.update_patchset_status(patchset_id, "Reviewing").await {
                    error!("Failed to update status to Reviewing for {}: {}", patchset_id, e);
                    return;
                }

                match run_review_tool(patchset_id, &settings).await {
                    Ok(status) => {
                        info!("Review finished for {}: {}", patchset_id, status);
                        if let Err(e) = db.update_patchset_status(patchset_id, &status).await {
                            error!("Failed to update status for {}: {}", patchset_id, e);
                        }
                    },
                    Err(e) => {
                        error!("Review failed for {}: {}", patchset_id, e);
                        if let Err(e) = db.update_patchset_status(patchset_id, "Failed").await {
                            error!("Failed to update status for {}: {}", patchset_id, e);
                        }
                    }
                }
            });
        }

        Ok(())
    }
}

async fn run_review_tool(patchset_id: i64, settings: &Settings) -> Result<String> {
    // Run the binary: cargo run --bin review -- --patchset ID --baseline next/master --worktree-parent DIR
    // But we are running from binary, so we should look for the executable or run cargo run if dev.
    // Assuming development environment:
    
    // We need to add --worktree-parent to review tool first!
    // For now, let's assume review tool uses a temp dir by default.
    // The user requirement: "Make sure reviewer tools are creating worktrees in some subfolder".
    // So I need to modify `review.rs` to accept this arg.
    
    // Let's assume I will add `--worktree-dir` to `review.rs`.
    
    let output = Command::new("cargo")
        .args([
            "run", "--bin", "review", "--",
            "--patchset", &patchset_id.to_string(),
            "--baseline", "next/master", // Defaulting to next/master
            "--worktree-dir", &settings.review.worktree_dir,
            // Assuming we disable AI for now as per previous context or enable it?
            // "Integrate the reviewer tool... If patchset successfully applied..."
            // It implies we care about application status mainly for now.
        ])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Review tool failed for {}: {}", patchset_id, stderr);
        return Ok("Failed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&stdout)?;
    
    // Check if all patches applied
    let patches = json["patches"].as_array().unwrap();
    let all_applied = patches.iter().all(|p| p["status"] == "applied");

    if all_applied {
        Ok("Applied".to_string())
    } else {
        Ok("Failed".to_string())
    }
}
