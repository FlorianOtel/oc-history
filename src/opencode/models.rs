use serde::Deserialize;

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
pub struct MessageEnvelope {
    pub info: MessageInfo,
    #[serde(default)]
    pub parts: Vec<serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MessageInfo {
    pub role: String,
    #[serde(default)]
    pub cost: Option<f64>,
    #[serde(default)]
    pub tokens: Option<TokenCounts>,
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
