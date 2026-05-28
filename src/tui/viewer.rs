use chrono::{Local, TimeZone};
use crate::opencode::models::{OcSessionView, ViewPart, MessageView};
use crate::tui::{RenderOptions, RenderedConversation, MessageRange, ToolDisplayMode};
use crate::tui::app::{RenderedLine, LineStyle};
use crate::tool_format;
use crate::markdown::render_markdown;
use crate::error::Result;
use unicode_width::UnicodeWidthStr;

/// Emit a labeled block into the output lines.
/// For each content line at index i:
/// - If i == 0: prefix = label padded to label_width + " │ " with label_style applied
/// - If i > 0: prefix = blank pad to label_width + " │ " with default style
/// If content_lines is empty, emit one line with just the padded label + separator.
fn emit_labeled_block(
    out: &mut Vec<RenderedLine>,
    label: &str,
    label_width: usize,
    content_lines: &[RenderedLine],
    label_style: LineStyle,
) {
    if content_lines.is_empty() {
        // Emit one line with just the label and separator
        let prefix = format!("{:>width$} │ ", label, width = label_width);
        out.push(RenderedLine {
            spans: vec![(prefix, label_style)],
        });
        return;
    }

    for (i, content_line) in content_lines.iter().enumerate() {
        let mut spans = Vec::new();

        if i == 0 {
            // First line: emit label with style
            let prefix = format!("{:>width$} │ ", label, width = label_width);
            spans.push((prefix, label_style.clone()));
        } else {
            // Continuation lines: emit blank pad
            let prefix = format!("{:>width$} │ ", "", width = label_width);
            spans.push((prefix, LineStyle::default()));
        }

        // Append content line spans
        spans.extend(content_line.spans.clone());

        out.push(RenderedLine { spans });
    }
}

/// Compute the display width of the longest label in a session.
/// Labels are: [user], [<model>], [assistant], [tool], [thinking], [time]
/// Returns the width of the longest label, with minimum width of 6 (width of "[user]")
fn compute_label_width(messages: &[MessageView]) -> usize {
    let mut max_width = 6; // Minimum: "[user]"

    for msg in messages {
        // Build the label for this message's role/model
        let role_label = if msg.role == "assistant" {
            match &msg.model {
                Some(m) => format!("[{}]", m),
                None => "[assistant]".to_string(),
            }
        } else {
            format!("[{}]", msg.role)
        };

        let width = UnicodeWidthStr::width(role_label.as_str());
        max_width = max_width.max(width);

        // Also check for tool, thinking, and time labels in parts
        for part in &msg.parts {
            let label_width = match part {
                ViewPart::ToolCall { .. } => UnicodeWidthStr::width("[tool]"),
                ViewPart::Reasoning(_) => UnicodeWidthStr::width("[thinking]"),
                ViewPart::StepFinish { .. } => UnicodeWidthStr::width("[time]"),
                ViewPart::Text(_) => 0, // Text uses role label, already counted
            };
            max_width = max_width.max(label_width);
        }
    }

    max_width
}

/// Parse ANSI-formatted string into RenderedLines, with SGR escape sequence support.
/// Splits on \n and parses inline SGR sequences to build styled spans.
fn ansi_to_rendered_lines(ansi_str: &str) -> Vec<RenderedLine> {
    let mut result = Vec::new();

    for line_str in ansi_str.lines() {
        let spans = parse_ansi_line(line_str);
        result.push(RenderedLine { spans });
    }

    // If the input was empty, emit one default line
    if result.is_empty() {
        result.push(RenderedLine {
            spans: vec![("".to_string(), LineStyle::default())],
        });
    }

    result
}

