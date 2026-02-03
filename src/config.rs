//! Configuration loading and types.
//!
//! Loads config from `./config.toml` or `~/.config/gh-dispatch/config.toml`.
//!
//! # Example config.toml
//!
//! ```toml
//! [apps.my-app]
//! build = { repo = "owner/repo", workflow = "build.yml", inputs = { app = "my-app" } }
//! deploy = { repo = "owner/repo", workflow = "deploy.yml" }
//! ```

use anyhow::{Context, Result, bail};
use indexmap::IndexMap;
use serde::Deserialize;
use std::{fs::read_to_string, path::PathBuf};

// -----------------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------------

/// Top-level config structure.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Map of application name to its configuration
    pub apps: IndexMap<String, AppConfig>,
}

/// Configuration for a single application.
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    /// Build workflow reference
    pub build: WorkflowRef,
    /// Deploy workflow reference
    pub deploy: WorkflowRef,
}

/// Reference to a GitHub Actions workflow.
#[derive(Debug, Deserialize)]
#[serde(try_from = "WorkflowRefRaw")]
pub struct WorkflowRef {
    /// Repository owner
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Workflow filename (e.g., "build.yml")
    pub workflow: String,
    /// Optional pre-filled input values (skip prompts for these)
    pub inputs: Option<IndexMap<String, String>>,
}

/// Raw deserialization struct for WorkflowRef.
#[derive(Deserialize)]
struct WorkflowRefRaw {
    repo: String,
    workflow: String,
    #[serde(default)]
    inputs: Option<IndexMap<String, String>>,
}

impl TryFrom<WorkflowRefRaw> for WorkflowRef {
    type Error = String;

    fn try_from(raw: WorkflowRefRaw) -> Result<Self, Self::Error> {
        let (owner, repo) = raw
            .repo
            .split_once('/')
            .map(|(o, r)| (o.to_string(), r.to_string()))
            .ok_or_else(|| format!("Invalid repo format '{}', expected 'owner/repo'", raw.repo))?;

        Ok(WorkflowRef {
            owner,
            repo,
            workflow: raw.workflow,
            inputs: raw.inputs,
        })
    }
}

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

/// Load configuration from disk.
///
/// Searches for config in order:
/// 1. `./config.toml` (current directory)
/// 2. `~/.config/gh-dispatch/config.toml` (user config)
pub fn load_config() -> Result<Config> {
    let local = PathBuf::from("./config.toml");
    let home_config = {
        let home = std::env::var_os("HOME").context("HOME not set")?;
        PathBuf::from(home)
            .join(".config")
            .join("gh-dispatch")
            .join("config.toml")
    };

    let config_path = if local.exists() {
        local
    } else if home_config.exists() {
        home_config
    } else {
        bail!(
            "No config file found. Checked:\n  {}\n  {}",
            local.display(),
            home_config.display()
        )
    };

    let content = read_to_string(&config_path)
        .with_context(|| format!("Failed to read {:?}", config_path))?;

    toml::from_str(&content).context("Failed to parse config TOML")
}
