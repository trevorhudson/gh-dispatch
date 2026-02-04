//! GitHub API interactions via octocrab.
//!
//! Provides functions for:
//! - Creating authenticated GitHub clients
//! - Fetching workflow schemas
//! - Dispatching workflows
//! - Polling workflow run status

use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use octocrab::Octocrab;
use octocrab::models::workflows::Run;
use serde::Deserialize;
use serde_yaml::Value;
use std::time::Duration;

const POLL_DELAY: u64 = 2;

// -----------------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------------

/// Workflow metadata and inputs parsed from a workflow file.
pub struct WorkflowSchema {
    /// Display name of the workflow
    pub name: String,
    /// Input definitions from `workflow_dispatch` trigger
    pub inputs: IndexMap<String, WorkflowInput>,
}

/// A single workflow input definition from `workflow_dispatch.inputs`.
#[derive(Debug, Deserialize, Clone)]
pub struct WorkflowInput {
    /// Default value if not provided
    pub default: Option<String>,
    /// Description shown in GitHub UI
    pub description: Option<String>,
    /// Input type: "string", "boolean", or "choice"
    #[serde(rename = "type")]
    pub input_type: Option<String>,
    /// Available options (only for choice type)
    pub options: Option<Vec<String>>,
    /// Whether the input is required
    pub required: Option<bool>,
}

// -----------------------------------------------------------------------------
// Job / Step Types
// -----------------------------------------------------------------------------

/// Response from GET /repos/{owner}/{repo}/actions/runs/{run_id}/jobs
#[derive(Debug, Deserialize)]
pub struct JobsResponse {
    pub jobs: Vec<Job>,
}

/// A single job within a workflow run.
#[derive(Debug, Deserialize, Clone)]
pub struct Job {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub steps: Option<Vec<Step>>,
}

/// A single step within a job.
#[derive(Debug, Deserialize, Clone)]
pub struct Step {
    pub name: String,
    pub number: u32,
    pub status: String,
    pub conclusion: Option<String>,
}

// -----------------------------------------------------------------------------
// Client
// -----------------------------------------------------------------------------

/// Create an authenticated octocrab client.
///
/// Attempts to get a token from:
/// 1. `GITHUB_TOKEN` environment variable
/// 2. `gh auth token` CLI command (if gh is installed and authenticated)
pub fn create_client() -> Result<Octocrab> {
    let token = get_token()?;
    Octocrab::builder()
        .personal_token(token)
        .build()
        .context("Failed to create GitHub client")
}

/// Get GitHub token from environment or gh CLI.
fn get_token() -> Result<String> {
    // Try environment variable first
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        return Ok(token);
    }

    // Fall back to gh CLI
    let output = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .context("Failed to run `gh auth token`")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        bail!("No GITHUB_TOKEN found and `gh auth token` failed")
    }
}

// -----------------------------------------------------------------------------
// Repository Info
// -----------------------------------------------------------------------------

/// Get the default branch for a repository.
pub async fn get_default_branch(client: &Octocrab, owner: &str, repo: &str) -> Result<String> {
    let repository = client
        .repos(owner, repo)
        .get()
        .await
        .context("Failed to fetch repository")?;

    repository
        .default_branch
        .context("Repository has no default branch")
}

// -----------------------------------------------------------------------------
// Workflow Schema
// -----------------------------------------------------------------------------

/// Fetch and parse a workflow file to extract its input schema.
///
/// Retrieves the workflow YAML from GitHub and parses the `workflow_dispatch.inputs`
/// section to determine what inputs the workflow accepts.
pub async fn get_workflow_schema(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    workflow: &str,
) -> Result<WorkflowSchema> {
    let path = format!(".github/workflows/{workflow}");

    let content = client
        .repos(owner, repo)
        .get_content()
        .path(&path)
        .send()
        .await
        .context("Failed to fetch workflow file")?;

    let file = content
        .items
        .into_iter()
        .next()
        .context("No content returned")?;

    let encoded = file.content.context("Workflow file has no content")?;

    // GitHub returns base64-encoded content with newlines
    let cleaned: String = encoded.chars().filter(|c| !c.is_whitespace()).collect();
    let decoded = general_purpose::STANDARD
        .decode(&cleaned)
        .context("Failed to decode base64")?;
    let yaml_content = String::from_utf8(decoded).context("Workflow is not valid UTF-8")?;

    parse_workflow_schema(&yaml_content)
}

/// Parse workflow YAML and extract the `workflow_dispatch` inputs section.
fn parse_workflow_schema(yaml_content: &str) -> Result<WorkflowSchema> {
    let yaml: Value =
        serde_yaml::from_str(yaml_content).context("Failed to parse workflow YAML")?;

    let name = yaml
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("Unnamed workflow")
        .to_string();

    let inputs_value = yaml
        .get("on")
        .and_then(|on| on.get("workflow_dispatch"))
        .and_then(|wd| wd.get("inputs"));

    let inputs: IndexMap<String, WorkflowInput> = match inputs_value {
        Some(v) => serde_yaml::from_value(v.clone()).context("Failed to parse inputs")?,
        None => IndexMap::new(),
    };

    Ok(WorkflowSchema { name, inputs })
}

// -----------------------------------------------------------------------------
// Workflow Dispatch
// -----------------------------------------------------------------------------

/// Dispatch a workflow with the given inputs.
///
/// Note: The GitHub API returns 204 No Content on success - no run ID is returned.
/// Use `get_latest_run` to find the triggered run.
pub async fn dispatch_workflow(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    workflow: &str,
    git_ref: &str,
    inputs: serde_json::Value,
) -> Result<()> {
    client
        .actions()
        .create_workflow_dispatch(owner, repo, workflow, git_ref)
        .inputs(inputs)
        .send()
        .await
        .with_context(|| format!("Failed to dispatch workflow: {workflow}"))?;

    Ok(())
}

// -----------------------------------------------------------------------------
// Workflow Run Polling
// -----------------------------------------------------------------------------

/// Find the most recent workflow run after dispatch.
///
/// Waits briefly then queries for the latest `workflow_dispatch` run on the branch.
/// This is best-effort - in high-traffic repos, you may get someone else's run.
pub async fn get_latest_run(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    workflow: &str,
    git_ref: &str,
) -> Result<Run> {
    // Brief delay to let GitHub register the run
    tokio::time::sleep(Duration::from_secs(POLL_DELAY)).await;

    let runs = client
        .workflows(owner, repo)
        .list_runs(workflow)
        .branch(git_ref)
        .event("workflow_dispatch")
        .per_page(1)
        .send()
        .await
        .context("Failed to list workflow runs")?;

    runs.items
        .into_iter()
        .next()
        .context("No workflow runs found")
}

/// Fetch jobs for a workflow run.
///
/// octocrab does not expose a jobs endpoint natively, so this uses a raw GET.
pub async fn get_run_jobs(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    run_id: u64,
) -> Result<Vec<Job>> {
    let route = format!("/repos/{owner}/{repo}/actions/runs/{run_id}/jobs");
    let response: JobsResponse = client
        .get(&route, None::<&()>)
        .await
        .context("Failed to fetch jobs")?;
    Ok(response.jobs)
}
