//! CLI argument parsing and types.
//!
//! Defines the command-line interface using clap.

use clap::Parser;

// -----------------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------------

/// Command-line arguments.
#[derive(Parser)]
#[command(name = "gh-dispatch")]
#[command(about = "A CLI tool for triggering GitHub Actions workflows with polling support.")]
#[command(version)]
pub struct Args {
    /// Application name from config
    pub app: Option<String>,

    /// Workflow to run (e.g., build, deploy, test)
    #[arg(short, long)]
    pub workflow: Option<String>,

    /// Don't wait for workflow to complete
    #[arg(long)]
    pub no_wait: bool,
}
