//! CLI argument parsing and types.
//!
//! Defines the command-line interface using clap, including
//! the main `Args` struct and `Workflow` enum.

use clap::{Parser, ValueEnum};
use inquire_derive::Selectable;
use std::fmt::{Display, Formatter};

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

    /// Workflow to run
    #[arg(short, long)]
    pub workflow: Option<Workflow>,

    /// Don't wait for workflow to complete
    #[arg(long)]
    pub no_wait: bool,
}

/// Workflow type to dispatch.
#[derive(Debug, Copy, Clone, Selectable, ValueEnum)]
pub enum Workflow {
    Build,
    Deploy,
}

impl Display for Workflow {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Workflow::Build => write!(f, "Build"),
            Workflow::Deploy => write!(f, "Deploy"),
        }
    }
}
