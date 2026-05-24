//! Claude conversation history loading and parsing.
//!
//! This module provides types and utilities for managing conversation history.

pub mod global_log;
pub mod path;

use chrono::{DateTime, Local};
use std::path::PathBuf;

// Re-export public API from path module
pub use path::{convert_path_to_project_dir_name, format_short_name_from_path, is_same_project};

/// Represents a conversation backed by opencode session data
#[derive(Debug, Clone)]
pub struct Conversation {
    // Real opencode-backed data
    pub id: String,                      // opencode session ID (used for delete)
    pub index: usize,
    pub timestamp: DateTime<Local>,      // from session.time.updated
    pub title: String,                   // session.title (or fallback)
    pub project: String,                 // session.directory
    pub turn_count: usize,               // count of role=assistant messages
    pub cost_usd: f64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub tokens_reasoning: u64,

    // Compile-stub fields — populated as None/empty/zero. Existing TUI reads them
    // through code paths that are stubbed in Steps 5-6, so values don't matter
    // beyond satisfying the borrow checker.
    pub path: PathBuf,                   // PathBuf::from(&id) — used for identity in delete dispatch
    pub project_name: Option<String>,
    pub summary: Option<String>,
    pub custom_title: Option<String>,
    pub model: Option<String>,
    pub total_tokens: u64,               // = tokens_in + tokens_out + tokens_reasoning
    pub message_count: usize,
    pub duration_minutes: Option<u64>,
    pub preview: String,
    pub preview_first: String,
    pub preview_last: String,
    pub full_text: String,
    pub search_text_lower: String,
    pub parse_errors: Vec<ParseError>,
    pub project_path: Option<PathBuf>,
    pub cwd: Option<PathBuf>,
}

/// Represents a parsing error
#[derive(Debug, Clone)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

/// Message sent from background loader to TUI
pub enum LoaderMessage {
    /// A batch of loaded conversations
    Batch(Vec<Conversation>),
    /// Loading completed
    Done,
    /// A fatal error occurred
    Fatal(String),
}

/// Find a conversation jsonl file by UUID (v0 stub)
pub fn find_jsonl_by_uuid(_uuid: &str) -> crate::error::Result<PathBuf> {
    Err(crate::error::AppError::Other(
        "UUID lookup not implemented in v0".to_string(),
    ))
}

/// Process a conversation file from disk (v0 stub)
pub fn process_conversation_file(
    _path: PathBuf,
    _modified: Option<std::time::SystemTime>,
    _title_override: Option<String>,
    _custom_title: Option<String>,
) -> crate::error::Result<Conversation> {
    Err(crate::error::AppError::Other(
        "File processing not implemented in v0".to_string(),
    ))
}
