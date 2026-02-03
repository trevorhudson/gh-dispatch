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

/// Spawn a task that updates the spinner message each second with elapsed time.
///
/// Returns a `JoinHandle` — call `.abort()` on it when done.
pub fn start_timer(spinner: &ProgressBar, prefix: &str) -> tokio::task::JoinHandle<()> {
    let spinner = spinner.clone();
    let prefix = prefix.to_string();
    tokio::spawn(async move {
        let start = std::time::Instant::now();
        loop {
            let secs = start.elapsed().as_secs();
            spinner.set_message(format!("{prefix} ({}:{:02})", secs / 60, secs % 60));
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
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
