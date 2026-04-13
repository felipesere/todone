use dialoguer::{theme::ColorfulTheme, Input, Select};

/// Interactively prompt for todo text and a project selection.
/// `existing_projects` is the list of sections already in today's file.
/// Returns `(text, project)`.
pub fn prompt_todo(existing_projects: &[&str]) -> anyhow::Result<(String, String)> {
    let theme = ColorfulTheme::default();

    let text: String = Input::with_theme(&theme)
        .with_prompt("Todo")
        .interact_text()?;

    let project = prompt_project_with_theme(&theme, existing_projects)?;

    Ok((text, project))
}

/// Interactively prompt for a project only (when text was already supplied).
pub fn prompt_project(existing_projects: &[&str]) -> anyhow::Result<String> {
    prompt_project_with_theme(&ColorfulTheme::default(), existing_projects)
}

const NEW_PROJECT: &str = "+ new project";

fn prompt_project_with_theme(
    theme: &ColorfulTheme,
    existing_projects: &[&str],
) -> anyhow::Result<String> {
    let mut projects: Vec<&str> = existing_projects.to_vec();
    if !projects.contains(&"inbox") {
        projects.push("inbox");
    }
    projects.push(NEW_PROJECT);

    let default_idx = projects.iter().position(|p| *p == "inbox").unwrap_or(0);

    let idx = Select::with_theme(theme)
        .with_prompt("Project")
        .items(&projects)
        .default(default_idx)
        .interact()?;

    if projects[idx] == NEW_PROJECT {
        let name: String = Input::with_theme(theme)
            .with_prompt("New project name")
            .interact_text()?;
        Ok(name)
    } else {
        Ok(projects[idx].to_string())
    }
}
