use anyhow::Result;
use clap::Parser;
use sashiko::{
    agent::{Agent, tools::ToolBox, prompts::PromptRegistry},
    ai::gemini::GeminiClient,
    db::Database,
    git_ops::GitWorktree,
    settings::Settings,
};
use std::path::PathBuf;
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    patchset: i64,

    #[arg(long, default_value = "review-prompts")]
    prompts: PathBuf,

    #[arg(long, default_value = "gemini-1.5-pro-latest")]
    model: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let settings = Settings::new().unwrap();

    let db = Database::new(&settings.database).await?;

    let patchset_json = db.get_patchset_details(args.patchset).await?
        .ok_or_else(|| anyhow::anyhow!("Patchset {} not found", args.patchset))?;

    info!("Reviewing patchset: {}", patchset_json["subject"]);

    let repo_path = PathBuf::from(&settings.git.repository_path);
    let worktree = GitWorktree::new(&repo_path, "HEAD").await?;

    info!("Created worktree at {:?}", worktree.path);

    let diffs = db.get_patch_diffs(args.patchset).await?;
    info!("Found {} patches to apply", diffs.len());
    
    for (idx, diff) in diffs {
        info!("Applying patch part {}", idx);
        if let Err(e) = worktree.apply_raw_diff(&diff).await {
            info!("Failed to apply patch {}: {}. Continuing...", idx, e);
        }
    }

    let client = GeminiClient::new(args.model)?;
    let tools = ToolBox::new(worktree.path.clone(), args.prompts.clone());
    let prompts = PromptRegistry::new(args.prompts);
    
    let mut agent = Agent::new(client, tools, prompts);
    
    match agent.run(patchset_json).await {
        Ok(review) => println!("Review:\n{}", review),
        Err(e) => eprintln!("Agent failed: {}", e),
    }

    worktree.remove().await?;

    Ok(())
}
