use std::sync::{Arc, mpsc};
use std::thread;
use chrono::{Local, TimeZone};
use rayon::prelude::*;

use crate::history::{Conversation, LoaderMessage, ParseError};
use crate::opencode::{Client, MessageEnvelope, Session};

/// Spawn a background thread that fetches sessions + message stats from the
/// opencode HTTP endpoint and streams them as LoaderMessage batches.
///
/// On success, sends one or more `LoaderMessage::Batch(...)` followed by
/// `LoaderMessage::Done`. On failure to list sessions, sends `LoaderMessage::Fatal(...)`.
pub fn load_sessions_streaming(
    client: Arc<Client>,
) -> mpsc::Receiver<LoaderMessage> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut sessions = match client.list_sessions() {
            Ok(s) => s,
            Err(e) => {
                let _ = tx.send(LoaderMessage::Fatal(format!("list_sessions: {e}")));
                return;
            }
        };

        // Sort by time.updated descending (newest first).
        sessions.sort_by(|a, b| b.time.updated.cmp(&a.time.updated));

        // Fan out per-session message fetches via rayon.
        let conversations: Vec<Conversation> = sessions
            .par_iter()
            .enumerate()
            .map(|(idx, session)| build_conversation(&client, idx, session))
            .collect();

        // Batch sends (20 per batch) to give the TUI smooth incremental rendering.
        const BATCH_SIZE: usize = 20;
        for chunk in conversations.chunks(BATCH_SIZE) {
            if tx.send(LoaderMessage::Batch(chunk.to_vec())).is_err() {
                return; // receiver dropped
            }
        }
        let _ = tx.send(LoaderMessage::Done);
    });
    rx
}

fn build_conversation(client: &Client, idx: usize, session: &Session) -> Conversation {
    // Best-effort message fetch — empty list on failure, surfaced via parse_errors.
    let (turn_count, cost_usd, tokens_in, tokens_out, tokens_reasoning, message_count, parse_errors) =
        match client.list_messages(&session.id) {
            Ok(envelopes) => aggregate(&envelopes),
            Err(e) => (0, 0.0, 0, 0, 0, 0, vec![ParseError {
                line: 0,
                message: format!("list_messages({}): {e}", session.id),
            }]),
        };

    let title = if session.title.trim().is_empty() {
        // Fallback: "ses_<7-char-id>" — show enough to disambiguate.
        let stripped = session.id.strip_prefix("ses_").unwrap_or(&session.id);
        let short: String = stripped.chars().take(7).collect();
        format!("ses_{short}")
    } else {
        session.title.clone()
    };

    let timestamp = Local
        .timestamp_millis_opt(session.time.updated)
        .single()
        .unwrap_or_else(Local::now);

    let total_tokens = tokens_in + tokens_out + tokens_reasoning;

    Conversation {
        id: session.id.clone(),
        index: idx,
        timestamp,
        title,
        project: session.directory.clone(),
        turn_count,
        cost_usd,
        tokens_in,
        tokens_out,
        tokens_reasoning,

        // Stub fields
        path: std::path::PathBuf::from(&session.id),
        project_name: None,
        summary: None,
        custom_title: None,
        model: None,
        total_tokens,
        message_count,
        duration_minutes: None,
        preview: String::new(),
        preview_first: String::new(),
        preview_last: String::new(),
        full_text: String::new(),
        search_text_lower: String::new(),
        parse_errors,
        project_path: None,
        cwd: None,
    }
}

fn aggregate(envelopes: &[MessageEnvelope]) -> (usize, f64, u64, u64, u64, usize, Vec<ParseError>) {
    let mut turn_count = 0usize;
    let mut cost_usd = 0.0f64;
    let mut tokens_in = 0u64;
    let mut tokens_out = 0u64;
    let mut tokens_reasoning = 0u64;
    let message_count = envelopes.len();
    for env in envelopes {
        if env.info.role == "assistant" {
            turn_count += 1;
        }
        if let Some(c) = env.info.cost {
            cost_usd += c;
        }
        if let Some(t) = &env.info.tokens {
            tokens_in += t.input;
            tokens_out += t.output;
            tokens_reasoning += t.reasoning;
        }
    }
    (turn_count, cost_usd, tokens_in, tokens_out, tokens_reasoning, message_count, Vec::new())
}
