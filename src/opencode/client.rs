use std::time::Duration;
use crate::error::AppError;
use crate::opencode::models::{Session, MessageEnvelope, DeleteResult, Project, OcSessionView, MessageView, ViewPart};

pub struct Client {
    base_url: String,
    inner: ureq::Agent,
}

impl Client {
    pub fn new(base_url: &str) -> Self {
        let inner = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(15))
            .build();
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            inner,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Probe the endpoint. Try GET /health first; on 404, fall back to GET /session.
    /// Returns Ok(()) if either responds 2xx; otherwise Err(EndpointUnreachable).
    pub fn probe_health(&self) -> Result<(), AppError> {
        let health_url = format!("{}/health", self.base_url);
        match self.inner.get(&health_url).call() {
            Ok(resp) if (200..300).contains(&resp.status()) => return Ok(()),
            Ok(_) => { /* fall through to /session probe */ }
            Err(ureq::Error::Status(404, _)) => { /* fall through */ }
            Err(_) => return Err(AppError::EndpointUnreachable(self.base_url.clone())),
        }
        let session_url = format!("{}/session", self.base_url);
        match self.inner.get(&session_url).call() {
            Ok(resp) if (200..300).contains(&resp.status()) => Ok(()),
            _ => Err(AppError::EndpointUnreachable(self.base_url.clone())),
        }
    }

    pub fn list_sessions(&self) -> Result<Vec<Session>, AppError> {
        let url = format!("{}/session", self.base_url);
        let resp = self.inner
            .get(&url)
            .call()
            .map_err(|e| AppError::Other(format!("list_sessions: {e}")))?;
        let sessions: Vec<Session> = serde_json::from_reader(resp.into_reader())
            .map_err(|e| AppError::Other(format!("list_sessions parse: {e}")))?;
        Ok(sessions)
    }

    /// GET /session/{id}/message — opencode returns a bare JSON array of envelopes.
    /// If a future version wraps in `{ data: [...] }`, this will need adjustment.
    pub fn list_messages(&self, session_id: &str) -> Result<Vec<MessageEnvelope>, AppError> {
        let url = format!("{}/session/{}/message", self.base_url, session_id);
        let resp = self.inner
            .get(&url)
            .call()
            .map_err(|e| AppError::Other(format!("list_messages({session_id}): {e}")))?;
        let envelopes: Vec<MessageEnvelope> = serde_json::from_reader(resp.into_reader())
            .map_err(|e| AppError::Other(format!("list_messages parse: {e}")))?;
        Ok(envelopes)
    }

    /// GET /project — returns all projects known to the opencode server.
    /// Used to build a projectID → worktree lookup for populating conv.project.
    pub fn list_projects(&self) -> Result<Vec<Project>, AppError> {
        let url = format!("{}/project", self.base_url);
        let resp = self.inner
            .get(&url)
            .call()
            .map_err(|e| AppError::Other(format!("list_projects: {e}")))?;
        let projects: Vec<Project> = serde_json::from_reader(resp.into_reader())
            .map_err(|e| AppError::Other(format!("list_projects parse: {e}")))?;
        Ok(projects)
    }

    pub fn delete_session(&self, session_id: &str) -> Result<DeleteResult, AppError> {
        let url = format!("{}/session/{}", self.base_url, session_id);
        match self.inner.delete(&url).call() {
            Ok(resp) if (200..300).contains(&resp.status()) => Ok(DeleteResult::Deleted),
            Err(ureq::Error::Status(404, _)) => Ok(DeleteResult::NotFound),
            Err(ureq::Error::Status(code, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                Ok(DeleteResult::Refused(format!("HTTP {code}: {body}")))
            }
            Err(e) => Err(AppError::Other(format!("delete_session: {e}"))),
            Ok(resp) => Ok(DeleteResult::Refused(format!("unexpected status {}", resp.status()))),
        }
    }

    pub fn fetch_session_content(&self, session_id: &str) -> Result<OcSessionView, AppError> {
        let envelopes = self.list_messages(session_id)?;

        let messages = envelopes.into_iter().map(|env| {
            let created = env.info.time.map(|t| t.created).unwrap_or(0);
            // Prefer the flat fields (assistant); fall back to nested object (user).
            let model_label: Option<String> = env.info.model_id.clone()
                .or_else(|| env.info.model.as_ref().map(|m| m.model_id.clone()))
                .filter(|s| !s.is_empty());
            // Extract and transform parts by inspecting the "type" field directly on the raw JSON
            // value. This avoids serde's limitation with #[serde(other)] on unit variants
            // in internally-tagged enums when the unknown variant has extra fields.
            let parts: Vec<ViewPart> = env.parts.iter()
                .filter_map(|part| {
                    let obj = part.as_object()?;
                    match obj.get("type")?.as_str()? {
                        "text" => {
                            let text = obj.get("text")?.as_str()?.to_string();
                            Some(ViewPart::Text(text))
                        }
                        "reasoning" => {
                            let text = obj.get("text")?.as_str()?.to_string();
                            Some(ViewPart::Reasoning(text))
                        }
                        "tool" => {
                            let name = obj.get("tool")?.as_str()?.to_string();
                            let call_id = obj.get("callID").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let state = obj.get("state")?;
                            let status = state.get("status")?.as_str()?.to_string();
                            let input = state.get("input").cloned().unwrap_or(serde_json::Value::Null);
                            let output = if status == "completed" {
                                state.get("output").cloned()
                            } else {
                                None
                            };
                            Some(ViewPart::ToolCall { name, call_id, input, output, status })
                        }
                        "step-finish" => {
                            let time = obj.get("time");
                            let cost = time.and_then(|t| t.get("cost")).and_then(|v| v.as_f64());
                            let tokens = time.and_then(|t| t.get("tokens"));
                            let input_tokens = tokens.and_then(|t| t.get("input")).and_then(|v| v.as_u64()).unwrap_or(0);
                            let output_tokens = tokens.and_then(|t| t.get("output")).and_then(|v| v.as_u64()).unwrap_or(0);
                            Some(ViewPart::StepFinish { cost, input_tokens, output_tokens })
                        }
                        _ => None, // step-start + unknowns dropped silently
                    }
                })
                .collect();
            MessageView { role: env.info.role, created, model: model_label, parts }
        }).collect();

        Ok(OcSessionView { session_id: session_id.to_string(), messages })
    }
}
