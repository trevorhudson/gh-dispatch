mod cli;
mod config;
mod github;
mod prompts;
mod ui;
mod watcher;

use anyhow::{Result, bail};
use clap::Parser;
use cli::Args;
use colored::Colorize;
use config::load_config;
use github::{
    create_client, dispatch_workflow, get_default_branch, get_latest_run, get_workflow_schema,
};
use inquire::{Confirm, Select};
use prompts::collect_workflow_inputs;
use ui::{create_spinner, info, success, warning};
use watcher::watch_run;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Args::parse();
    let config = load_config()?;
    let client = create_client()?;

    // Get app from arg or prompt
    let selected_app = if let Some(app) = &cli.app {
        if !config.apps.contains_key(app) {
            bail!("App '{app}' not found in config");
        }
        app.as_str()
    } else {
        let mut app_names: Vec<&String> = config.apps.keys().collect();
        app_names.sort();
        Select::new("Select application:", app_names)
            .with_help_message("Application to build/deploy")
            .prompt()?
    };

    let app = &config.apps[selected_app];

    // Get workflow from arg or prompt
    let selected_workflow = if let Some(wf) = &cli.workflow {
        if !app.contains_key(wf) {
            bail!("Workflow '{wf}' not found for app '{selected_app}'");
        }
        wf.clone()
    } else {
        let workflow_names: Vec<&String> = app.keys().collect();
        Select::new("Select workflow:", workflow_names)
            .prompt()?
            .clone()
    };

    let workflow_ref = &app[&selected_workflow];

    let owner = &workflow_ref.owner;
    let repo = &workflow_ref.repo;

    // Fetch workflow schema; resolve git ref from config or default branch
    let spinner = create_spinner("Fetching workflow...");
    let schema = get_workflow_schema(&client, owner, repo, &workflow_ref.workflow).await?;
    let git_ref = match &workflow_ref.git_ref {
        Some(r) => r.clone(),
        None => get_default_branch(&client, owner, repo).await?,
    };
    spinner.finish_and_clear();
    info(&format!(
        "Workflow: '{}' ({})",
        schema.name.cyan(),
        git_ref.dimmed()
    ));

    // Collect inputs (prefilled from config, prompt for missing)
    let inputs = collect_workflow_inputs(&schema.inputs, workflow_ref.inputs.as_ref())?;

    println!(
        "\nRunning '{}' for {} with inputs:",
        selected_workflow.bold(),
        selected_app.cyan().bold()
    );
    for (key, value) in &inputs {
        println!("  {} = {}", key.dimmed(), value.yellow());
    }
    println!();

    if !Confirm::new("Continue?").with_default(true).prompt()? {
        warning("Aborted");
        return Ok(());
    }

    // Dispatch workflow
    let spinner = create_spinner("Dispatching workflow...");
    let inputs_json = serde_json::to_value(&inputs)?;
    dispatch_workflow(
        &client,
        owner,
        repo,
        &workflow_ref.workflow,
        &git_ref,
        inputs_json,
    )
    .await?;
    spinner.finish_and_clear();

    // Wait for completion if requested
    if cli.no_wait {
        success("Workflow dispatched (not waiting for completion)");
    } else {
        success("Workflow dispatched");
        let spinner = create_spinner("Finding workflow run...");
        let run = get_latest_run(&client, owner, repo, &workflow_ref.workflow, &git_ref).await?;
        spinner.finish_and_clear();

        info(&format!("Run #{}", run.run_number.to_string().cyan()));
        println!("  {}", run.html_url.to_string().underline().blue());
        println!();

        let completed = watch_run(&client, owner, repo, run.id.into_inner()).await?;

        let conclusion = completed.conclusion.as_deref().unwrap_or("unknown");
        match conclusion {
            "success" => success("Workflow completed successfully"),
            "failure" => {
                bail!("Workflow failed");
            }
            "cancelled" => warning("Workflow was cancelled"),
            other => info(&format!("Workflow finished: {other}")),
        }
    }

    Ok(())
}
