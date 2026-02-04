//! Rich workflow run watching with per-job progress display.
//!
//! Polls a workflow run and renders each job as a live spinner inside an
//! `indicatif::MultiProgress` group.  Completed steps are printed once as
//! they finish.  Annotations (notices, warnings, errors) are fetched and
//! displayed when each job completes.  The loop exits when the run reaches
//! "completed" status.

use anyhow::{Result, bail};
use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use octocrab::{Octocrab, models::workflows::Run, params::checks::CheckRunAnnotation};

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use crate::github::{
    Job, JobConclusion, JobStatus, check_run_id_from_url, get_annotations, get_run_jobs,
};

const POLL_INTERVAL: u64 = 5; // seconds
const MAX_WAIT: u64 = 30 * 60; // 30 minutes
const TICK_INTERVAL: u64 = 80; // milliseconds

/// Watch a workflow run, rendering job/step progress until completion.
pub async fn watch_run(client: &Octocrab, owner: &str, repo: &str, run_id: u64) -> Result<Run> {
    let multi = MultiProgress::new();
    // Per-job state: the progress bar and the last step number we already printed.
    let mut job_bars: HashMap<u64, (ProgressBar, u32)> = HashMap::new();
    // Jobs whose annotations we have already fetched and printed.
    let mut annotated: HashSet<u64> = HashSet::new();
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > Duration::from_secs(MAX_WAIT) {
            bail!("Timeout waiting for workflow completion (30 minutes)");
        }

        let run = client.workflows(owner, repo).get(run_id.into()).await?;

        let jobs = get_run_jobs(client, owner, repo, run_id.into()).await?;

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
            let new_steps: Vec<_> = job
                .steps
                .iter()
                .filter(|s| s.number > *last_step && s.status == JobStatus::Completed)
                .collect();
            for step in new_steps {
                let icon = match &step.conclusion {
                    Some(JobConclusion::Success) => "  ✓".green().to_string(),
                    Some(JobConclusion::Failure) => "  ✗".red().to_string(),
                    Some(JobConclusion::Skipped) => "  ○".dimmed().to_string(),
                    _ => "  ?".dimmed().to_string(),
                };
                let _ = multi.println(format!("{} {}", icon, step.name));
                *last_step = step.number;
            }

            // Update the job's spinner message.
            bar.set_message(format_job_message(job));

            if job.status == JobStatus::Completed {
                bar.finish();

                // Fetch and print annotations once per job.
                if let Some(check_run_id) = check_run_id_from_url(&job.check_run_url)
                    && annotated.insert(job.id)
                {
                    let annotations = get_annotations(client, owner, repo, check_run_id).await?;
                    for ann in &annotations {
                        let (prefix, msg) = format_annotation(ann);
                        let _ = multi.println(format!("{prefix} {msg}"));
                    }
                }
            }
        }

        if run.status == "completed" {
            // Ensure all bars are finished (handles edge case where jobs
            // weren't fetched on the final tick).
            for (bar, _) in job_bars.values() {
                bar.finish();
            }
            let _ = multi.println("");
            return Ok(run);
        }

        tokio::time::sleep(Duration::from_secs(POLL_INTERVAL)).await;
    }
}

/// Build the display message for a single job spinner.
fn format_job_message(job: &Job) -> String {
    let icon = match (&job.status, &job.conclusion) {
        (JobStatus::Completed, Some(JobConclusion::Success)) => "✓".green().bold().to_string(),
        (JobStatus::Completed, Some(JobConclusion::Failure)) => "✗".red().bold().to_string(),
        (JobStatus::Completed, Some(JobConclusion::Cancelled)) => "○".yellow().to_string(),
        (JobStatus::Completed, _) => "○".dimmed().to_string(),
        (JobStatus::InProgress, _) => "●".cyan().to_string(),
        _ => "○".dimmed().to_string(), // queued / waiting / pending
    };

    let status_suffix = match &job.status {
        JobStatus::Queued => " (queued)".dimmed().to_string(),
        JobStatus::Waiting => " (waiting)".dimmed().to_string(),
        JobStatus::InProgress => {
            // Show the currently running step if available.
            job.steps
                .iter()
                .find(|s| s.status == JobStatus::InProgress)
                .map_or_else(
                    || " (running)".dimmed().to_string(),
                    |s| format!(" → {}", s.name.dimmed()),
                )
        }
        JobStatus::Completed => format_duration(job),
        _ => String::new(),
    };

    format!("{} {}{}", icon, job.name.bold(), status_suffix)
}

/// Format a single annotation for terminal output.
///
/// Returns (colored prefix, message body).  The prefix reflects the annotation
/// level: notice (blue →), warning (yellow !), failure (red ✗).
fn format_annotation(ann: &CheckRunAnnotation) -> (String, String) {
    let level = ann.annotation_level.as_deref().unwrap_or("notice");
    let prefix = match level {
        "failure" => "    ✗".red().bold().to_string(),
        "warning" => "    !".yellow().bold().to_string(),
        _ => "    →".blue().bold().to_string(), // notice
    };

    let title = ann.title.as_deref().unwrap_or("");
    let message = ann.message.as_deref().unwrap_or("");
    let body = match (title.is_empty(), message.is_empty()) {
        (false, false) => format!("{}: {}", title.bold(), message),
        (false, true) => title.bold().to_string(),
        _ => message.to_string(),
    };

    (prefix, body)
}

/// Format the duration a completed job took, or empty string if timestamps missing.
fn format_duration(job: &Job) -> String {
    match (&job.started_at, &job.completed_at) {
        (Some(start), Some(end)) => {
            let secs = (*end - *start).num_seconds().max(0);
            format!(" ({}:{:02})", secs / 60, secs % 60)
                .dimmed()
                .to_string()
        }
        _ => String::new(),
    }
}
