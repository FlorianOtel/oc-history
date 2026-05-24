use std::time::Duration;
use crate::error::AppError;
use crate::opencode::models::{Session, MessageEnvelope, DeleteResult};

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
}
