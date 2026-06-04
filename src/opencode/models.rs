use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug, Clone)]
pub struct Session {
    pub id: String,
    #[serde(rename = "projectID")]
    pub project_id: String,
    pub directory: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub version: String,
    pub time: SessionTime,
    #[serde(rename = "parentID", default)]
    pub parent_id: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SessionTime {
    pub created: i64,
    pub updated: i64,
    #[serde(default)]
    pub compacting: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct V2SessionList {
    pub items: Vec<V2SessionItem>,
    #[serde(default)]
    pub cursor: V2Cursor,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct V2Cursor {
    #[serde(default)]
    pub previous: Option<String>,
    #[serde(default)]
    pub next: Option<String>,
}

// V2 /api/session item (older opencode) — uses a relative `path` field.
#[derive(Deserialize, Debug, Clone)]
pub struct V2SessionItem {
    pub id: String,
    #[serde(rename = "projectID")]
    pub project_id: String,
    // path is the session cwd relative to filesystem root, without leading slash.
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub title: String,
    pub time: SessionTime,
}

// V2 /api/session list (newer opencode) — top-level key is `data`, not `items`.
#[derive(Deserialize, Debug, Clone)]
pub struct V2NewSessionList {
    pub data: Vec<V2NewSessionItem>,
    #[serde(default)]
    pub cursor: V2Cursor,
}

// V2 /api/session item (newer opencode) — uses a `location` object with an absolute `directory`.
#[derive(Deserialize, Debug, Clone)]
pub struct V2NewSessionItem {
    pub id: String,
    #[serde(rename = "projectID")]
    pub project_id: String,
    #[serde(default)]
    pub title: String,
    pub location: V2Location,
    pub time: SessionTime,
}

#[derive(Deserialize, Debug, Clone)]
pub struct V2Location {
    pub directory: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MessageTime {
    pub created: i64,
    #[serde(default)]
    pub completed: Option<i64>,
}

// Parts are kept as raw JSON values to avoid serde edge cases with
// internally-tagged enums + unknown variants. Text extraction is done
// in client.rs by inspecting the "type" field directly.
#[derive(Deserialize, Debug, Clone)]
pub struct MessageEnvelope {
    pub info: MessageInfo,
    #[serde(default)]
    pub parts: Vec<Value>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct MessageModel {
    #[serde(rename = "providerID", default)]
    pub provider_id: String,
    #[serde(rename = "modelID", default)]
    pub model_id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MessageInfo {
    pub role: String,
    #[serde(default)]
    pub cost: Option<f64>,
    #[serde(default)]
    pub tokens: Option<TokenCounts>,
    #[serde(default)]
    pub time: Option<MessageTime>,
    // User messages carry a nested "model" object; assistant messages have
    // "modelID" / "providerID" at the info level. Both paths are captured here.
    #[serde(default)]
    pub model: Option<MessageModel>,
    #[serde(rename = "modelID", default)]
    pub model_id: Option<String>,
    #[serde(rename = "providerID", default)]
    pub provider_id: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct TokenCounts {
    #[serde(default)]
    pub input: u64,
    #[serde(default)]
    pub output: u64,
    #[serde(default)]
    pub reasoning: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Project {
    pub id: String,
    pub worktree: String,
    #[serde(rename = "vcsDir", default)]
    pub vcs_dir: Option<String>,
    #[serde(default)]
    pub vcs: Option<String>,
}

#[derive(Debug)]
pub enum DeleteResult {
    Deleted,
    NotFound,
    Refused(String),
}

#[derive(Debug, Clone)]
pub struct OcSessionView {
    pub session_id: String,
    pub messages: Vec<MessageView>,
}

/// View-layer representation of a message part, constructed by JSON inspection.
/// Not deserialized directly from serde.
#[derive(Debug, Clone)]
pub enum ViewPart {
    Text(String),
    Reasoning(String),
    ToolCall {
        name: String,
        call_id: String,
        input: serde_json::Value,
        output: Option<serde_json::Value>,
        status: String,
    },
    StepFinish {
        cost: Option<f64>,
        input_tokens: u64,
        output_tokens: u64,
    },
}

#[derive(Debug, Clone)]
pub struct MessageView {
    pub role: String,
    pub created: i64,
    pub model: Option<String>,
    pub parts: Vec<ViewPart>,
}
