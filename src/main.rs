mod claude;
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

    // Load any config the existing infrastructure expects (keep the call; ignore if unused).
    let config = config::load_config().unwrap_or_default();

    let client = Arc::new(Client::new(&args.endpoint));
    client.probe_health()?;

    let rx = opencode::loader::load_sessions_streaming(Arc::clone(&client));

    // Keep the existing `tui::run_with_loader` call shape, but add the client.
    let _ = tui::run_with_loader(
        rx,
        Arc::clone(&client),
        config,
        &args,
    )?;

    Ok(())
}
