# Git Log Tool Design

## 1. Overview
This document outlines the design for a customizable `git log` tool within Sashiko. The goal is to provide a flexible interface for LLM agents or internal components to retrieve git history with specific content filtering.

## 2. Requirements
-   **Path Filtering**: Ability to view history for specific files or directories.
-   **Content Selection**: Customizable output fields (Hash, Author, Date, Subject, Body, Stat) to reduce context window usage for LLMs.
-   **Revision Control**: Support for limiting commits (count) and specifying revision ranges (e.g., `HEAD~5..HEAD`).
-   **Output Format**: Clear, parseable text output suitable for LLM consumption.

## 3. API Design

### 3.1. Struct: `GitLogParams`
A Rust struct to encapsulate all options.

```rust
#[derive(Debug, Clone)]
pub struct GitLogParams {
    pub repo_path: PathBuf,
    pub limit: Option<usize>,
    pub rev_range: Option<String>,
    pub paths: Vec<String>,
    
    // Output toggle flags
    pub show_hash: bool,
    pub show_author: bool,
    pub show_date: bool,
    pub show_subject: bool,
    pub show_body: bool,
    pub show_stat: bool,
}

impl Default for GitLogParams {
    fn default() -> Self {
        Self {
            repo_path: PathBuf::new(),
            limit: None,
            rev_range: None,
            paths: Vec::new(),
            show_hash: true,
            show_author: false,
            show_date: false,
            show_subject: true,
            show_body: false,
            show_stat: false,
        }
    }
}
```

### 3.2. Function: `get_git_log`
```rust
pub async fn get_git_log(params: GitLogParams) -> Result<String>
```

## 4. Implementation Details
-   Uses `tokio::process::Command` to execute `git log`.
-   Constructs a `--pretty=format:` string based on enabled flags.
    -   `%h`: Abbreviated hash
    -   `%an`: Author name
    -   `%ad`: Author date (format: short)
    -   `%s`: Subject
    -   `%b`: Body
-   Appends `--stat` if requested.
-   Appends `--max-count` if limit is set.
-   Appends `--date=short` if date is shown.
-   Handles empty results gracefully.

## 5. Usage Example
```rust
let params = GitLogParams {
    repo_path: Path::new("/path/to/repo").to_path_buf(),
    limit: Some(10),
    show_subject: true,
    show_author: true,
    ..Default::default()
};
let log = get_git_log(params).await?;
```
