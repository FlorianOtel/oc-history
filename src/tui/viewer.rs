use chrono::{Local, TimeZone};
use crate::opencode::models::OcSessionView;
use crate::tui::{RenderOptions, RenderedConversation, MessageRange};
use crate::tui::app::{RenderedLine, LineStyle};
use crate::error::Result;

/// Render an opencode session as plain text with role/timestamp headers.
pub fn render_oc_session(
    content: Option<&OcSessionView>,
    _options: &RenderOptions,
) -> Result<RenderedConversation> {
    match content {
        None => Ok(placeholder("Loading…")),
        Some(session) => {
            if session.messages.is_empty() {
                return Ok(placeholder("No messages in this session."));
            }

            let mut lines = Vec::new();
            let mut message_ranges = Vec::new();

            for (idx, msg) in session.messages.iter().enumerate() {
                let start_line = lines.len();

                // Header: bold role label + dim timestamp
                let dt = Local.timestamp_millis_opt(msg.created).single()
                    .unwrap_or_else(|| Local::now());
                let timestamp = dt.format("%Y-%m-%d %H:%M").to_string();

                let role_label = format!("[{}]", msg.role);
                let role_span = (role_label, LineStyle {
                    bold: true,
                    ..Default::default()
                });
                let time_span = (format!(" {}", timestamp), LineStyle {
                    dimmed: true,
                    ..Default::default()
                });

                lines.push(RenderedLine {
                    spans: vec![role_span, time_span],
                });

                // Text content lines
                for text in &msg.text_parts {
                    for line in text.lines() {
                        lines.push(RenderedLine {
                            spans: vec![(line.to_string(), LineStyle::default())],
                        });
                    }
                }

                // Blank separator line
                lines.push(RenderedLine {
                    spans: vec![("".to_string(), LineStyle::default())],
                });

                let end_line = lines.len().saturating_sub(1);
                message_ranges.push(MessageRange {
                    start_line,
                    end_line,
                    entry_index: idx,
                });
            }

            Ok(RenderedConversation {
                lines,
                messages: message_ranges,
            })
        }
    }
}

/// Helper to create a placeholder single-line response.
fn placeholder(msg: &str) -> RenderedConversation {
    RenderedConversation {
        lines: vec![RenderedLine {
            spans: vec![(msg.to_string(), LineStyle {
                dimmed: true,
                ..Default::default()
            })],
        }],
        messages: vec![],
    }
}
