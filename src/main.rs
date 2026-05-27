mod cli;
mod config;
mod debug;
mod error;
mod history;
mod markdown;
mod opencode;
mod pager;
mod syntax;
mod tool_format;
mod tui;
mod update;

use clap::Parser;
use crate::cli::Args;
use crate::error::AppError;
use crate::opencode::Client;
use std::sync::Arc;

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), AppError> {
    let args = Args::parse();

    // Handle subcommands
    if let Some(command) = args.command {
        return match command {
            cli::Commands::Update => update::run(),
        };
    }

    let config = config::load_config().unwrap_or_default();
    let client = Arc::new(Client::new(&args.endpoint));
    client.probe_health()?;

    // Open pager first when a session-ID arg is given; then fall through to TUI.
    let pre_select_id: Option<String> = if let Some(ref raw) = args.session {
        let session_id = cli::parse_session_id(raw).map_err(AppError::Other)?;
        run_session_pager(&client, &args, &session_id)?;
        Some(session_id)
    } else {
        None
    };

    let rx = opencode::loader::load_sessions_streaming(Arc::clone(&client));
    let _ = tui::run_with_loader(
        rx,
        Arc::clone(&client),
        config,
        &args,
        pre_select_id.as_deref(),
    )?;
    Ok(())
}

fn run_session_pager(client: &Client, args: &cli::Args, session_id: &str) -> Result<(), AppError> {
    use crate::tui::ToolDisplayMode;
    let session = client.fetch_session_content(session_id)?;
    let tool_display = if args.no_tools {
        ToolDisplayMode::Hidden
    } else if args.show_tools {
        ToolDisplayMode::Full
    } else {
        ToolDisplayMode::Truncated
    };
    let options = tui::RenderOptions {
        content_width: 0,
        tool_display,
        show_thinking: args.show_thinking,
        show_timing: false,
    };
    let text = match tui::render_conversation(Some(&session), &options) {
        Ok(rendered) => rendered
            .lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|(t, style)| {
                        let needs = style.bold
                            || style.dimmed
                            || style.italic
                            || style.fg.is_some();
                        if !needs {
                            return t.clone();
                        }
                        let mut prefix = String::new();
                        if style.bold {
                            prefix.push_str("\x1b[1m");
                        }
                        if style.dimmed {
                            prefix.push_str("\x1b[2m");
                        }
                        if style.italic {
                            prefix.push_str("\x1b[3m");
                        }
                        if let Some((r, g, b)) = style.fg {
                            prefix.push_str(&format!("\x1b[38;2;{};{};{}m", r, g, b));
                        }
                        format!("{}{}\x1b[0m", prefix, t)
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Err(e) => return Err(AppError::Other(format!("render error: {e}"))),
    };
    pager::open_text_in_pager(&text)?;
    Ok(())
}
