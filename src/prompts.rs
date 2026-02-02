//! Interactive prompts for collecting workflow inputs.
//!
//! Generates prompts based on workflow input schemas, supporting:
//! - Choice inputs (dropdown selection)
//! - Boolean inputs (yes/no confirmation)
//! - String inputs (text entry with optional default)

use anyhow::{Context, Result};
use indexmap::IndexMap;
use inquire::{Confirm, Select, Text, validator::ValueRequiredValidator};

use crate::github::WorkflowInput;

// -----------------------------------------------------------------------------
// Prompt Helpers
// -----------------------------------------------------------------------------

/// Prompt for a choice input (dropdown selection).
fn prompt_choice(label: &str, options: &[String]) -> Result<String> {
    let prompt = format!("Select {label}:");
    Ok(Select::new(&prompt, options.to_vec()).prompt()?)
}

/// Prompt for a boolean input (yes/no).
fn prompt_boolean(label: &str, default: bool) -> Result<String> {
    Ok(Confirm::new(label)
        .with_default(default)
        .prompt()?
        .to_string())
}

/// Prompt for a text input with optional default.
fn prompt_text(label: &str, default: Option<&str>, required: bool) -> Result<String> {
    let prompt = format!("Enter {label}:");
    let mut text = Text::new(&prompt);
    if let Some(d) = default {
        text = text.with_default(d);
    }
    if required {
        text = text.with_validator(ValueRequiredValidator::default());
    }
    Ok(text.prompt()?)
}

/// Collect workflow inputs by prompting the user.
///
/// For each input in the schema:
/// - If a prefilled value exists in config, use it (no prompt)
/// - Otherwise, prompt based on the input type (choice/boolean/string)
///
/// Returns an ordered map of input name -> value.
pub fn collect_workflow_inputs(
    inputs: &IndexMap<String, WorkflowInput>,
    prefilled: Option<&IndexMap<String, String>>,
) -> Result<IndexMap<String, String>> {
    let mut results = IndexMap::new();

    for (name, input) in inputs {
        // Use prefilled value if available
        if let Some(prefilled_values) = prefilled
            && let Some(value) = prefilled_values.get(name)
        {
            results.insert(name.clone(), value.clone());
            continue;
        }

        // Prompt user based on input type
        let label = input.description.as_deref().unwrap_or(name);
        let value = match input.input_type.as_deref() {
            Some("choice") => {
                let options = input
                    .options
                    .as_ref()
                    .context(format!("Choice input '{name}' has no options"))?;
                prompt_choice(label, options)?
            }
            Some("boolean") => {
                let default = input.default.as_deref() == Some("true");
                prompt_boolean(label, default)?
            }
            _ => {
                let default = input.default.as_deref();
                let required = input.required.unwrap_or(false);
                prompt_text(label, default, required)?
            }
        };

        results.insert(name.clone(), value);
    }

    Ok(results)
}
