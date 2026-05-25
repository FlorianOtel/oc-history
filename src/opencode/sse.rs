use std::io::{BufRead, BufReader};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Normalized SSE events from the opencode stream
pub enum SseEvent {
    /// Message content changed (message.part.delta, message.part.updated, message.updated)
    ContentChanged,
    /// Session marked idle
    SessionIdle,
    /// Connection dropped; reconnecting with attempt number
    Reconnecting { attempt: u32 },
    /// Gave up after max retries
    Failed(String),
}

/// Spawn a background thread that subscribes to GET /sse/global/event,
/// filters events for `session_id`, and sends normalised events to `tx`.
/// The thread exits cleanly when `tx.send` fails (receiver dropped).
pub fn spawn_sse_subscriber(
    base_url: String,
    session_id: String,
    tx: mpsc::Sender<SseEvent>,
) {
    thread::Builder::new()
        .name("sse-subscriber".into())
        .spawn(move || {
            let mut attempt = 0u32;
            let max_attempts = 10;

            loop {
                // Try to connect
                match connect_and_subscribe(&base_url, &session_id, &tx) {
                    Ok(_) => {
                        // Connection closed normally; reset backoff
                        attempt = 0;
                    }
                    Err(e) => {
                        attempt += 1;
                        let backoff_secs = {
                            let base = 1u64 << attempt.min(5); // 2^attempt, capped at 32
                            base.min(30) // max 30 seconds
                        };

                        if attempt >= max_attempts {
                            let msg = format!("SSE failed after {max_attempts} attempts: {e}");
                            let _ = tx.send(SseEvent::Failed(msg));
                            break;
                        }

                        // Send reconnecting signal and sleep
                        let _ = tx.send(SseEvent::Reconnecting { attempt });
                        thread::sleep(Duration::from_secs(backoff_secs));
                    }
                }
            }
        })
        .expect("failed to spawn sse-subscriber thread");
}

/// Connect to the SSE endpoint and subscribe to events.
/// Returns Ok(()) if the stream closed normally, Err(e) on connection error.
fn connect_and_subscribe(
    base_url: &str,
    session_id: &str,
    tx: &mpsc::Sender<SseEvent>,
) -> Result<(), String> {
    let url = format!("{}/global/event", base_url);

    // Create a no-timeout agent (separate from Client's 15s timeout)
    let agent = ureq::AgentBuilder::new().build();

    let resp = agent
        .get(&url)
        .call()
        .map_err(|e| format!("sse connect failed: {e}"))?;

    if !((200..300).contains(&resp.status())) {
        return Err(format!("sse status {}", resp.status()));
    }

    let reader = BufReader::new(resp.into_reader());
    let mut lines = reader.lines();

    // Process lines until EOF or send error
    loop {
        // Accumulate lines until blank line (SSE event boundary)
        let mut event_lines = Vec::new();

        // Read until we hit a blank line or EOF
        loop {
            match lines.next() {
                Some(Ok(line)) => {
                    if line.is_empty() {
                        break;
                    }
                    event_lines.push(line);
                }
                Some(Err(e)) => {
                    return Err(format!("read error: {e}"));
                }
                None => {
                    // EOF reached
                    return Ok(());
                }
            }
        }

        // Process accumulated lines into a single event
        if let Err(_) = process_event_lines(&event_lines, session_id, tx) {
            // Send failed; receiver dropped
            return Ok(());
        }
    }
}

/// Process lines from a single SSE event.
/// Returns Err if tx.send failed (receiver dropped).
///
/// opencode wraps all event payloads inside a `properties` sub-object; the
/// sessionID field is therefore NOT at the top level of the event. Each event
/// type has a different path to the sessionID:
///
///   message.part.delta    → event.properties.sessionID
///   message.part.updated  → event.properties.part.sessionID
///   message.updated       → event.properties.info.sessionID
///   session.idle          → event.properties.sessionID
fn process_event_lines(
    lines: &[String],
    session_id: &str,
    tx: &mpsc::Sender<SseEvent>,
) -> Result<(), mpsc::SendError<SseEvent>> {
    for line in lines {
        if let Some(data_str) = line.strip_prefix("data: ") {
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(data_str) {
                if let Some(event_type) = event.get("type").and_then(|v| v.as_str()) {
                    match event_type {
                        "message.part.delta" => {
                            let sid = event["properties"]["sessionID"].as_str().unwrap_or("");
                            if sid == session_id {
                                tx.send(SseEvent::ContentChanged)?;
                            }
                        }
                        "message.part.updated" => {
                            let sid = event["properties"]["part"]["sessionID"].as_str().unwrap_or("");
                            if sid == session_id {
                                tx.send(SseEvent::ContentChanged)?;
                            }
                        }
                        "message.updated" => {
                            let sid = event["properties"]["info"]["sessionID"].as_str().unwrap_or("");
                            if sid == session_id {
                                tx.send(SseEvent::ContentChanged)?;
                            }
                        }
                        "session.idle" => {
                            let sid = event["properties"]["sessionID"].as_str().unwrap_or("");
                            if sid == session_id {
                                tx.send(SseEvent::SessionIdle)?;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}
