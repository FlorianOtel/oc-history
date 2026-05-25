mod app;
mod export;
pub mod search;
pub mod theme;
mod ui;
mod viewer;

pub use app::{Action, run_single_file, run_with_loader, RenderedLine};
pub use export::{ExportFormat, copy_to_system_clipboard, sanitize_filename, render_oc_export};

// Stub declarations for v0 (export and viewer are v1 cleanup)
pub struct RenderOptions {
    pub tool_display: ToolDisplayMode,
    pub show_thinking: bool,
    pub show_timing: bool,
    pub content_width: usize,
}

pub struct RenderedConversation {
    pub lines: Vec<app::RenderedLine>,
    pub messages: Vec<MessageRange>,
}

pub const GUTTER_WIDTH: usize = 4; // Stub constant

pub fn render_conversation(
    content: Option<&crate::opencode::models::OcSessionView>,
    options: &RenderOptions,
) -> crate::error::Result<RenderedConversation> {
    viewer::render_oc_session(content, options)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolDisplayMode {
    Hidden,
    Truncated,
    Full,
}
impl Default for ToolDisplayMode {
    fn default() -> Self {
        ToolDisplayMode::Truncated
    }
}
impl ToolDisplayMode {
    pub fn is_visible(&self) -> bool {
        !matches!(self, ToolDisplayMode::Hidden)
    }
    /// Cycle to the next tool display mode
    pub fn next(self) -> Self {
        match self {
            ToolDisplayMode::Hidden => ToolDisplayMode::Truncated,
            ToolDisplayMode::Truncated => ToolDisplayMode::Full,
            ToolDisplayMode::Full => ToolDisplayMode::Hidden,
        }
    }
    /// Get a status label for the current mode
    pub fn status_label(&self) -> &'static str {
        match self {
            ToolDisplayMode::Hidden => "tools:off",
            ToolDisplayMode::Truncated => "tools:short",
            ToolDisplayMode::Full => "tools:full",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MessageRange {
    pub start_line: usize,
    pub end_line: usize,
    pub entry_index: usize,
}

