//! Conversation export functionality for opencode sessions.
//!
//! This module provides functions to export conversations in different formats:
//! - Ledger format (formatted text with speaker names)
//! - Plain text (simple speaker: message format)
//! - Markdown (with headers for speakers)
//! - Operator Markdown (dialogue only, no tools/thinking)
//!
//! Conversations can be exported to files or copied to the clipboard.
//! Export respects the current display settings for thinking blocks and tool calls.

use crate::opencode::models::{OcSessionView, ViewPart};
use crate::tui::ToolDisplayMode;
use crate::tool_format;
use std::process::{Command, Stdio};
#[cfg(target_os = "linux")]
use std::io::Write as _;

/// Export format options
#[derive(Clone, Copy, Debug)]
pub enum ExportFormat {
    Ledger,
    Plain,
    Markdown,
}

impl ExportFormat {
    /// Get format from menu option index (0-2)
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(ExportFormat::Ledger),
            1 => Some(ExportFormat::Plain),
            2 => Some(ExportFormat::Markdown),
            _ => None,
        }
    }

    /// Get file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Ledger | ExportFormat::Plain => "txt",
            ExportFormat::Markdown => "md",
        }
    }
}

/// Sanitize a string for use as a filename
pub fn sanitize_filename(s: &str) -> String {
    let s: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let s = s
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let s = s.trim_matches('-').to_string();
    let s = if s.len() > 60 { s[..60].to_string() } else { s };
    if s.is_empty() {
        "session".to_string()
    } else {
        s
    }
}

/// Total line width for ledger export (including name column and separator)
const LEDGER_WIDTH: usize = 90;

/// Copy text to the system clipboard.
///
/// On Linux, selects clipboard tools based on the display server: `wl-copy`
/// for Wayland, `xclip`/`xsel` for X11. These persist clipboard data
/// independently of the calling process (unlike arboard, which loses
/// contents when the process exits). Falls back to arboard if no external
/// tool is available.
pub fn copy_to_system_clipboard(text: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let candidates = linux_clipboard_candidates();
        for (cmd, args) in &candidates {
            match copy_via_command(cmd, args, text) {
                Ok(Ok(())) => return Ok(()),
                Ok(Err(_)) => continue, // command found but failed, try next
                Err(()) => continue,    // command not found, try next
            }
        }
        // Fall through to arboard
    }

    // arboard fallback (primary method on macOS/Windows)
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => clipboard
            .set_text(text)
            .map_err(|e| format!("Clipboard error: {}", e)),
        Err(e) => Err(format!("Clipboard unavailable: {}", e)),
    }
}

/// Return clipboard tool candidates based on the active display server.
#[cfg(target_os = "linux")]
fn linux_clipboard_candidates() -> Vec<(&'static str, &'static [&'static str])> {
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let x11 = std::env::var_os("DISPLAY").is_some();

    let mut candidates = Vec::new();
    if wayland {
        candidates.push(("wl-copy", ["--type", "text/plain;charset=utf-8"].as_slice()));
    }
    if x11 {
        candidates.push(("xclip", ["-selection", "clipboard"].as_slice()));
        candidates.push(("xsel", ["--clipboard", "--input"].as_slice()));
    }
    candidates
}

/// Try to copy text via an external command (e.g. wl-copy, xclip, xsel).
/// Returns `Ok(Ok(()))` on success, `Ok(Err(msg))` if the command ran but failed,
/// or `Err(())` if the command was not found (caller should try next option).
#[cfg(target_os = "linux")]
fn copy_via_command(cmd: &str, args: &[&str], text: &str) -> Result<Result<(), String>, ()> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ())?; // command not available → try next

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
    }

    match child.wait() {
        Ok(status) if status.success() => Ok(Ok(())),
        Ok(status) => Ok(Err(format!("{} exited with {}", cmd, status))),
        Err(e) => Ok(Err(format!("{} error: {}", cmd, e))),
    }
}

/// Render conversation in the specified format
pub fn render_oc_export(
    session: &OcSessionView,
    format: ExportFormat,
    tool_display: ToolDisplayMode,
    show_thinking: bool,
    show_timing: bool,
) -> String {
    match format {
        ExportFormat::Plain => render_oc_plain(session, tool_display, show_thinking, show_timing),
        ExportFormat::Markdown => render_oc_markdown(session, tool_display, show_thinking, show_timing),
        ExportFormat::Ledger => render_oc_ledger(session, tool_display, show_thinking, show_timing),
    }
}

