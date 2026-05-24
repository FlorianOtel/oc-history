//! Markdown rendering
//!
//! Converts markdown text to styled strings with line wrapping.
//! Supports two modes:
//! - ANSI: colored terminal output with syntax highlighting
//! - Plain: clean plain text for export/clipboard (no escape codes)

use colored::{ColoredString, Colorize};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use unicode_width::UnicodeWidthStr;

/// Render mode controls whether output includes ANSI escape codes
#[derive(Clone, Copy, PartialEq)]
enum RenderMode {
    /// ANSI-styled output for terminal display
    Ansi,
    /// Plain text output for export/clipboard
    Plain,
}

/// Render markdown text to ANSI-styled string with line wrapping
pub fn render_markdown(input: &str, max_width: usize) -> String {
    render_markdown_with_mode(input, max_width, RenderMode::Ansi)
}

/// Render markdown text to plain text (no ANSI codes) with line wrapping
pub fn render_markdown_plain(input: &str, max_width: usize) -> String {
    render_markdown_with_mode(input, max_width, RenderMode::Plain)
}

fn render_markdown_with_mode(input: &str, max_width: usize, mode: RenderMode) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(input, options);
    let mut renderer = MarkdownRenderer::new(max_width, mode);

    for event in parser {
        renderer.handle_event(event);
    }

    renderer.finish()
}

struct MarkdownRenderer {
    output: String,
    max_width: usize,
    mode: RenderMode,
    style_stack: Vec<TextStyle>,
    list_stack: Vec<ListContext>,
    in_code_block: bool,
    code_block_lang: String,
    code_block_content: String,
    pending_text: String,
    at_line_start: bool,
    in_list_item_start: bool, // Suppress paragraph newline right after list bullet
    item_indent: usize,       // Width of current list item prefix for continuation lines
    table_state: Option<TableState>,
}

struct TableState {
    rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    current_cell: String,
}

impl TableState {
    fn new() -> Self {
        Self {
            rows: Vec::new(),
            current_row: Vec::new(),
            current_cell: String::new(),
        }
    }
}

#[derive(Clone)]
struct ListContext {
    index: Option<u64>,
    depth: usize,
}

#[derive(Clone)]
enum TextStyle {
    Bold,
    Italic,
    Strikethrough,
    Quote,
    Link(String),
}

impl MarkdownRenderer {
    fn new(max_width: usize, mode: RenderMode) -> Self {
        Self {
            output: String::new(),
            max_width,
            mode,
            style_stack: vec![],
            list_stack: vec![],
            in_code_block: false,
            code_block_lang: String::new(),
            code_block_content: String::new(),
            pending_text: String::new(),
            at_line_start: true,
            in_list_item_start: false,
            item_indent: 0,
            table_state: None,
        }
    }

