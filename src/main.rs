mod cli;
mod config;
mod github;
mod prompts;
mod ui;

use anyhow::{Result, bail};
use clap::Parser;
use cli::{Args, Workflow};
use colored::Colorize;
use config::load_config;
use github::{
    create_client, dispatch_workflow, get_default_branch, get_latest_run, get_workflow_schema,
    wait_for_completion,
};
use inquire::{Confirm, Select};
use prompts::collect_workflow_inputs;
use ui::{create_spinner, info, success, warning};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Args::parse();
    let config = load_config()?;
    let client = create_client()?;

    // Get app from arg or prompt
    let selected_app = if let Some(app) = &cli.app {
        if !config.apps.contains_key(app) {
            bail!("App '{}' not found in config", app);
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
        *wf
    } else {
        Workflow::select("Select workflow:").prompt()?
    };

    let workflow_ref = match selected_workflow {
        Workflow::Build => &app.build,
        Workflow::Deploy => &app.deploy,
    };

    let owner = &workflow_ref.owner;
    let repo = &workflow_ref.repo;

    // Fetch workflow schema and default branch
    let spinner = create_spinner("Fetching workflow...");
    let schema = get_workflow_schema(&client, owner, repo, &workflow_ref.workflow).await?;
    let git_ref = get_default_branch(&client, owner, repo).await?;
    spinner.finish_and_clear();
    info(&format!(
        "Workflow: '{}' ({})",
        schema.name.cyan(),
        git_ref.dimmed()
    ));

    // Collect inputs (prefilled from config, prompt for missing)
    let inputs = collect_workflow_inputs(&schema.inputs, workflow_ref.inputs.as_ref())?;

    println!(
        "\n{}ing {} with inputs:",
        selected_workflow.to_string().bold(),
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
    if !cli.no_wait {
        success("Workflow dispatched");
        let spinner = create_spinner("Finding workflow run...");
        let run = get_latest_run(&client, owner, repo, &workflow_ref.workflow, &git_ref).await?;
        spinner.finish_and_clear();

        info(&format!("Run #{}", run.run_number.to_string().cyan()));
        println!("  {}", run.html_url.to_string().underline().blue());

        let spinner = create_spinner("Waiting for completion...");
        let completed = wait_for_completion(&client, owner, repo, run.id.into_inner()).await?;
        spinner.finish_and_clear();

        let conclusion = completed.conclusion.as_deref().unwrap_or("unknown");
        match conclusion {
            "success" => success("Workflow completed successfully"),
            "failure" => {
                bail!("Workflow failed");
            }
            "cancelled" => warning("Workflow was cancelled"),
            other => info(&format!("Workflow finished: {}", other)),
        }
    } else {
        success("Workflow dispatched (not waiting for completion)");
    }

    Ok(())
}