/// Render in plain text format: "User:\n{text}" / "Assistant:\n{text}"
fn render_oc_plain(
    session: &OcSessionView,
    tool_display: ToolDisplayMode,
    show_thinking: bool,
    show_timing: bool,
) -> String {
    let mut output = String::new();

    for message in &session.messages {
        let role_label = if message.role == "user" { "User" } else { "Assistant" };

        for part in &message.parts {
            match part {
                ViewPart::Text(text) => {
                    if !output.is_empty() {
                        output.push_str("\n\n");
                    }
                    output.push_str(&format!("{}:\n{}", role_label, text));
                }
                ViewPart::ToolCall { name, input, output: tool_output, status, .. } if tool_display.is_visible() => {
                    if !output.is_empty() {
                        output.push_str("\n\n");
                    }
                    let formatted = tool_format::format_tool_call(name, input, usize::MAX);
                    let tool_text = match formatted.body {
                        Some(body) => format!("{}\n{}", formatted.header, body),
                        None => formatted.header,
                    };
                    output.push_str(&format!("Tool: {}\n{}", name, tool_text));

                    if let Some(tool_out) = tool_output {
                        output.push_str("\nTool Result:\n");
                        let result_text = serde_json::to_string_pretty(tool_out)
                            .unwrap_or_else(|_| "<error>".to_string());
                        output.push_str(&result_text);
                    }
                }
                ViewPart::Reasoning(text) if show_thinking => {
                    if !output.is_empty() {
                        output.push_str("\n\n");
                    }
                    output.push_str(&format!("Thinking:\n{}", text));
                }
                ViewPart::StepFinish { cost, input_tokens, output_tokens } if show_timing => {
                    if !output.is_empty() {
                        output.push_str("\n");
                    }
                    output.push_str("\n[Step finish]");
                    if let Some(c) = cost {
                        output.push_str(&format!(" cost: ${:.6}", c));
                    }
                    output.push_str(&format!(" tokens: {}/{}", input_tokens, output_tokens));
                }
                _ => {}
            }
        }
    }

    output
}

/// Render in markdown format: "## User\n\n{text}" / "## Assistant\n\n{text}"
/// Tools in fenced code block, thinking as blockquote
fn render_oc_markdown(
    session: &OcSessionView,
    tool_display: ToolDisplayMode,
    show_thinking: bool,
    show_timing: bool,
) -> String {
    let mut output = String::new();

    for message in &session.messages {
        let role_label = if message.role == "user" { "User" } else { "Assistant" };

        for part in &message.parts {
            match part {
                ViewPart::Text(text) => {
                    if !output.is_empty() {
                        output.push_str("\n");
                    }
                    output.push_str(&format!("## {}\n\n{}\n", role_label, text));
                }
                ViewPart::ToolCall { name, input, output: tool_output, .. } if tool_display.is_visible() => {
                    if !output.is_empty() {
                        output.push_str("\n");
                    }
                    let formatted = tool_format::format_tool_call(name, input, usize::MAX);
                    let tool_text = match formatted.body {
                        Some(body) => format!("{}\n{}", formatted.header, body),
                        None => formatted.header,
                    };
                    let fenced = markdown_code_fence(&tool_text);
                    output.push_str(&format!("### Tool: {}\n\n{}\n", name, fenced));

                    if let Some(tool_out) = tool_output {
                        output.push_str("\n### Tool Result\n\n");
                        let result_text = serde_json::to_string_pretty(tool_out)
                            .unwrap_or_else(|_| "<error>".to_string());
                        let result_fenced = markdown_code_fence(&result_text);
                        output.push_str(&format!("{}\n", result_fenced));
                    }
                }
                ViewPart::Reasoning(text) if show_thinking => {
                    if !output.is_empty() {
                        output.push_str("\n");
                    }
                    output.push_str("### Thinking\n\n");
                    for line in text.lines() {
                        output.push_str(&format!("> {}\n", line));
                    }
                    output.push('\n');
                }
                ViewPart::StepFinish { cost, input_tokens, output_tokens } if show_timing => {
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str("**[Step finish]**");
                    if let Some(c) = cost {
                        output.push_str(&format!(" cost: ${:.6}", c));
                    }
                    output.push_str(&format!(" tokens: {}/{}\n", input_tokens, output_tokens));
                }
                _ => {}
            }
        }
    }

    output
}

