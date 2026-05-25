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

/// Render a single ViewPart, respecting display options
fn render_part(part: &ViewPart, options: &RenderOptions, lines: &mut Vec<RenderedLine>) {
    match part {
        ViewPart::Text(s) => {
            // Regular text lines
            for line in s.lines() {
                lines.push(RenderedLine {
                    spans: vec![(line.to_string(), LineStyle::default())],
                });
            }
        }
        ViewPart::Reasoning(s) => {
            // Only show if options.show_thinking is true
            if options.show_thinking {
                lines.push(RenderedLine {
                    spans: vec![("[thinking]".to_string(), LineStyle {
                        dimmed: true,
                        ..Default::default()
                    })],
                });
                for line in s.lines() {
                    lines.push(RenderedLine {
                        spans: vec![(line.to_string(), LineStyle {
                            dimmed: true,
                            ..Default::default()
                        })],
                    });
                }
            }
        }
        ViewPart::ToolCall { name, input, output, status, .. } => {
            match options.tool_display {
                ToolDisplayMode::Hidden => {
                    // Skip entirely
                }
                ToolDisplayMode::Truncated => {
                    let formatted = tool_format::format_tool_call(name, input, options.content_width);

                    // Header line with ▶ prefix (dim)
                    lines.push(RenderedLine {
                        spans: vec![(format!("▶ {}", formatted.header), LineStyle {
                            dimmed: true,
                            ..Default::default()
                        })],
                    });

                    // If completed, show one truncated output line
                    if output.is_some() && status == "completed" {
                        let output_str = tool_format::format_tool_output(output.as_ref().unwrap(), true);
                        lines.push(RenderedLine {
                            spans: vec![(output_str, LineStyle {
                                dimmed: true,
                                ..Default::default()
                            })],
                        });
                    }
                }
                ToolDisplayMode::Full => {
                    let formatted = tool_format::format_tool_call(name, input, options.content_width);

                    // Header line with ▶ prefix (dim)
                    lines.push(RenderedLine {
                        spans: vec![(format!("▶ {}", formatted.header), LineStyle {
                            dimmed: true,
                            ..Default::default()
                        })],
                    });

                    // Body lines if present
                    if let Some(body) = &formatted.body {
                        for body_line in body.lines() {
                            lines.push(RenderedLine {
                                spans: vec![(body_line.to_string(), LineStyle {
                                    dimmed: true,
                                    ..Default::default()
                                })],
                            });
                        }
                    }

                    // All output lines if completed
                    if output.is_some() && status == "completed" {
                        let output_str = tool_format::format_tool_output(output.as_ref().unwrap(), false);
                        for output_line in output_str.lines() {
                            lines.push(RenderedLine {
                                spans: vec![(output_line.to_string(), LineStyle {
                                    dimmed: true,
                                    ..Default::default()
                                })],
                            });
                        }
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
                    spans: vec![(timing_line, LineStyle {
                        dimmed: true,
                        ..Default::default()
                    })],
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
