#![allow(dead_code, unused, non_camel_case_types)]

//! Stub module for v1 cleanup. Real implementations were removed in v0.
//! These types exist only to satisfy imports in dead code files.

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum LogEntry {
    #[serde(rename = "summary")]
    Summary { summary: String },
    User {
        message: UserMessage,
        #[serde(default)]
        timestamp: Option<String>,
        #[allow(dead_code)]
        uuid: Option<String>,
        cwd: Option<String>,
        #[serde(default, rename = "parent_tool_use_id")]
        parent_tool_use_id: Option<String>,
    },
    Assistant {
        message: AssistantMessage,
        #[serde(default)]
        timestamp: Option<String>,
        #[allow(dead_code)]
        uuid: Option<String>,
        #[serde(default, rename = "parent_tool_use_id")]
        parent_tool_use_id: Option<String>,
    },
    #[serde(rename = "system")]
    System { message: String },
    #[serde(rename = "custom-title")]
    CustomTitle { title: String },
    #[serde(rename = "file-history-snapshot")]
    FileHistorySnapshot {
        #[serde(rename = "messageId")]
        message_id: String,
        snapshot: serde_json::Value,
        #[serde(rename = "isSnapshotUpdate")]
        is_snapshot_update: bool,
    },
    #[serde(rename = "agent-progress")]
    Progress { data: serde_json::Value },
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum UserContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

impl Default for UserContent {
    fn default() -> Self {
        UserContent::Text(String::new())
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct UserMessage {
    #[serde(default)]
    pub content: UserContent,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum AssistantContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

impl Default for AssistantContent {
    fn default() -> Self {
        AssistantContent::Text(String::new())
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AssistantMessage {
    #[serde(default)]
    pub content: AssistantContent,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum ContentBlock {
    Text { text: String },
    Thinking { thinking: String },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool-result")]
    ToolResult {
        tool_use_id: String,
        #[serde(default)]
        content: Option<serde_json::Value>,
    },
}

#[derive(Debug, Clone)]
pub struct AgentProgressData {
    pub message: String,
    pub agent_id: String,
}

#[derive(Debug, Clone)]
pub enum AgentContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

pub fn parse_agent_progress(_data: &serde_json::Value) -> Option<AgentProgressData> {
    None
}

pub fn short_parent_id(id: &str) -> String {
    id.chars().take(8).collect()
}