/// Render in ledger format with 9-char name column and "│" separator
fn render_oc_ledger(
    session: &OcSessionView,
    tool_display: ToolDisplayMode,
    show_thinking: bool,
    show_timing: bool,
) -> String {
    let mut output = String::new();
    const NAME_WIDTH: usize = 9;
    let content_width = LEDGER_WIDTH - NAME_WIDTH - 3;

    for message in &session.messages {
        let speaker = if message.role == "user" {
            "You".to_string()
        } else {
            "Assistant".to_string()
        };

        for part in &message.parts {
            match part {
                ViewPart::Text(text) => {
                    let wrapped = wrap_plain_text(text, content_width);
                    append_ledger_block(&mut output, &speaker, &wrapped, NAME_WIDTH);
                    output.push('\n');
                }
                ViewPart::ToolCall { name, input, output: tool_output, .. } if tool_display.is_visible() => {
                    let formatted = tool_format::format_tool_call(name, input, content_width);
                    let tool_text = match formatted.body {
                        Some(body) => format!("{}\n{}", formatted.header, body),
                        None => formatted.header,
                    };
                    let wrapped = wrap_plain_text(&tool_text, content_width);
                    append_ledger_block(&mut output, "Tool", &wrapped, NAME_WIDTH);
                    output.push('\n');

                    if let Some(tool_out) = tool_output {
                        let result_text = serde_json::to_string_pretty(tool_out)
                            .unwrap_or_else(|_| "<error>".to_string());
                        let wrapped = wrap_plain_text(&result_text, content_width);
                        append_ledger_block(&mut output, "Result", &wrapped, NAME_WIDTH);
                        output.push('\n');
                    }
                }
                ViewPart::Reasoning(text) if show_thinking => {
                    let wrapped = wrap_plain_text(text, content_width);
                    append_ledger_block(&mut output, "Thinking", &wrapped, NAME_WIDTH);
                    output.push('\n');
                }
                ViewPart::StepFinish { cost, input_tokens, output_tokens } if show_timing => {
                    let mut timing_str = "[Step finish]".to_string();
                    if let Some(c) = cost {
                        timing_str.push_str(&format!(" cost: ${:.6}", c));
                    }
                    timing_str.push_str(&format!(" tokens: {}/{}", input_tokens, output_tokens));
                    append_ledger_block(&mut output, "Time", &timing_str, NAME_WIDTH);
                    output.push('\n');
                }
                _ => {}
            }
        }
    }

    output
}

/// Append a ledger-formatted block to the output
fn append_ledger_block(output: &mut String, speaker: &str, text: &str, name_width: usize) {
    for (i, line) in text.lines().enumerate() {
        if i == 0 {
            output.push_str(&format!(
                "{:>width$} │ {}\n",
                speaker,
                line,
                width = name_width
            ));
        } else {
            output.push_str(&format!("{:>width$} │ {}\n", "", line, width = name_width));
        }
    }
}

/// Wrap plain text to max_width, preserving existing line breaks
fn wrap_plain_text(text: &str, max_width: usize) -> String {
    let mut result = String::new();
    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            result.push('\n');
        }
        if line.is_empty() {
            continue;
        }
        let wrapped: Vec<_> = textwrap::wrap(line, max_width)
            .into_iter()
            .map(|cow| cow.into_owned())
            .collect();
        for (j, w) in wrapped.iter().enumerate() {
            if j > 0 {
                result.push('\n');
            }
            result.push_str(w);
        }
    }
    result
}

/// Wrap content in markdown code fence, handling nested backticks
fn markdown_code_fence(content: &str) -> String {
    // Find the longest run of backticks in content and use one more
    let max_backticks = content
        .split(|c| c != '`')
        .map(|s| s.len())
        .max()
        .unwrap_or(0);
    let fence_len = std::cmp::max(3, max_backticks + 1);
    let fence: String = std::iter::repeat_n('`', fence_len).collect();
    format!("{}\n{}\n{}", fence, content, fence)
}