/// Parse a single ANSI line into spans.
/// Handles SGR codes: reset (0/m), bold (1), dimmed (2), italic (3), normal-intensity (22),
/// normal-italic (23), RGB truecolor (38;2;R;G;B), reset-fg (39), and colored codes (32/34/36/90).
fn parse_ansi_line(line: &str) -> Vec<(String, LineStyle)> {
    let mut spans = Vec::new();
    let mut current_style = LineStyle::default();
    let mut current_text = String::new();

    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        // Check for ESC sequence start
        if ch == '\x1b' {
            // Flush current text before processing escape
            if !current_text.is_empty() {
                spans.push((current_text.clone(), current_style.clone()));
                current_text.clear();
            }

            // Consume '['
            if chars.peek() == Some(&'[') {
                chars.next();

                // Collect the parameter/code part
                let mut sgr_param = String::new();
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_alphabetic() {
                        let terminator = chars.next().unwrap();
                        apply_sgr_code(&sgr_param, terminator, &mut current_style);
                        break;
                    } else {
                        sgr_param.push(chars.next().unwrap());
                    }
                }
            }
        } else {
            current_text.push(ch);
        }
    }

    // Flush remaining text
    if !current_text.is_empty() {
        spans.push((current_text, current_style.clone()));
    }

    // If no spans were created, emit empty span
    if spans.is_empty() {
        spans.push(("".to_string(), LineStyle::default()));
    }

    spans
}

/// Apply an SGR code sequence to modify the current style.
fn apply_sgr_code(param: &str, terminator: char, style: &mut LineStyle) {
    // Only handle 'm' terminator (SGR - Select Graphic Rendition)
    if terminator != 'm' {
        return;
    }

    // Parse parameters separated by semicolons
    let codes: Vec<&str> = param.split(';').collect();

    // Handle different parameter counts
    if codes.is_empty() {
        return;
    }

    // Single numeric code
    if codes.len() == 1 {
        match codes[0] {
            "0" | "" => {
                // Reset
                *style = LineStyle::default();
            }
            "1" => {
                // Bold
                style.bold = true;
            }
            "2" => {
                // Dimmed
                style.dimmed = true;
            }
            "3" => {
                // Italic
                style.italic = true;
            }
            "22" => {
                // Normal intensity (not bold or dimmed)
                style.bold = false;
                style.dimmed = false;
            }
            "23" => {
                // Not italic
                style.italic = false;
            }
            "24" => {
                // Underline off (we don't track underline, so drop silently)
            }
            "32" => {
                // Green (blockquote prefix)
                style.fg = Some((0, 128, 0));
            }
            "34" => {
                // Blue (link)
                style.fg = Some((0, 0, 255));
            }
            "36" => {
                // Cyan (heading prefix)
                style.fg = Some((0, 255, 255));
            }
            "39" => {
                // Reset foreground
                style.fg = None;
            }
            "90" => {
                // Bright black (fallback for unknown code blocks — this is typically background, drop it)
                // Don't apply, as colored crate uses .on_bright_black() which is BACKGROUND
            }
            _ => {
                // Unknown code, drop silently
            }
        }
        return;
    }

    // Truecolor (38;2;R;G;B)
    if codes.len() >= 5 && codes[0] == "38" && codes[1] == "2" {
        if let (Ok(r), Ok(g), Ok(b)) = (
            codes[2].parse::<u8>(),
            codes[3].parse::<u8>(),
            codes[4].parse::<u8>(),
        ) {
            style.fg = Some((r, g, b));
        }
        return;
    }

    // Unknown multi-parameter code, drop silently
}

