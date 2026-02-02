//! Terminal UI helpers.
//!
//! Provides styled output functions for consistent CLI feedback:
//! spinners, success/info/warning messages.

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

// -----------------------------------------------------------------------------
// Output Helpers
// -----------------------------------------------------------------------------

const TICK_INTERVAL: u64 = 80; // milliseconds

/// Create a spinner with the given message.
pub fn create_spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(Duration::from_millis(TICK_INTERVAL));
    spinner
}

/// Print a success message with green checkmark.
pub fn success(msg: &str) {
    println!("{} {}", "✓".green().bold(), msg);
}

/// Print an info message with blue arrow.
pub fn info(msg: &str) {
    println!("{} {}", "→".blue().bold(), msg);
}

/// Print a warning message with yellow exclamation.
pub fn warning(msg: &str) {
    println!("{} {}", "!".yellow().bold(), msg);
}
