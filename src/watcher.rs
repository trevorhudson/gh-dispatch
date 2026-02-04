//! Rich workflow run watching with per-job progress display.
//!
//! Polls a workflow run and renders each job as a live spinner inside an
//! `indicatif::MultiProgress` group.  Completed steps are printed once as
//! they finish.  The loop exits when the run reaches "completed" status.

use anyhow::{Result, bail};
use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use octocrab::Octocrab;
use octocrab::models::workflows::Run;
use std::collections::HashMap;
use std::time::Duration;

use crate::github::{get_run_jobs, Job};

const POLL_INTERVAL: u64 = 5; // seconds
const MAX_WAIT: u64 = 30 * 60; // 30 minutes
const TICK_INTERVAL: u64 = 80; // milliseconds

/// Watch a workflow run, rendering job/step progress until completion.
pub async fn watch_run(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    run_id: u64,
) -> Result<Run> {
    let multi = MultiProgress::new();
    // Per-job state: the progress bar and the last step number we already printed.
    let mut job_bars: HashMap<u64, (ProgressBar, u32)> = HashMap::new();
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > Duration::from_secs(MAX_WAIT) {
            bail!("Timeout waiting for workflow completion (30 minutes)");
        }

        let run = client
            .workflows(owner, repo)
            .get(run_id.into())
            .await?;

        let jobs = get_run_jobs(client, owner, repo, run_id).await?;

        for job in &jobs {
            let (bar, last_step) = job_bars.entry(job.id).or_insert_with(|| {
                let b = multi.add(ProgressBar::new_spinner());
                b.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.cyan} {msg}")
                        .unwrap(),
                );
                b.enable_steady_tick(Duration::from_millis(TICK_INTERVAL));
                (b, 0)
            });

            // Print any newly-completed steps (only once each).
            if let Some(steps) = &job.steps {
                let new_steps: Vec<_> = steps
                    .iter()
                    .filter(|s| s.number > *last_step && s.status == "completed")
                    .collect();
                for step in new_steps {
                    let icon = match step.conclusion.as_deref() {
                        Some("success") => "  ✓".green().to_string(),
                        Some("failure") => "  ✗".red().to_string(),
                        Some("skipped") => "  ○".dimmed().to_string(),
                        _ => "  ?".dimmed().to_string(),
                    };
                    let _ = multi.println(format!("{} {}", icon, step.name));
                    *last_step = step.number;
                }
            }

            // Update the job's spinner message.
            bar.set_message(format_job_message(job));

            if job.status == "completed" {
                bar.finish();
            }
        }

        if run.status == "completed" {
            // Ensure all bars are finished (handles edge case where jobs
            // weren't fetched on the final tick).
            for (bar, _) in job_bars.values() {
                bar.finish();
            }
            return Ok(run);
        }

        tokio::time::sleep(Duration::from_secs(POLL_INTERVAL)).await;
    }
}

/// Build the display message for a single job spinner.
fn format_job_message(job: &Job) -> String {
    let icon = match (job.status.as_str(), job.conclusion.as_deref()) {
        ("completed", Some("success")) => "✓".green().bold().to_string(),
        ("completed", Some("failure")) => "✗".red().bold().to_string(),
        ("completed", Some("cancelled")) => "○".yellow().to_string(),
        ("completed", _) => "○".dimmed().to_string(),
        ("in_progress", _) => "●".cyan().to_string(),
        _ => "○".dimmed().to_string(), // queued / waiting / pending
    };

    let status_suffix = match job.status.as_str() {
        "queued" => " (queued)".dimmed().to_string(),
        "waiting" => " (waiting)".dimmed().to_string(),
        "in_progress" => {
            // Show the currently running step if available.
            job.steps
                .as_ref()
                .and_then(|steps| steps.iter().find(|s| s.status == "in_progress"))
                .map(|s| format!(" → {}", s.name.dimmed()))
                .unwrap_or_else(|| " (running)".dimmed().to_string())
        }
        "completed" => format_duration(job),
        _ => String::new(),
    };

    format!("{} {}{}", icon, job.name.bold(), status_suffix)
}

/// Format the duration a completed job took, or empty string if timestamps missing.
fn format_duration(job: &Job) -> String {
    match (&job.started_at, &job.completed_at) {
        (Some(start), Some(end)) => {
            let secs = (*end - *start).num_seconds().max(0);
            format!(" ({}:{:02})", secs / 60, secs % 60).dimmed().to_string()
        }
        _ => String::new(),
    }
}