    fn is_plain(&self) -> bool {
        self.mode == RenderMode::Plain
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => self.text(&text),
            Event::Code(code) => self.inline_code(&code),
            Event::SoftBreak => self.soft_break(),
            Event::HardBreak => self.hard_break(),
            Event::Rule => self.rule(),
            _ => {}
        }
    }

    fn start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {
                self.flush_pending();
                // Don't add newline if we just started a list item (bullet is on same line)
                if !self.in_list_item_start && !self.output.is_empty() {
                    // Add blank line before paragraph for visual separation (like original)
                    if !self.output.ends_with("\n\n") {
                        if !self.output.ends_with('\n') {
                            self.output.push('\n');
                        }
                        self.output.push('\n');
                    }
                }
                self.in_list_item_start = false;
            }
            Tag::Heading { level, .. } => {
                self.flush_pending();
                // Add blank line before heading for visual separation
                if !self.output.is_empty() && !self.output.ends_with("\n\n") {
                    if !self.output.ends_with('\n') {
                        self.output.push('\n');
                    }
                    self.output.push('\n');
                }
                let hashes = heading_level_to_usize(level);
                let prefix = "#".repeat(hashes);
                if self.is_plain() {
                    self.output.push_str(&format!("{} ", prefix));
                } else {
                    self.output
                        .push_str(&format!("{} ", prefix).cyan().bold().to_string());
                }
            }
            Tag::CodeBlock(kind) => {
                self.flush_pending();
                self.in_code_block = true;
                self.code_block_content.clear();
                // Add blank line before code block for visual separation
                if !self.output.is_empty() && !self.output.ends_with("\n\n") {
                    if !self.output.ends_with('\n') {
                        self.output.push('\n');
                    }
                    self.output.push('\n');
                }
                let lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                self.code_block_lang = lang.clone();
                let fence = if lang.is_empty() {
                    "```".to_string()
                } else {
                    format!("```{}", lang)
                };
                if self.is_plain() {
                    self.output.push_str(&fence);
                } else {
                    self.output.push_str(&fence.dimmed().to_string());
                }
                self.output.push('\n');
            }
            Tag::List(start) => {
                self.flush_pending();
                // Add blank line before list for visual separation
                if !self.output.is_empty() && !self.output.ends_with("\n\n") {
                    if !self.output.ends_with('\n') {
                        self.output.push('\n');
                    }
                    self.output.push('\n');
                }
                let depth = self.list_stack.len();
                self.list_stack.push(ListContext {
                    index: start,
                    depth,
                });
            }
            Tag::Item => {
                self.flush_pending();
                if !self.output.is_empty() && !self.output.ends_with('\n') {
                    self.output.push('\n');
                }
                let indent = if let Some(ctx) = self.list_stack.last() {
                    "  ".repeat(ctx.depth)
                } else {
                    String::new()
                };
                let plain = self.is_plain();
                if let Some(ctx) = self.list_stack.last_mut() {
                    match &mut ctx.index {
                        None => {
                            let bullet = format!("{}- ", indent);
                            self.item_indent = bullet.len();
                            self.output.push_str(&bullet);
                        }
                        Some(n) => {
                            let bullet = format!("{}{}. ", indent, n);
                            self.item_indent = bullet.len();
                            if plain {
                                self.output.push_str(&bullet);
                            } else {
                                self.output.push_str(&bullet.dimmed().to_string());
                            }
                            *n += 1;
                        }
                    }
                }
                self.at_line_start = false;
                self.in_list_item_start = true; // Next paragraph shouldn't add newline
            }
            Tag::Emphasis => self.style_stack.push(TextStyle::Italic),
            Tag::Strong => self.style_stack.push(TextStyle::Bold),
            Tag::Strikethrough => self.style_stack.push(TextStyle::Strikethrough),
            Tag::BlockQuote(_) => {
                self.flush_pending();
                if !self.output.is_empty() && !self.output.ends_with('\n') {
                    self.output.push('\n');
                }
                if self.is_plain() {
                    self.output.push_str("> ");
                } else {
                    self.output.push_str(&"> ".green().to_string());
                }
                self.style_stack.push(TextStyle::Quote);
            }
            Tag::Link { dest_url, .. } => {
                self.style_stack.push(TextStyle::Link(dest_url.to_string()));
            }
            Tag::Table(_alignments) => {
                self.flush_pending();
                if !self.output.is_empty() && !self.output.ends_with("\n\n") {
                    if !self.output.ends_with('\n') {
                        self.output.push('\n');
                    }
                    self.output.push('\n');
                }
                self.table_state = Some(TableState::new());
            }
            Tag::TableHead | Tag::TableRow => {
                if let Some(ref mut state) = self.table_state {
                    state.current_row = Vec::new();
                }
            }
            Tag::TableCell => {
                if let Some(ref mut state) = self.table_state {
                    state.current_cell = String::new();
                }
            }
            _ => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.flush_pending();
                self.output.push('\n');
                self.at_line_start = true;
            }
            TagEnd::Heading(_) => {
                self.flush_pending();
                self.output.push('\n');
                self.at_line_start = true;
            }
            TagEnd::CodeBlock => {
                self.in_code_block = false;

                // Wrap long lines before highlighting so they fit within max_width
                let code = std::mem::take(&mut self.code_block_content);
                let code = wrap_code_lines(&code, self.max_width);

                if self.is_plain() {
                    // Plain text: output code as-is (no syntax highlighting)
                    self.output.push_str(&code);
                } else if let Some(highlighted) =
                    crate::syntax::highlight_code_ansi(&code, &self.code_block_lang)
                {
                    self.output.push_str(&highlighted);
                } else {
                    // Fallback: apply uniform style per line for unknown languages
                    for line in code.lines() {
                        self.output.push_str(&line.on_bright_black().to_string());
                        self.output.push('\n');
                    }
                }

                // Ensure proper line ending before closing fence
                if !self.output.ends_with('\n') {
                    self.output.push('\n');
                }
                if self.is_plain() {
                    self.output.push_str("```");
                } else {
                    self.output.push_str(&"```".dimmed().to_string());
                }
                self.output.push('\n');
                self.at_line_start = true;
            }
            TagEnd::List(_) => {
                self.list_stack.pop();
                self.in_list_item_start = false; // Clear flag when list ends
            }
            TagEnd::Item => {
                self.flush_pending();
                self.item_indent = 0;
                self.in_list_item_start = false; // Clear flag when item ends
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => {
                self.style_stack.pop();
            }
            TagEnd::BlockQuote(_) => {
                self.flush_pending();
                self.style_stack.pop();
            }
            TagEnd::Link => {
                if let Some(TextStyle::Link(url)) = self.style_stack.pop() {
                    if self.is_plain() {
                        self.pending_text.push_str(&format!(" ({})", url));
                    } else {
                        self.pending_text
                            .push_str(&format!(" ({})", url).blue().underline().to_string());
                    }
                }
            }
            TagEnd::Table => {
                if let Some(state) = self.table_state.take() {
                    let rendered = render_table(&state.rows, !self.is_plain());
                    self.output.push_str(&rendered);
                }
                self.at_line_start = true;
            }
            TagEnd::TableHead | TagEnd::TableRow => {
                if let Some(ref mut state) = self.table_state {
                    let row = std::mem::take(&mut state.current_row);
                    state.rows.push(row);
                }
            }
            TagEnd::TableCell => {
                if let Some(ref mut state) = self.table_state {
                    let cell = std::mem::take(&mut state.current_cell);
                    state.current_row.push(cell);
                }
            }
            _ => {}
        }
    }

    fn text(&mut self, text: &str) {
        // Handle table cell text
        if let Some(ref mut state) = self.table_state {
            // Replace newlines with spaces to prevent breaking table layout
            state.current_cell.push_str(&text.replace('\n', " "));
            return;
        }

        if self.in_code_block {
            // Buffer code block content for syntax highlighting at block end
            self.code_block_content.push_str(text);
        } else if self.is_plain() {
            // Plain mode: no ANSI styling
            self.pending_text.push_str(text);
        } else {
            // Apply styles immediately (before they get popped from stack)
            let styled = apply_styles(text, &self.style_stack);
            self.pending_text.push_str(&styled);
        }
    }

    fn inline_code(&mut self, code: &str) {
        // Handle table cell inline code
        if let Some(ref mut state) = self.table_state {
            state.current_cell.push_str(code);
            return;
        }

        if self.is_plain() {
            // Plain mode: wrap in backticks since there's no color to distinguish
            self.pending_text.push('`');
            self.pending_text.push_str(code);
            self.pending_text.push('`');
        } else {
            // Inline code with subtle blueish color (no backticks - color distinguishes it)
            let styled = code.truecolor(147, 161, 199).to_string();
            self.pending_text.push_str(&styled);
        }
    }

    fn soft_break(&mut self) {
        // Preserve line breaks instead of converting to space (standard markdown behavior)
        // For conversation display, users expect their line breaks to be kept
        self.flush_pending();
        self.output.push('\n');
    }

    fn hard_break(&mut self) {
        self.flush_pending();
        self.output.push('\n');
        self.at_line_start = true;
    }

    fn rule(&mut self) {
        self.flush_pending();
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        let rule = "─".repeat(self.max_width.min(40));
        if self.is_plain() {
            self.output.push_str(&rule);
        } else {
            self.output.push_str(&rule.dimmed().to_string());
        }
        self.output.push('\n');
        self.at_line_start = true;
    }

    fn flush_pending(&mut self) {
        if self.pending_text.is_empty() {
            return;
        }

        let text = std::mem::take(&mut self.pending_text);

        // When inside a list item, reduce wrap width to account for the bullet prefix
        let wrap_width = if self.item_indent > 0 {
            self.max_width.saturating_sub(self.item_indent)
        } else {
            self.max_width
        };

        let wrapped = wrap_text_preserve_ansi(&text, wrap_width);

        for (i, line) in wrapped.iter().enumerate() {
            if i > 0 {
                self.output.push('\n');
                // Add continuation indent matching the list item prefix width
                if self.item_indent > 0 {
                    self.output.push_str(&" ".repeat(self.item_indent));
                }
            }
            self.output.push_str(line);
        }

        self.at_line_start = false;
    }

    fn finish(mut self) -> String {
        self.flush_pending();
        // Trim trailing whitespace but preserve structure
        while self.output.ends_with("\n\n") {
            self.output.pop();
        }
        self.output
    }
}

