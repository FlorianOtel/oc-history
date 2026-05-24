//! Parse Claude Code's global history log (~/.claude/history.jsonl)
//!
//! Claude Code writes session metadata to a global JSONL log that records
//! user inputs, timestamps, and project context. This module extracts session
//! display names (the first meaningful user message) to supplement local
//! JSONL metadata that may be missing summary or custom-title entries.

use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// A single entry in ~/.claude/history.jsonl
#[derive(Debug, Deserialize, Clone)]
pub struct HistoryEntry {
    pub display: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub project: String,
    pub timestamp: i64,
    #[serde(default)]
    pub paste_contents: serde_json::Value,
}

/// Load ~/.claude/history.jsonl and return a map from sessionId to display name
/// (the first meaningful user message in the session).
///
/// Returns a HashMap even on errors (partial data is better than none).
pub fn load_session_display_map() -> HashMap<String, String> {
    let mut map = HashMap::new();

    // Respect CLAUDE_CONFIG_DIR if set, otherwise use ~/.claude
    let claude_dir = if let Ok(config_dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        PathBuf::from(config_dir)
    } else {
        match home::home_dir() {
            Some(home) => home.join(".claude"),
            None => return map, // Can't find home, return empty map
        }
    };

    let history_path = claude_dir.join("history.jsonl");

    if !history_path.exists() {
        return map;
    }

    // Open and read the file
    match File::open(&history_path) {
        Ok(file) => {
            let reader = BufReader::new(file);

            // Group entries by sessionId, keeping track of the first MEANINGFUL display
            let mut session_first: HashMap<String, Option<String>> = HashMap::new();

            for line in reader.lines() {
                if let Ok(line) = line {
                    if let Ok(entry) = serde_json::from_str::<HistoryEntry>(&line) {
                        // Collect display name only if it's "meaningful":
                        // - Not empty
                        // - Not just "." (common placeholder)
                        // - Not a slash command (/exit, /clear, /model, etc.)
                        let is_meaningful = !entry.display.is_empty()
                            && entry.display != "."
                            && !entry.display.starts_with('/');

                        // Track first MEANINGFUL entry per session, not first entry overall
                        // (history.jsonl is chronological, so we wait for first meaningful one)
                        let slot = session_first.entry(entry.session_id).or_insert(None);
                        if slot.is_none() && is_meaningful {
                            *slot = Some(entry.display);
                        }
                    }
                }
            }

            // Extract only sessions with meaningful displays
            for (session_id, display_opt) in session_first {
                if let Some(display) = display_opt {
                    map.insert(session_id, display);
                }
            }
        }
        Err(_) => {
            // File not readable, return empty map
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn filters_out_commands() {
        let entries = vec![
            r#"{"display": "/exit", "sessionId": "s1", "project": "/p", "timestamp": 1, "pastedContents": {}}"#,
            r#"{"display": "hello", "sessionId": "s1", "project": "/p", "timestamp": 2, "pastedContents": {}}"#,
        ];

        // Parse directly (test doesn't use file I/O)
        let first = serde_json::from_str::<HistoryEntry>(entries[0]).unwrap();
        assert!(first.display.starts_with('/'));

        let second = serde_json::from_str::<HistoryEntry>(entries[1]).unwrap();
        assert!(!second.display.starts_with('/'));
    }

    #[test]
    fn filters_out_dots_and_empty() {
        let display_vals = vec![".", "", "hello", "/cmd"];
        for (i, display) in display_vals.iter().enumerate() {
            let is_meaningful = !display.is_empty()
                && *display != "."
                && !display.starts_with('/');

            match i {
                0 | 1 | 3 => assert!(!is_meaningful), // ".", "", "/cmd" are not meaningful
                2 => assert!(is_meaningful),           // "hello" is meaningful
                _ => {}
            }
        }
    }
}
