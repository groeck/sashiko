use crate::ai::gemini::{FunctionDeclaration, Tool};
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;

pub struct ToolBox {
    worktree_path: PathBuf,
    prompts_dir: PathBuf,
}

impl ToolBox {
    pub fn new(worktree_path: PathBuf, prompts_dir: PathBuf) -> Self {
        Self {
            worktree_path,
            prompts_dir,
        }
    }

    pub fn get_declarations(&self) -> Tool {
        Tool {
            function_declarations: vec![
                FunctionDeclaration {
                    name: "read_file".to_string(),
                    description: "Read the content of a file (with optional line range)."
                        .to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Relative path to the file." },
                            "start_line": { "type": "integer", "description": "1-based start line (optional)." },
                            "end_line": { "type": "integer", "description": "1-based end line (optional)." }
                        },
                        "required": ["path"]
                    }),
                },
                FunctionDeclaration {
                    name: "git_blame".to_string(),
                    description: "Show what revision and author last modified each line of a file."
                        .to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Relative path to the file." },
                            "start_line": { "type": "integer", "description": "1-based start line (optional)." },
                            "end_line": { "type": "integer", "description": "1-based end line (optional)." }
                        },
                        "required": ["path"]
                    }),
                },
                FunctionDeclaration {
                    name: "git_diff".to_string(),
                    description: "Show changes between commits, commit and working tree, etc."
                        .to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "args": { "type": "array", "items": { "type": "string" }, "description": "Arguments for git diff (e.g., ['HEAD^', 'HEAD'])." }
                        },
                        "required": ["args"]
                    }),
                },
                FunctionDeclaration {
                    name: "git_show".to_string(),
                    description: "Show various types of objects (blobs, trees, tags and commits)."
                        .to_string(),
                    parameters: json!({
                         "type": "object",
                         "properties": {
                             "object": { "type": "string", "description": "The object to show (e.g. 'HEAD:README.md')." }
                         },
                         "required": ["object"]
                    }),
                },
                FunctionDeclaration {
                    name: "list_dir".to_string(),
                    description: "List files in a directory.".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Directory path." }
                        },
                        "required": ["path"]
                    }),
                },
                FunctionDeclaration {
                    name: "write_file".to_string(),
                    description: "Write content to a file in the worktree. Primarily used for 'review-inline.txt'."
                        .to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Relative path to the file (e.g., 'review-inline.txt')." },
                            "content": { "type": "string", "description": "Content to write." }
                        },
                        "required": ["path", "content"]
                    }),
                },
                FunctionDeclaration {
                    name: "read_prompt".to_string(),
                    description: "Read a specific prompt documentation file.".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "Name of the prompt file (e.g., 'core/identity.md')." }
                        },
                        "required": ["name"]
                    }),
                },
                FunctionDeclaration {
                    name: "search_file_content".to_string(),
                    description: "Search for a pattern in files using grep. Returns matching lines with context.".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "pattern": { "type": "string", "description": "Regex pattern to search for." },
                            "path": { "type": "string", "description": "Directory to search in (defaults to root)." },
                            "context_lines": { "type": "integer", "description": "Number of context lines to show (default 0)." }
                        },
                        "required": ["pattern"]
                    }),
                },
                FunctionDeclaration {
                    name: "find_files".to_string(),
                    description: "Find files matching a glob pattern (e.g., '*.rs', 'src/**/mod.rs').".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "pattern": { "type": "string", "description": "Glob pattern to match." },
                            "path": { "type": "string", "description": "Directory to search in (defaults to root)." }
                        },
                        "required": ["pattern"]
                    }),
                },
            ],
        }
    }

    pub async fn call(&self, name: &str, args: Value) -> Result<Value> {
        match name {
            "read_file" => self.read_file(args).await,
            "write_file" => self.write_file(args).await,
            "git_blame" => self.git_blame(args).await,
            "git_diff" => self.git_diff(args).await,
            "git_show" => self.git_show(args).await,
            "list_dir" => self.list_dir(args).await,
            "read_prompt" => self.read_prompt(args).await,
            "search_file_content" => self.search_file_content(args).await,
            "find_files" => self.find_files(args).await,
            _ => Err(anyhow!("Unknown tool: {}", name)),
        }
    }

    fn truncate_output(&self, output: String) -> String {
        const MAX_LEN: usize = 32 * 1024; // 32KB limit
        if output.len() > MAX_LEN {
            let mut truncated = output[..MAX_LEN].to_string();
            truncated.push_str(&format!(
                "\n... (truncated, total length: {} bytes)",
                output.len()
            ));
            truncated
        } else {
            output
        }
    }

    async fn read_file(&self, args: Value) -> Result<Value> {
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing path"))?;
        let start_line = args["start_line"].as_u64().map(|v| v as usize);
        let end_line = args["end_line"].as_u64().map(|v| v as usize);

        let path = self.validate_path(path_str, &self.worktree_path)?;
        let content = fs::read_to_string(path).await?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let (start, end) = match (start_line, end_line) {
            (Some(s), Some(e)) => (s.max(1) - 1, e.min(total_lines)),
            (Some(s), None) => (s.max(1) - 1, total_lines),
            (None, Some(e)) => (0, e.min(total_lines)),
            (None, None) => (0, total_lines),
        };

        if start >= total_lines {
            return Ok(json!({ "content": "", "lines_read": 0, "total_lines": total_lines }));
        }

        let slice = &lines[start..end];
        let result = slice.join("\n");
        let truncated = self.truncate_output(result);

        Ok(json!({
            "content": truncated,
            "lines_read": slice.len(),
            "total_lines": total_lines,
            "start_line": start + 1,
            "end_line": end
        }))
    }

    async fn write_file(&self, args: Value) -> Result<Value> {
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing path"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing content"))?;

        let path = self.validate_path(path_str, &self.worktree_path)?;
        fs::write(path, content).await?;

        Ok(json!({ "status": "success" }))
    }

    async fn git_blame(&self, args: Value) -> Result<Value> {
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing path"))?;
        let start_line = args["start_line"].as_u64();
        let end_line = args["end_line"].as_u64();

        let mut cmd = Command::new("git");
        cmd.current_dir(&self.worktree_path).arg("blame");

        if let (Some(s), Some(e)) = (start_line, end_line) {
            cmd.arg(format!("-L{},{}", s, e));
        }

        cmd.arg("--").arg(path_str);

        let output = cmd.output().await?;
        if !output.status.success() {
            return Err(anyhow!(
                "git blame failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let content = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(json!({ "content": self.truncate_output(content) }))
    }

    async fn git_diff(&self, args: Value) -> Result<Value> {
        let diff_args = args["args"]
            .as_array()
            .ok_or_else(|| anyhow!("Missing args"))?;
        let diff_args_str: Vec<&str> = diff_args.iter().filter_map(|v| v.as_str()).collect();

        let output = Command::new("git")
            .current_dir(&self.worktree_path)
            .arg("diff")
            .args(&diff_args_str)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut error_msg = format!("git diff failed: {}", stderr);

            if stderr.contains("unknown revision") || stderr.contains("ambiguous argument") {
                error_msg.push_str("\nHint: The repository might be a shallow clone (depth=1). You cannot access history beyond HEAD. Try using 'HEAD' or diffing against specific files without revision ranges.");
            }

            return Err(anyhow!(error_msg));
        }

        let content = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(json!({ "content": self.truncate_output(content) }))
    }

    async fn git_show(&self, args: Value) -> Result<Value> {
        let object = args["object"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing object"))?;

        let output = Command::new("git")
            .current_dir(&self.worktree_path)
            .arg("show")
            .arg(object)
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!(
                "git show failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let content = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(json!({ "content": self.truncate_output(content) }))
    }

    async fn list_dir(&self, args: Value) -> Result<Value> {
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing path"))?;
        let path = self.validate_path(path_str, &self.worktree_path)?;

        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(path).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let ty = if entry.file_type().await?.is_dir() {
                "dir"
            } else {
                "file"
            };
            entries.push(json!({ "name": entry.file_name().to_string_lossy(), "type": ty }));
        }

        // List dir can also be huge if directory has many files, but usually JSON structure overhead is the issue.
        // We probably don't need to truncate list_dir unless it's thousands of files.
        // But for safety, let's limit the number of entries if needed, or just let it be.
        // 32KB text limit for file content is reasonable.
        // For list_dir, we can limit entries count.
        if entries.len() > 1000 {
            entries.truncate(1000);
            // We can't easily signal truncation in JSON array without changing structure or adding a dummy entry.
            // Let's leave list_dir as is for now, it's less likely to produce GBs than git show.
        }

        Ok(json!({ "entries": entries }))
    }

    async fn read_prompt(&self, args: Value) -> Result<Value> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing name"))?;
        let path = self.validate_path(name, &self.prompts_dir)?;

        let content = fs::read_to_string(path).await?;
        Ok(json!({ "content": content }))
    }

    fn validate_path(&self, relative: &str, base: &Path) -> Result<PathBuf> {
        // Simple security check: prevent traversal out of base
        if relative.contains("..") || relative.starts_with("/") {
            return Err(anyhow!("Invalid path: {}", relative));
        }
        let full_path = base.join(relative);
        if !full_path.starts_with(base) {
            return Err(anyhow!("Path traversal detected: {:?}", full_path));
        }
        Ok(full_path)
    }

    async fn search_file_content(&self, args: Value) -> Result<Value> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing pattern"))?;
        let path_str = args["path"].as_str().unwrap_or(".");
        let context_lines = args["context_lines"].as_u64().unwrap_or(0);

        let path = self.validate_path(path_str, &self.worktree_path)?;

        let mut cmd = Command::new("grep");
        cmd.current_dir(&self.worktree_path)
            .arg("-rnI") // Recursive, line numbers, skip binary
            .arg(format!("-C{}", context_lines))
            .arg(pattern)
            .arg(path);

        let output = cmd.output().await?;

        // grep returns exit code 1 if no matches found, which is not an error for us
        if !output.status.success() && output.status.code() != Some(1) {
            return Err(anyhow!(
                "grep failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let content = String::from_utf8_lossy(&output.stdout).to_string();
        if content.is_empty() {
            return Ok(json!({ "matches": [], "message": "No matches found." }));
        }

        Ok(json!({ "content": self.truncate_output(content) }))
    }

    async fn find_files(&self, args: Value) -> Result<Value> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing pattern"))?;
        let path_str = args["path"].as_str().unwrap_or(".");

        let path = self.validate_path(path_str, &self.worktree_path)?;

        // Using 'find' command
        let output = Command::new("find")
            .current_dir(&self.worktree_path)
            .arg(path)
            .arg("-name")
            .arg(pattern)
            .arg("-not")
            .arg("-path")
            .arg("*/.*") // Ignore hidden files/dirs like .git
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!(
                "find failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let content = String::from_utf8_lossy(&output.stdout).to_string();
        let files: Vec<&str> = content.lines().collect();

        // Limit results
        if files.len() > 1000 {
            let truncated = files[..1000].join("\n");
            return Ok(json!({
                 "files": truncated,
                 "total_found": files.len(),
                 "message": "Output truncated to 1000 files."
            }));
        }

        Ok(json!({ "files": content }))
    }
}
