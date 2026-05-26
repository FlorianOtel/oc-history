use chrono::{Local, TimeZone};
use crate::opencode::models::{OcSessionView, ViewPart};
use crate::tui::{RenderOptions, RenderedConversation, MessageRange, ToolDisplayMode};
use crate::tui::app::{RenderedLine, LineStyle};
use crate::tool_format;
use crate::error::Result;

/// Render an opencode session as plain text with role/timestamp headers.
pub fn render_oc_session(
    content: Option<&OcSessionView>,
    options: &RenderOptions,
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

                let role_label = if msg.role == "assistant" {
                    match &msg.model {
                        Some(m) => format!("[assistant - {}]", m),
                        None    => "[assistant]".to_string(),
                    }
                } else {
                    format!("[{}]", msg.role)
                };
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

                // Render parts
                for part in &msg.parts {
                    render_part(part, options, &mut lines);
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

/// Wrap `text` at `width` characters, preserving existing line breaks.
/// Each resulting line is emitted as a `RenderedLine` with `style` applied.
/// If `width` is 0, lines pass through without wrapping.
fn wrap_into_lines(text: &str, width: usize, style: LineStyle) -> Vec<RenderedLine> {
    let mut result = Vec::new();
    for src_line in text.lines() {
        if src_line.is_empty() {
            result.push(RenderedLine { spans: vec![("".to_string(), style.clone())] });
        } else if width > 0 && src_line.chars().count() > width {
            for wrapped in textwrap::wrap(src_line, width) {
                result.push(RenderedLine {
                    spans: vec![(wrapped.into_owned(), style.clone())],
                });
            }
        } else {
            result.push(RenderedLine {
                spans: vec![(src_line.to_string(), style.clone())],
            });
        }
    }
    // Preserve trailing newline as a blank line
    if text.ends_with('\n') {
        result.push(RenderedLine { spans: vec![("".to_string(), style.clone())] });
    }
    result
}

/// Render a single ViewPart, respecting display options
fn render_part(part: &ViewPart, options: &RenderOptions, lines: &mut Vec<RenderedLine>) {
    let w = options.content_width;
    let dim = LineStyle { dimmed: true, ..Default::default() };

    match part {
        ViewPart::Text(s) => {
            lines.extend(wrap_into_lines(s, w, LineStyle::default()));
        }
        ViewPart::Reasoning(s) => {
            if options.show_thinking {
                lines.push(RenderedLine {
                    spans: vec![("[thinking]".to_string(), dim.clone())],
                });
                lines.extend(wrap_into_lines(s, w, dim));
            }
        }
        ViewPart::ToolCall { name, input, output, status, .. } => {
            match options.tool_display {
                ToolDisplayMode::Hidden => {}
                ToolDisplayMode::Truncated => {
                    let formatted = tool_format::format_tool_call(name, input, w);
                    lines.extend(wrap_into_lines(&format!("▶ {}", formatted.header), w, dim.clone()));

                    if output.is_some() && status == "completed" {
                        let output_str = tool_format::format_tool_output(output.as_ref().unwrap(), true);
                        lines.extend(wrap_into_lines(&output_str, w, dim));
                    }
                }
                ToolDisplayMode::Full => {
                    let formatted = tool_format::format_tool_call(name, input, w);
                    lines.extend(wrap_into_lines(&format!("▶ {}", formatted.header), w, dim.clone()));

                    if let Some(body) = &formatted.body {
                        lines.extend(wrap_into_lines(body, w, dim.clone()));
                    }

                    if output.is_some() && status == "completed" {
                        let output_str = tool_format::format_tool_output(output.as_ref().unwrap(), false);
                        lines.extend(wrap_into_lines(&output_str, w, dim));
                    }
                }
            }
        }
        ViewPart::StepFinish { cost, input_tokens, output_tokens } => {
            if options.show_timing {
                let mut timing_line = format!("  ↳ {}↑ {}↓ tokens", input_tokens, output_tokens);
                if let Some(c) = cost {
                    timing_line.push_str(&format!(", ${:.4}", c));
                }
                lines.push(RenderedLine {
                    spans: vec![(timing_line, dim)],
                });
            }
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
