//! Attune CLI. Test harness for the `attune-core` library during
//! development. `main` is a thin dispatch table; each subcommand lives
//! in its own module under [`commands`].

mod cli;
mod commands;
mod tracing_init;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Command};

fn main() -> Result<()> {
    tracing_init::init_tracing();
    let parsed = Cli::parse();
    match parsed.command {
        Command::Record(args) => commands::record::run(args),
        Command::Devices => commands::devices::run(),
        Command::Transcribe(args) => commands::transcribe::run(args),
        #[cfg(target_os = "macos")]
        Command::VpioSmoke(args) => commands::vpio_smoke::run(args),
        Command::Sessions(args) => commands::sessions::run(args),
        Command::Tasks(args) => commands::tasks::run(args),
        Command::MemorySearch(args) => commands::memory_search::run(args),
    }
}
