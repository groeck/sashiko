use anyhow::Result;
use std::path::PathBuf;
use tokio::fs;
use serde_json::Value;

pub struct PromptRegistry {
    base_dir: PathBuf,
}

impl PromptRegistry {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub async fn get_system_prompt(&self) -> Result<String> {
        let identity = fs::read_to_string(self.base_dir.join("core/identity.md")).await
            .unwrap_or_else(|_| "You are a Linux Kernel Maintainer.".to_string());
        let workflow = fs::read_to_string(self.base_dir.join("core/review_workflow.md")).await
            .unwrap_or_else(|_| "Follow the standard review process.".to_string());
        
        Ok(format!("{}\n\n{}", identity, workflow))
    }

    pub async fn build_context_prompt(&self, patchset: &Value) -> Result<String> {
        // analyze patchset to find touched files and guess subsystems
        let mut instructions = Vec::new();
        instructions.push("Specific guidelines for this patchset:".to_string());

        // Always add technical patterns
        if let Ok(content) = fs::read_to_string(self.base_dir.join("technical-patterns.md")).await {
             instructions.push(format!("\n## Technical Patterns\n{}", content));
        }

        // Detect subsystems from touched files
        // We iterate over "patches" in the patchset JSON
        let patches = patchset["patches"].as_array();
        if let Some(_patches) = patches {
             // We don't have file lists in patchset summary usually, unless we parse diffs or have it stored.
             // The patchset details JSON has "patches" list.
             // We'd need to fetch file stats or just look at diffs if available.
             // For now, let's assume we can't easily get full file list without expensive calls.
             // But the Agent can call tools.
             // Here we are building the initial prompt.
             
             // If we can't detect, we just add general advice or ask the model to check.
             instructions.push("Please analyze the touched files and apply relevant subsystem rules.".to_string());
        }

        // Add False Positive Guide
        if let Ok(content) = fs::read_to_string(self.base_dir.join("false-positive-guide.md")).await {
             instructions.push(format!("\n## False Positive Guide\n{}", content));
        }

        Ok(instructions.join("\n"))
    }
}