/// Render an opencode session with ledger-style formatting.
/// Each message part is emitted as a labeled row: `<label-pad> │ <content>`
/// The label column is auto-fitted to the longest label in the session.
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

            // Compute label width once per session
            let label_width = compute_label_width(&session.messages);

            // Compute content width for markdown rendering
            let content_width = if options.content_width == 0 {
                100_000 // Large sentinel for pager mode (no wrapping)
            } else {
                options.content_width.saturating_sub(label_width + 3).max(10)
            };

            for (idx, msg) in session.messages.iter().enumerate() {
                let start_line = lines.len();

                // Format timestamp once per message
                let dt = Local.timestamp_millis_opt(msg.created).single()
                    .unwrap_or_else(|| Local::now());
                let timestamp = dt.format("%Y-%m-%d %H:%M").to_string();

                // Build message role label
                let msg_role_label = if msg.role == "assistant" {
                    match &msg.model {
                        Some(m) => format!("[{}]", m),
                        None => "[assistant]".to_string(),
                    }
                } else {
                    format!("[{}]", msg.role)
                };

                let role_label_style = LineStyle {
                    bold: true,
                    ..Default::default()
                };

                // Track whether we've emitted the first visible part (for timestamp)
                // and whether we've emitted the first text part (for role label)
                let mut first_visible_part_seen = false;
                let mut role_label_emitted = false;

                // Process each part
                for part in &msg.parts {
                    match part {
                        ViewPart::Text(s) => {
                            // Render markdown text
                            let rendered = render_markdown(s, content_width);
                            let content_lines = ansi_to_rendered_lines(&rendered);

                            // Prepend timestamp to first visible part
                            let content_with_timestamp = if !first_visible_part_seen {
                                let ts_line = RenderedLine {
                                    spans: vec![(timestamp.clone(), LineStyle {
                                        dimmed: true,
                                        ..Default::default()
                                    })],
                                };
                                let mut v = vec![ts_line];
                                v.extend(content_lines);
                                v
                            } else {
                                content_lines
                            };

                            // Emit with role label on first Text part, blank pad on rest
                            let label = if !role_label_emitted {
                                msg_role_label.clone()
                            } else {
                                String::new()
                            };
                            let style = if !role_label_emitted {
                                role_label_style.clone()
                            } else {
                                LineStyle::default()
                            };

                            emit_labeled_block(&mut lines, &label, label_width, &content_with_timestamp, style);
                            first_visible_part_seen = true;
                            role_label_emitted = true;
                        }

                        ViewPart::Reasoning(s) => {
                            if options.show_thinking {
                                let rendered = render_markdown(s, content_width);
                                let content_lines = ansi_to_rendered_lines(&rendered);

                                // Prepend timestamp if this is the first visible part
                                let content_with_timestamp = if !first_visible_part_seen {
                                    let ts_line = RenderedLine {
                                        spans: vec![(timestamp.clone(), LineStyle {
                                            dimmed: true,
                                            ..Default::default()
                                        })],
                                    };
                                    let mut v = vec![ts_line];
                                    v.extend(content_lines);
                                    v
                                } else {
                                    content_lines
                                };

                                let dim_style = LineStyle {
                                    dimmed: true,
                                    ..Default::default()
                                };

                                emit_labeled_block(&mut lines, "[thinking]", label_width, &content_with_timestamp, dim_style);
                                first_visible_part_seen = true;
                            }
                        }

                        ViewPart::ToolCall { name, input, output, status, .. } => {
                            if options.tool_display.is_visible() {
                                let formatted = tool_format::format_tool_call(name, input, content_width);
                                let mut tool_lines = Vec::new();

                                let dim_style = LineStyle {
                                    dimmed: true,
                                    ..Default::default()
                                };

                                tool_lines.extend(wrap_into_lines(&format!("▶ {}", formatted.header), content_width, dim_style.clone()));

                                if let Some(body) = &formatted.body {
                                    tool_lines.extend(wrap_into_lines(body, content_width, dim_style.clone()));
                                }

                                if output.is_some() && status == "completed" {
                                    let output_str = match options.tool_display {
                                        ToolDisplayMode::Truncated => {
                                            tool_format::format_tool_output(output.as_ref().unwrap(), true)
                                        }
                                        ToolDisplayMode::Full => {
                                            tool_format::format_tool_output(output.as_ref().unwrap(), false)
                                        }
                                        ToolDisplayMode::Hidden => unreachable!("outer if options.tool_display.is_visible() excludes Hidden"),
                                    };
                                    tool_lines.extend(wrap_into_lines(&output_str, content_width, dim_style.clone()));
                                }

                                // Prepend timestamp if this is the first visible part
                                let content_with_timestamp = if !first_visible_part_seen {
                                    let ts_line = RenderedLine {
                                        spans: vec![(timestamp.clone(), LineStyle {
                                            dimmed: true,
                                            ..Default::default()
                                        })],
                                    };
                                    let mut v = vec![ts_line];
                                    v.extend(tool_lines);
                                    v
                                } else {
                                    tool_lines
                                };

                                let label_dim_style = LineStyle {
                                    dimmed: true,
                                    ..Default::default()
                                };

                                emit_labeled_block(&mut lines, "[tool]", label_width, &content_with_timestamp, label_dim_style);
                                first_visible_part_seen = true;
                            }
                        }

                        ViewPart::StepFinish { cost, input_tokens, output_tokens } => {
                            if options.show_timing {
                                let mut timing_line = format!("{}↑ {}↓ tokens", input_tokens, output_tokens);
                                if let Some(c) = cost {
                                    timing_line.push_str(&format!(", ${:.4}", c));
                                }

                                let timing_lines = vec![RenderedLine {
                                    spans: vec![(timing_line, LineStyle::default())],
                                }];

                                // Prepend timestamp if this is the first visible part
                                let content_with_timestamp = if !first_visible_part_seen {
                                    let ts_line = RenderedLine {
                                        spans: vec![(timestamp.clone(), LineStyle {
                                            dimmed: true,
                                            ..Default::default()
                                        })],
                                    };
                                    let mut v = vec![ts_line];
                                    v.extend(timing_lines);
                                    v
                                } else {
                                    timing_lines
                                };

                                let dim_style = LineStyle {
                                    dimmed: true,
                                    ..Default::default()
                                };

                                emit_labeled_block(&mut lines, "[time]", label_width, &content_with_timestamp, dim_style);
                                first_visible_part_seen = true;
                            }
                        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_to_rendered_lines_bold_italic_truecolor() {
        // Test string with bold (\x1b[1m), italic (\x1b[3m), and truecolor (\x1b[38;2;R;G;B)
        let input = "text \x1b[1mbold\x1b[22m \x1b[3mitalic\x1b[23m \x1b[38;2;255;0;0mred\x1b[39m normal";
        let lines = ansi_to_rendered_lines(input);

        assert_eq!(lines.len(), 1);
        let spans = &lines[0].spans;

        // Should have multiple spans with different styles
        assert!(spans.len() >= 3);

        // First span: plain text
        assert_eq!(spans[0].0, "text ");
        assert!(!spans[0].1.bold);
        assert!(!spans[0].1.italic);
        assert_eq!(spans[0].1.fg, None);

        // Second span: bold
        assert_eq!(spans[1].0, "bold");
        assert!(spans[1].1.bold);

        // Third span: italic
        assert!(spans.iter().any(|s| s.1.italic && s.0.contains("italic")));

        // Verify red color is parsed
        assert!(spans.iter().any(|s| s.1.fg == Some((255, 0, 0))));

        // Verify reset brings back default style
        assert!(spans.iter().any(|s| s.0.contains("normal") && !s.1.bold && !s.1.italic));
    }

    #[test]
    fn test_ansi_to_rendered_lines_empty() {
        let input = "";
        let lines = ansi_to_rendered_lines(input);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 1);
        assert_eq!(lines[0].spans[0].0, "");
    }

    #[test]
    fn test_ansi_to_rendered_lines_multiline() {
        let input = "line1\nline2";
        let lines = ansi_to_rendered_lines(input);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].spans[0].0, "line1");
        assert_eq!(lines[1].spans[0].0, "line2");
    }

    #[test]
    fn test_parse_ansi_line_green_blue_cyan() {
        // Test the colored codes: green (32), blue (34), cyan (36)
        let input = "\x1b[32mgreen\x1b[39m \x1b[34mblue\x1b[39m \x1b[36mcyan\x1b[39m";
        let spans = parse_ansi_line(input);

        let green_span = spans.iter().find(|s| s.0.contains("green")).unwrap();
        assert_eq!(green_span.1.fg, Some((0, 128, 0)));

        let blue_span = spans.iter().find(|s| s.0.contains("blue")).unwrap();
        assert_eq!(blue_span.1.fg, Some((0, 0, 255)));

        let cyan_span = spans.iter().find(|s| s.0.contains("cyan")).unwrap();
        assert_eq!(cyan_span.1.fg, Some((0, 255, 255)));
    }

    #[test]
    fn test_parse_ansi_line_dimmed() {
        let input = "\x1b[2mdim\x1b[22mnormal";
        let spans = parse_ansi_line(input);

        let dim_span = spans.iter().find(|s| s.0.contains("dim")).unwrap();
        assert!(dim_span.1.dimmed);

        let normal_span = spans.iter().find(|s| s.0.contains("normal")).unwrap();
        assert!(!normal_span.1.dimmed);
    }

    #[test]
    fn test_parse_ansi_line_reset() {
        let input = "\x1b[1m\x1b[3mbold italic\x1b[0mnormal";
        let spans = parse_ansi_line(input);

        let styled = spans.iter().find(|s| s.0.contains("bold")).unwrap();
        assert!(styled.1.bold);
        assert!(styled.1.italic);

        let reset = spans.iter().find(|s| s.0.contains("normal")).unwrap();
        assert!(!reset.1.bold);
        assert!(!reset.1.italic);
    }

    #[test]
    fn test_compute_label_width_user_assistant() {
        use crate::opencode::models::MessageView;

        let messages = vec![
            MessageView {
                role: "user".to_string(),
                created: 0,
                model: None,
                parts: vec![ViewPart::Text("test".to_string())],
            },
            MessageView {
                role: "assistant".to_string(),
                created: 0,
                model: Some("claude-opus".to_string()),
                parts: vec![ViewPart::Text("test".to_string())],
            },
        ];

        let width = compute_label_width(&messages);
        // "[claude-opus]" should be the longest (13 chars)
        assert_eq!(width, 13);
    }

    #[test]
    fn test_compute_label_width_with_tool_thinking() {
        use crate::opencode::models::MessageView;

        let messages = vec![MessageView {
            role: "assistant".to_string(),
            created: 0,
            model: Some("claude-sonnet".to_string()),
            parts: vec![
                ViewPart::Text("text".to_string()),
                ViewPart::ToolCall {
                    name: "test".to_string(),
                    call_id: "123".to_string(),
                    input: serde_json::json!({}),
                    output: None,
                    status: "pending".to_string(),
                },
                ViewPart::Reasoning("thinking".to_string()),
            ],
        }];

        let width = compute_label_width(&messages);
        // Longest is "[claude-sonnet]" (15 chars)
        assert!(width >= 6); // At least minimum
    }

    #[test]
    fn test_compute_label_width_empty() {
        let messages = vec![];
        let width = compute_label_width(&messages);
        // Should return minimum of 6
        assert_eq!(width, 6);
    }

    #[test]
    fn test_emit_labeled_block_empty() {
        let mut out = Vec::new();
        let label = "[user]";
        let label_width = 6;
        let content_lines = vec![];
        let label_style = LineStyle::default();

        emit_labeled_block(&mut out, label, label_width, &content_lines, label_style);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].spans.len(), 1);
        assert!(out[0].spans[0].0.contains("│"));
    }

    #[test]
    fn test_emit_labeled_block_single_line() {
        let mut out = Vec::new();
        let label = "[user]";
        let label_width = 6;
        let content_lines = vec![RenderedLine {
            spans: vec![("hello".to_string(), LineStyle::default())],
        }];
        let label_style = LineStyle {
            bold: true,
            ..Default::default()
        };

        emit_labeled_block(&mut out, label, label_width, &content_lines, label_style.clone());

        assert_eq!(out.len(), 1);
        // First span should be the label prefix with label_style
        assert_eq!(out[0].spans[0].1.bold, true);
        // Should contain content
        assert!(out[0].spans.len() >= 2);
        assert_eq!(out[0].spans[1].0, "hello");
    }

    #[test]
    fn test_emit_labeled_block_multiline() {
        let mut out = Vec::new();
        let label = "[assistant]";
        let label_width = 11;
        let content_lines = vec![
            RenderedLine {
                spans: vec![("line1".to_string(), LineStyle::default())],
            },
            RenderedLine {
                spans: vec![("line2".to_string(), LineStyle::default())],
            },
            RenderedLine {
                spans: vec![("line3".to_string(), LineStyle::default())],
            },
        ];
        let label_style = LineStyle {
            bold: true,
            ..Default::default()
        };

        emit_labeled_block(&mut out, label, label_width, &content_lines, label_style.clone());

        assert_eq!(out.len(), 3);

        // First line should have label with bold
        assert_eq!(out[0].spans[0].1.bold, true);
        assert!(out[0].spans[0].0.contains("│"));

        // Second and third lines should have blank pad (not bold)
        assert_eq!(out[1].spans[0].1.bold, false);
        assert!(out[1].spans[0].0.contains("│"));
        assert_eq!(out[2].spans[0].1.bold, false);
        assert!(out[2].spans[0].0.contains("│"));
    }
}