fn heading_level_to_usize(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn apply_styles(text: &str, styles: &[TextStyle]) -> String {
    if styles.is_empty() {
        return text.to_string();
    }

    let mut result: ColoredString = text.normal();

    for style in styles {
        result = match style {
            TextStyle::Bold => result.bold(),
            TextStyle::Italic => result.italic(),
            TextStyle::Strikethrough => result.strikethrough(),
            TextStyle::Quote => result.green(),
            TextStyle::Link(_) => result.blue().underline(),
        };
    }

    result.to_string()
}

/// Hard-wrap code block lines that exceed max_width at character boundaries.
/// Operates on plain text (before syntax highlighting) so no ANSI handling needed.
pub fn wrap_code_lines(code: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthChar;

    if max_width == 0 {
        return code.to_string();
    }

    let mut result = String::new();
    for line in code.lines() {
        let line_width = line.width();
        if line_width <= max_width {
            result.push_str(line);
            result.push('\n');
        } else {
            let mut current_width = 0;
            for ch in line.chars() {
                let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
                if current_width + ch_width > max_width && current_width > 0 {
                    result.push('\n');
                    current_width = 0;
                }
                result.push(ch);
                current_width += ch_width;
            }
            result.push('\n');
        }
    }
    result
}

/// Wrap text while preserving ANSI escape codes
/// This is a simplified approach: we strip ANSI for width calculation
fn wrap_text_preserve_ansi(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 || text.is_empty() {
        return vec![text.to_string()];
    }

    // Simple wrapping that respects word boundaries
    // ANSI codes are preserved but may cause slight width miscalculation
    textwrap::wrap(text, max_width)
        .into_iter()
        .map(|cow| cow.into_owned())
        .collect()
}

/// Render a table with box-drawing characters
/// When `styled` is true, borders are dimmed with ANSI codes.
fn render_table(rows: &[Vec<String>], styled: bool) -> String {
    if rows.is_empty() {
        return String::new();
    }

    // Calculate column widths based on display width (handles Unicode)
    let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut col_widths = vec![0usize; num_cols];

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                col_widths[i] = col_widths[i].max(cell.trim().width());
            }
        }
    }

    // Box-drawing characters
    let h = '─'; // horizontal
    let v = '│'; // vertical
    let tl = '┌'; // top-left
    let tr = '┐'; // top-right
    let bl = '└'; // bottom-left
    let br = '┘'; // bottom-right
    let lj = '├'; // left junction
    let rj = '┤'; // right junction
    let tj = '┬'; // top junction
    let bj = '┴'; // bottom junction
    let cj = '┼'; // center junction

    let mut output = String::new();

    // Optionally apply dimmed style to border text
    let dim = |s: String| -> String { if styled { s.dimmed().to_string() } else { s } };

    // Helper to build horizontal line
    let build_line = |left: char, mid: char, right: char| -> String {
        let mut line = String::new();
        line.push(left);
        for (i, &width) in col_widths.iter().enumerate() {
            line.extend(std::iter::repeat_n(h, width + 2)); // +2 for padding
            if i < col_widths.len() - 1 {
                line.push(mid);
            }
        }
        line.push(right);
        line.push('\n');
        line
    };

    // Top border (dimmed like code fences)
    output.push_str(&dim(build_line(tl, tj, tr)));

    // Rows with separators
    for (row_idx, row) in rows.iter().enumerate() {
        // Row content
        output.push_str(&dim(v.to_string()));
        for (i, width) in col_widths.iter().enumerate() {
            let cell = row.get(i).map(|s| s.trim()).unwrap_or("");
            let cell_width = cell.width();
            let padding = width.saturating_sub(cell_width);
            output.push(' ');
            output.push_str(cell);
            output.push_str(&" ".repeat(padding + 1));
            output.push_str(&dim(v.to_string()));
        }
        output.push('\n');

        // Separator (between all rows)
        if row_idx < rows.len() - 1 {
            output.push_str(&dim(build_line(lj, cj, rj)));
        }
    }

    // Bottom border (dimmed like code fences)
    output.push_str(&dim(build_line(bl, bj, br)));

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let result = render_markdown("Hello world", 80);
        assert_eq!(result.trim(), "Hello world");
    }

    #[test]
    fn test_inline_code() {
        let result = render_markdown("Use `foo()` here", 80);
        assert!(result.contains("foo()"));
    }

    #[test]
    fn test_bold() {
        // Force colors for test
        colored::control::set_override(true);
        let result = render_markdown("This is **bold** text", 80);
        assert!(result.contains("bold"));
        // Check for ANSI bold code (ESC[1m)
        assert!(
            result.contains("\x1b[1m"),
            "Expected bold ANSI codes in: {:?}",
            result
        );
    }

    #[test]
    fn test_code_block() {
        colored::control::set_override(true);
        let result = render_markdown("```rust\nlet x = 1;\n```", 80);
        // With syntax highlighting, tokens are split with ANSI codes
        // Check individual tokens are present
        assert!(result.contains("let"));
        assert!(result.contains("x"));
        assert!(result.contains("1"));
        assert!(result.contains("```"));
        // Verify syntax highlighting is applied (ANSI color codes present)
        assert!(
            result.contains("\x1b[38;2;"),
            "Expected syntax highlighting ANSI codes in: {:?}",
            result
        );
    }

    #[test]
    fn test_list() {
        let result = render_markdown("- item 1\n- item 2", 80);
        assert!(result.contains("- item 1"));
        assert!(result.contains("- item 2"));
    }

    #[test]
    fn test_heading() {
        let result = render_markdown("# Heading", 80);
        assert!(result.contains("#"));
        assert!(result.contains("Heading"));
    }

    #[test]
    fn test_linebreaks_preserved() {
        let input = "Line one here\nLine two here\nLine three";
        let result = render_markdown(input, 80);
        // Should have newlines between lines
        let lines: Vec<&str> = result.lines().collect();
        eprintln!("DEBUG lines: {:?}", lines);
        assert!(
            lines.len() >= 3,
            "Expected at least 3 lines, got {}: {:?}",
            lines.len(),
            lines
        );
    }

    #[test]
    fn test_paragraph_then_list() {
        let input = "Some text here:\n- Item one\n- Item two";
        let result = render_markdown(input, 80);
        eprintln!("DEBUG output:\n{}", result);
        eprintln!("DEBUG escaped: {:?}", result);
        // Should have newline between text and list
        assert!(result.contains("here:\n"), "Expected newline after colon");
    }

    #[test]
    fn test_list_then_paragraph() {
        let input = "- Item with text\n- Another item\n\nParagraph after list.";
        let result = render_markdown(input, 80);
        eprintln!("DEBUG output:\n{}", result);
        eprintln!("DEBUG escaped: {:?}", result);
        // Should have newline between list and paragraph
        assert!(
            result.contains("item\n"),
            "Expected newline after list item"
        );
        assert!(
            result.contains("\nParagraph"),
            "Expected paragraph on new line"
        );
    }

    #[test]
    fn test_complex_structure() {
        let input = r#"Arguments: `--no-review` task description
- Detects OS
- Downloads binary

Next paragraph here."#;
        let result = render_markdown(input, 80);
        eprintln!("DEBUG output:\n{}", result);
        eprintln!("DEBUG escaped: {:?}", result);
    }

    #[test]
    fn test_blank_line_before_list() {
        let input = "Some intro text:\n1. First item\n2. Second item";
        let result = render_markdown(input, 80);
        eprintln!("DEBUG output:\n{}", result);
        eprintln!("DEBUG escaped: {:?}", result);
        // Should have blank line between text and list
        assert!(
            result.contains("text:\n\n"),
            "Expected blank line before list, got: {:?}",
            result
        );
    }

    #[test]
    fn test_code_block_wrapping() {
        // Use plain mode to test wrapping without ANSI codes affecting width
        let long_line = "x".repeat(100);
        let input = format!("```\n{}\n```", long_line);
        let result = render_markdown_plain(&input, 40);
        // Every output line should fit within max_width
        for line in result.lines() {
            let width = UnicodeWidthStr::width(line);
            assert!(
                width <= 40,
                "Line exceeds max_width ({}): {:?}",
                width,
                line
            );
        }
        // Content should still be present (just wrapped)
        let total_x: usize = result.lines().map(|l| l.matches('x').count()).sum();
        assert_eq!(total_x, 100, "All characters should be preserved");
    }

    #[test]
    fn test_table_basic() {
        let input = r#"| A | B |
|---|---|
| 1 | 2 |"#;
        let result = render_markdown(input, 80);
        eprintln!("Table output:\n{}", result);
        assert!(result.contains("┌"), "Expected top-left corner");
        assert!(result.contains("│"), "Expected vertical border");
        assert!(result.contains("└"), "Expected bottom-left corner");
        assert!(result.contains(" A "), "Expected cell A");
        assert!(result.contains(" B "), "Expected cell B");
        assert!(result.contains(" 1 "), "Expected cell 1");
        assert!(result.contains(" 2 "), "Expected cell 2");
    }

    #[test]
    fn test_table_column_widths() {
        let input = r#"| Column A | Column B |
|----------|----------|
| Short    | Longer text |"#;
        let result = render_markdown(input, 80);
        eprintln!("Table output:\n{}", result);
        // Columns should be sized to fit longest content
        assert!(result.contains("Column A"), "Expected Column A");
        assert!(result.contains("Longer text"), "Expected Longer text");
    }

    #[test]
    fn test_table_multiple_rows() {
        let input = r#"| H1 | H2 | H3 |
|----|----|----|
| A  | B  | C  |
| D  | E  | F  |
| G  | H  | I  |"#;
        let result = render_markdown(input, 80);
        eprintln!("Table output:\n{}", result);
        // Should have separators between rows
        assert!(result.contains("├"), "Expected row separators");
        assert!(result.contains("┼"), "Expected cross junctions");
    }

    // Tests for render_markdown_plain

    #[test]
    fn test_plain_no_ansi_codes() {
        let input = "This is **bold** and *italic* and `code`";
        let result = render_markdown_plain(input, 80);
        assert!(
            !result.contains("\x1b"),
            "Plain output should not contain ANSI escape codes: {:?}",
            result
        );
    }

    #[test]
    fn test_plain_inline_code_has_backticks() {
        let result = render_markdown_plain("Use `foo()` here", 80);
        assert!(
            result.contains("`foo()`"),
            "Plain inline code should have backticks: {:?}",
            result
        );
    }

    #[test]
    fn test_plain_code_block() {
        let result = render_markdown_plain("```rust\nlet x = 1;\n```", 80);
        assert!(result.contains("```rust"), "Should have opening fence");
        assert!(result.contains("let x = 1;"), "Should have code content");
        // Count closing fences (should have opening and closing)
        assert_eq!(
            result.matches("```").count(),
            2,
            "Should have exactly 2 fences (open + close)"
        );
    }

    #[test]
    fn test_plain_heading() {
        let result = render_markdown_plain("## Heading", 80);
        assert!(
            result.contains("## Heading"),
            "Should have heading with hash prefix: {:?}",
            result
        );
    }

    #[test]
    fn test_plain_list() {
        let result = render_markdown_plain("- item 1\n- item 2", 80);
        assert!(result.contains("- item 1"), "Should have list items");
        assert!(result.contains("- item 2"), "Should have list items");
    }

    #[test]
    fn test_plain_link() {
        let result = render_markdown_plain("[click here](https://example.com)", 80);
        assert!(
            result.contains("click here"),
            "Should have link text: {:?}",
            result
        );
        assert!(
            result.contains("(https://example.com)"),
            "Should have link URL: {:?}",
            result
        );
    }

    #[test]
    fn test_plain_wrapping() {
        let long_text = "word ".repeat(20); // 100 chars
        let result = render_markdown_plain(&long_text, 40);
        for line in result.lines() {
            let width = UnicodeWidthStr::width(line);
            assert!(
                width <= 40,
                "Line exceeds max_width ({}): {:?}",
                width,
                line
            );
        }
    }

    #[test]
    fn test_plain_table() {
        let input = r#"| A | B |
|---|---|
| 1 | 2 |"#;
        let result = render_markdown_plain(input, 80);
        assert!(
            !result.contains("\x1b"),
            "Plain table should not contain ANSI: {:?}",
            result
        );
        assert!(result.contains("┌"), "Should have box-drawing chars");
        assert!(result.contains(" A "), "Should have cell content");
    }

    #[test]
    fn test_plain_block_quote() {
        let result = render_markdown_plain("> quoted text", 80);
        assert!(
            result.contains("> "),
            "Should have block quote prefix: {:?}",
            result
        );
        assert!(
            !result.contains("\x1b"),
            "Plain block quote should not contain ANSI: {:?}",
            result
        );
    }

    #[test]
    fn test_plain_horizontal_rule() {
        let result = render_markdown_plain("---", 80);
        assert!(
            result.contains("─"),
            "Should have horizontal rule: {:?}",
            result
        );
        assert!(
            !result.contains("\x1b"),
            "Plain rule should not contain ANSI: {:?}",
            result
        );
    }

    #[test]
    fn test_plain_list_continuation_indent() {
        // Long list item text should wrap with proper continuation indent
        let input = "4. This is a long list item that should wrap and the continuation line should be indented to match the bullet prefix width";
        let result = render_markdown_plain(input, 50);
        eprintln!("List continuation:\n{}", result);
        let lines: Vec<&str> = result.lines().collect();
        assert!(lines.len() > 1, "Should wrap to multiple lines");
        // First line starts with "4. "
        assert!(
            lines[0].starts_with("4. "),
            "First line should start with bullet: {:?}",
            lines[0]
        );
        // Continuation lines should be indented by 3 spaces (matching "4. " width)
        for line in &lines[1..] {
            assert!(
                line.starts_with("   "),
                "Continuation should be indented 3 spaces: {:?}",
                line
            );
        }
    }

    #[test]
    fn test_plain_unordered_list_continuation_indent() {
        let input = "- This is a long unordered list item that should wrap and the continuation line should be indented to match";
        let result = render_markdown_plain(input, 40);
        eprintln!("Unordered list continuation:\n{}", result);
        let lines: Vec<&str> = result.lines().collect();
        assert!(lines.len() > 1, "Should wrap to multiple lines");
        // Continuation lines should be indented by 2 spaces (matching "- " width)
        for line in &lines[1..] {
            assert!(
                line.starts_with("  "),
                "Continuation should be indented 2 spaces: {:?}",
                line
            );
        }
    }
}
