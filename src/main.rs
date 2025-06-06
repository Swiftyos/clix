use clap::{CommandFactory, Parser};
use clap_complete::{Shell as CompletionShell, generate};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};

use clix::ai::{ConversationSession, ConversationState, MessageRole};
use clix::cli::app::{CliArgs, Commands, GitCommands, SettingsCommands, Shell};
use clix::commands::{
    Command, CommandExecutor, Workflow, WorkflowStep, WorkflowVariable, WorkflowVariableProfile,
};
use clix::error::{ClixError, Result};
use clix::share::{ExportManager, ImportManager};
use clix::storage::{ConversationStorage, GitIntegratedStorage};
use clix::{ClaudeAssistant, SettingsManager};

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e.to_user_friendly_message());

        // Show suggestions if available
        let suggestions = e.get_suggestions();
        if !suggestions.is_empty() {
            eprintln!("\n{}", "Suggestions:".yellow().bold());
            for suggestion in suggestions {
                eprintln!("  • {}", suggestion);
            }
        }

        exit(1);
    }
}

fn run() -> Result<()> {
    let args = CliArgs::parse();
    let mut storage = GitIntegratedStorage::new()?;

    // Sync with git repositories at startup
    if let Err(e) = storage.sync_with_repositories() {
        eprintln!("Warning: Failed to sync with git repositories: {}", e);
    }

    match args.command {
        Commands::Add(add_args) => {
            let tags = add_args.tags.unwrap_or_else(Vec::new);

            let command = if let Some(command_str) = add_args.command {
                // Simple command
                Command::new(add_args.name, add_args.description, command_str, tags)
            } else if let Some(steps_file) = add_args.steps_file {
                // Workflow from steps file
                let steps_json = fs::read_to_string(&steps_file).map_err(ClixError::Io)?;
                let steps: Vec<WorkflowStep> =
                    serde_json::from_str(&steps_json).map_err(ClixError::Serialization)?;
                Command::new_workflow(add_args.name, add_args.description, steps, tags)
            } else {
                return Err(ClixError::InvalidCommandFormat(
                    "Either --command or --steps-file must be provided".to_string(),
                ));
            };

            storage.add_command(command)?;
            println!("{} Command added successfully", "Success:".green().bold());
        }

        Commands::Run(run_args) => {
            let command = storage.get_command(&run_args.name)?;

            if command.is_workflow() {
                // Handle workflow execution
                let vars = if let Some(var_args) = &run_args.var {
                    let mut vars_map = HashMap::new();
                    for var_str in var_args {
                        if let Some((key, value)) = var_str.split_once('=') {
                            vars_map.insert(key.to_string(), value.to_string());
                        } else {
                            return Err(ClixError::InvalidCommandFormat(format!(
                                "Invalid variable format: {}, expected key=value",
                                var_str
                            )));
                        }
                    }
                    Some(vars_map)
                } else {
                    None
                };

                // Create a temporary workflow for execution
                let mut workflow = Workflow::new(
                    command.name.clone(),
                    command.description.clone(),
                    command.steps.clone().unwrap_or_default(),
                    command.tags.clone(),
                );

                // Add variables and profiles from the command
                workflow.variables = command.variables.clone();
                workflow.profiles = command.profiles.clone();

                let results = CommandExecutor::execute_workflow(
                    &workflow,
                    run_args.profile.as_deref(),
                    vars,
                )?;

                // Print all results
                println!("\n{}", "Workflow Results:".blue().bold());
                println!("{}", "=".repeat(50));

                for (name, result) in results {
                    println!("{}: {}", "Step".green().bold(), name);

                    match result {
                        Ok(output) => CommandExecutor::print_command_output(&output),
                        Err(e) => println!("{} {}", "Error:".red().bold(), e),
                    }

                    println!("{}", "-".repeat(50));
                }
            } else {
                // Handle simple command execution
                let output = CommandExecutor::execute_command(&command)?;
                CommandExecutor::print_command_output(&output);
            }

            // Update usage statistics
            storage.update_command_usage(&run_args.name)?;
        }

        Commands::List(list_args) => {
            let all_commands = storage.list_commands()?;
            // Get old workflows for backward compatibility during migration
            let old_workflows = storage.list_workflows()?;

            if all_commands.is_empty() && old_workflows.is_empty() {
                println!("No commands or workflows stored yet.");
                return Ok(());
            }

            // Separate simple commands from workflows in the unified structure
            let (simple_commands, workflow_commands): (Vec<_>, Vec<_>) =
                all_commands.into_iter().partition(|cmd| !cmd.is_workflow());

            // Skip workflows if commands_only is set
            let show_workflows = !list_args.commands_only;
            // Skip commands if workflows_only is set
            let show_commands = !list_args.workflows_only;

            // Filter by tag if provided
            let filtered_simple_commands: Vec<_> = if let Some(ref tag) = list_args.tag {
                simple_commands
                    .into_iter()
                    .filter(|cmd| cmd.tags.contains(tag))
                    .collect()
            } else {
                simple_commands
            };

            let filtered_workflow_commands: Vec<_> = if let Some(ref tag) = list_args.tag {
                workflow_commands
                    .into_iter()
                    .filter(|cmd| cmd.tags.contains(tag))
                    .collect()
            } else {
                workflow_commands
            };

            let filtered_old_workflows: Vec<_> = if let Some(ref tag) = list_args.tag {
                old_workflows
                    .into_iter()
                    .filter(|wf| wf.tags.contains(tag))
                    .collect()
            } else {
                old_workflows
            };

            // Print simple commands
            if show_commands && !filtered_simple_commands.is_empty() {
                println!("\n{}", "Commands:".blue().bold());
                println!("{}", "=".repeat(50));

                for cmd in &filtered_simple_commands {
                    println!("{}: {}", "Name".green().bold(), cmd.name);
                    println!("{}: {}", "Description".green(), cmd.description);
                    println!(
                        "{}: {}",
                        "Command".green(),
                        cmd.command.as_ref().unwrap_or(&"<no command>".to_string())
                    );

                    if !cmd.tags.is_empty() {
                        println!("{}: {}", "Tags".green(), cmd.tags.join(", "));
                    }

                    if let Some(last_used) = cmd.last_used {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        let days_ago = (now - last_used) / (60 * 60 * 24);

                        println!(
                            "{}: {} ({} days ago)",
                            "Last used".green(),
                            cmd.use_count,
                            days_ago
                        );
                    }

                    println!("{}", "-".repeat(50));
                }
            }

            // Print workflow commands (from unified structure)
            if show_workflows && !filtered_workflow_commands.is_empty() {
                println!("\n{}", "Workflows:".blue().bold());
                println!("{}", "=".repeat(50));

                for cmd in &filtered_workflow_commands {
                    println!("{}: {}", "Name".green().bold(), cmd.name);
                    println!("{}: {}", "Description".green(), cmd.description);
                    println!(
                        "{}: {}",
                        "Steps".green(),
                        cmd.steps.as_ref().map_or(0, |s| s.len())
                    );

                    if !cmd.tags.is_empty() {
                        println!("{}: {}", "Tags".green(), cmd.tags.join(", "));
                    }

                    if let Some(last_used) = cmd.last_used {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        let days_ago = (now - last_used) / (60 * 60 * 24);

                        println!(
                            "{}: {} ({} days ago)",
                            "Last used".green(),
                            cmd.use_count,
                            days_ago
                        );
                    }

                    println!("{}", "-".repeat(50));
                }
            }

            // Print old workflows (for backward compatibility during migration)
            if show_workflows && !filtered_old_workflows.is_empty() {
                if filtered_workflow_commands.is_empty() {
                    println!("\n{}", "Workflows (legacy):".blue().bold());
                } else {
                    println!("\n{}", "Legacy Workflows:".blue().bold());
                }
                println!("{}", "=".repeat(50));

                for wf in filtered_old_workflows {
                    println!("{}: {}", "Name".green().bold(), wf.name);
                    println!("{}: {}", "Description".green(), wf.description);
                    println!("{}: {}", "Steps".green(), wf.steps.len());

                    if !wf.tags.is_empty() {
                        println!("{}: {}", "Tags".green(), wf.tags.join(", "));
                    }

                    if let Some(last_used) = wf.last_used {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        let days_ago = (now - last_used) / (60 * 60 * 24);

                        println!(
                            "{}: {} ({} days ago)",
                            "Last used".green(),
                            wf.use_count,
                            days_ago
                        );
                    }

                    println!("{}", "-".repeat(50));
                }
            }
        }

        Commands::Remove(remove_args) => {
            storage.remove_command(&remove_args.name)?;
            println!(
                "{} Command '{}' removed successfully",
                "Success:".green().bold(),
                remove_args.name
            );
        }

        Commands::AddVar(add_var_args) => {
            let mut command = storage.get_command(&add_var_args.command_name)?;

            if !command.is_workflow() {
                return Err(ClixError::InvalidCommandFormat(
                    "Variables can only be added to workflows".to_string(),
                ));
            }

            let variable = WorkflowVariable::new(
                add_var_args.name,
                add_var_args.description,
                add_var_args.default,
                add_var_args.required,
            );

            command.add_variable(variable);
            storage.update_command(&command)?;

            println!(
                "{} Variable added to workflow '{}'",
                "Success:".green().bold(),
                add_var_args.command_name
            );
        }

        Commands::AddProfile(add_profile_args) => {
            let mut command = storage.get_command(&add_profile_args.command_name)?;

            if !command.is_workflow() {
                return Err(ClixError::InvalidCommandFormat(
                    "Profiles can only be added to workflows".to_string(),
                ));
            }

            // Parse variable values
            let mut vars_map = HashMap::new();
            for var_str in &add_profile_args.var {
                if let Some((key, value)) = var_str.split_once('=') {
                    vars_map.insert(key.to_string(), value.to_string());
                } else {
                    return Err(ClixError::InvalidCommandFormat(format!(
                        "Invalid variable format: {}, expected key=value",
                        var_str
                    )));
                }
            }

            let profile = WorkflowVariableProfile::new(
                add_profile_args.name,
                add_profile_args.description,
                vars_map,
            );

            command.add_profile(profile);
            storage.update_command(&command)?;

            println!(
                "{} Profile added to workflow '{}'",
                "Success:".green().bold(),
                add_profile_args.command_name
            );
        }

        Commands::ListProfiles(list_profiles_args) => {
            let command = storage.get_command(&list_profiles_args.command_name)?;

            if !command.is_workflow() {
                return Err(ClixError::InvalidCommandFormat(
                    "Profiles can only be listed for workflows".to_string(),
                ));
            }

            if command.profiles.is_empty() {
                println!(
                    "No profiles defined for workflow '{}'.",
                    list_profiles_args.command_name
                );
                return Ok(());
            }

            println!("{}", "Workflow Profiles:".blue().bold());
            println!("{}", "=".repeat(50));

            for (name, profile) in &command.profiles {
                println!("{}: {}", "Profile".green().bold(), name);
                println!("{}: {}", "Description".green(), profile.description);
                println!("{}: {}", "Variables".green(), profile.variables.len());

                for (var_name, var_value) in &profile.variables {
                    println!("{}: {} = {}", "  Variable".yellow(), var_name, var_value);
                }

                println!("{}", "-".repeat(50));
            }
        }

        Commands::AddCondition(args) => {
            use clix::commands::models::{Condition, ConditionalAction, WorkflowStep};

            let mut command = storage.get_command(&args.command_name)?;

            if !command.is_workflow() {
                return Err(ClixError::InvalidCommandFormat(
                    "Conditions can only be added to workflows".to_string(),
                ));
            }

            // Read steps from JSON files
            let then_steps_json = fs::read_to_string(&args.then_file).map_err(ClixError::Io)?;
            let then_steps: Vec<WorkflowStep> =
                serde_json::from_str(&then_steps_json).map_err(ClixError::Serialization)?;

            let else_steps = if let Some(else_file) = &args.else_file {
                let else_steps_json = fs::read_to_string(else_file).map_err(ClixError::Io)?;
                let steps: Vec<WorkflowStep> =
                    serde_json::from_str(&else_steps_json).map_err(ClixError::Serialization)?;
                Some(steps)
            } else {
                None
            };

            // Parse action if provided
            let action = if let Some(action_str) = &args.action {
                match action_str.as_str() {
                    "run_then" => Some(ConditionalAction::RunThen),
                    "run_else" => Some(ConditionalAction::RunElse),
                    "continue" => Some(ConditionalAction::Continue),
                    "break" => Some(ConditionalAction::Break),
                    "return" => {
                        let return_code = args.return_code.unwrap_or(0);
                        Some(ConditionalAction::Return(return_code))
                    }
                    _ => {
                        return Err(ClixError::InvalidCommandFormat(format!(
                            "Invalid action '{}'. Valid actions: run_then, run_else, continue, break, return",
                            action_str
                        )));
                    }
                }
            } else {
                None
            };

            // Create condition
            let condition = Condition {
                expression: args.condition.clone(),
                variable: args.variable.clone(),
            };

            // Create conditional step
            let conditional_step = WorkflowStep::new_conditional(
                args.name.clone(),
                args.description.clone(),
                condition,
                then_steps,
                else_steps,
                action,
            );

            // Add the conditional step to the workflow
            if let Some(ref mut steps) = command.steps {
                steps.push(conditional_step);
            }
            storage.update_command(&command)?;

            println!(
                "{} Conditional step '{}' added to workflow '{}'",
                "Success:".green().bold(),
                args.name,
                args.command_name
            );
        }

        Commands::AddBranch(args) => {
            use clix::commands::models::{BranchCase, WorkflowStep};

            let mut command = storage.get_command(&args.command_name)?;

            if !command.is_workflow() {
                return Err(ClixError::InvalidCommandFormat(
                    "Branches can only be added to workflows".to_string(),
                ));
            }

            // Read cases from JSON file
            let cases_json = fs::read_to_string(&args.cases_file).map_err(ClixError::Io)?;
            let cases: Vec<BranchCase> =
                serde_json::from_str(&cases_json).map_err(ClixError::Serialization)?;

            // Read default case if provided
            let default_case = if let Some(default_file) = &args.default_file {
                let default_json = fs::read_to_string(default_file).map_err(ClixError::Io)?;
                let steps: Vec<WorkflowStep> =
                    serde_json::from_str(&default_json).map_err(ClixError::Serialization)?;
                Some(steps)
            } else {
                None
            };

            // Create branch step
            let branch_step = WorkflowStep::new_branch(
                args.name.clone(),
                args.description.clone(),
                args.variable.clone(),
                cases,
                default_case,
            );

            // Add the branch step to the workflow
            if let Some(ref mut steps) = command.steps {
                steps.push(branch_step);
            }
            storage.update_command(&command)?;

            println!(
                "{} Branch step '{}' added to workflow '{}'",
                "Success:".green().bold(),
                args.name,
                args.command_name
            );
        }

        Commands::ConvertFunction(args) => {
            use clix::commands::FunctionConverter;

            println!(
                "{} Converting function '{}' from '{}'...",
                "Info:".blue().bold(),
                args.function,
                args.file
            );

            let tags = args.tags.unwrap_or_else(Vec::new);

            match FunctionConverter::convert_function(
                &args.file,
                &args.function,
                &args.command_name,
                &args.description,
                tags.clone(),
            ) {
                Ok(workflow) => {
                    // Convert the workflow to a unified command
                    let command = Command::new_workflow(
                        args.command_name.clone(),
                        args.description.clone(),
                        workflow.steps,
                        tags,
                    );
                    storage.add_command(command)?;
                    println!(
                        "{} Function '{}' successfully converted to workflow '{}'",
                        "Success:".green().bold(),
                        args.function,
                        args.command_name
                    );
                }
                Err(e) => {
                    println!(
                        "{} Failed to convert function: {}",
                        "Error:".red().bold(),
                        e
                    );
                    return Err(e);
                }
            }
        }

        Commands::Export(export_args) => {
            let export_manager = ExportManager::new(storage.get_local_storage().clone());

            export_manager.export_with_filter(
                &export_args.output,
                export_args.tag,
                export_args.commands_only,
                export_args.workflows_only,
            )?;

            println!(
                "{} Commands and workflows exported to: {}",
                "Success:".green().bold(),
                export_args.output
            );
        }

        Commands::Ask(ask_args) => {
            // Load settings
            let settings_manager = SettingsManager::new()?;
            let settings = settings_manager.load()?;

            // Initialize Claude Assistant and conversation storage
            let assistant = ClaudeAssistant::new(settings)?;
            let conversation_storage = ConversationStorage::new()?;

            // Get all commands and workflows for context
            let commands = storage.list_commands()?;
            let workflows = storage.list_workflows()?;

            // Convert to references for the assistant
            let command_refs: Vec<&Command> = commands.iter().collect();
            let workflow_refs: Vec<&Workflow> = workflows.iter().collect();

            // Handle interactive mode or session continuation
            if ask_args.interactive || ask_args.session.is_some() {
                handle_conversational_ask(
                    ask_args,
                    &assistant,
                    &conversation_storage,
                    &storage,
                    command_refs,
                    workflow_refs,
                )?;
            } else {
                // Handle single-shot ask (legacy behavior)
                handle_single_ask(
                    &ask_args.question,
                    &assistant,
                    &storage,
                    command_refs,
                    workflow_refs,
                )?;
            }
        }

        Commands::Settings(settings_cmd) => {
            let settings_manager = SettingsManager::new()?;

            match settings_cmd {
                SettingsCommands::List => {
                    let settings = settings_manager.load()?;

                    println!("{}", "Current Settings:".blue().bold());
                    println!("{}", "=".repeat(50));
                    println!("{}: {}", "AI Model".green().bold(), settings.ai_model);
                    println!(
                        "{}: {}",
                        "AI Temperature".green().bold(),
                        settings.ai_settings.temperature
                    );
                    println!(
                        "{}: {}",
                        "AI Max Tokens".green().bold(),
                        settings.ai_settings.max_tokens
                    );
                }

                SettingsCommands::SetAiModel(args) => {
                    settings_manager.update_ai_model(&args.model)?;
                    println!(
                        "{} AI model set to: {}",
                        "Success:".green().bold(),
                        args.model
                    );
                }

                SettingsCommands::ListAiModels => {
                    // Load settings
                    let settings = settings_manager.load()?;

                    // Initialize Claude Assistant
                    let assistant = ClaudeAssistant::new(settings)?;

                    println!("{} Fetching available models...", "Info:".blue().bold());

                    match assistant.list_models() {
                        Ok(models) => {
                            println!("{}", "Available AI Models:".blue().bold());
                            println!("{}", "=".repeat(50));

                            for model in models {
                                println!("{}", model);
                            }
                        }
                        Err(e) => {
                            eprintln!("{} Failed to fetch models: {}", "Error:".red().bold(), e);
                            eprintln!(
                                "{} Make sure your Anthropic API key is set correctly.",
                                "Hint:".yellow().bold()
                            );
                        }
                    }
                }

                SettingsCommands::SetAiTemperature(args) => {
                    if args.temperature < 0.0 || args.temperature > 1.0 {
                        return Err(ClixError::InvalidCommandFormat(
                            "Temperature must be between 0.0 and 1.0".to_string(),
                        ));
                    }

                    settings_manager.update_ai_temperature(args.temperature)?;
                    println!(
                        "{} AI temperature set to: {}",
                        "Success:".green().bold(),
                        args.temperature
                    );
                }

                SettingsCommands::SetAiMaxTokens(args) => {
                    settings_manager.update_ai_max_tokens(args.max_tokens)?;
                    println!(
                        "{} AI max tokens set to: {}",
                        "Success:".green().bold(),
                        args.max_tokens
                    );
                }
            }
        }

        Commands::Import(import_args) => {
            let import_manager = ImportManager::new(storage.get_local_storage().clone());

            let summary =
                import_manager.import_from_file(&import_args.input, import_args.overwrite)?;

            println!(
                "{} Import completed from: {}",
                "Success:".green().bold(),
                import_args.input
            );

            println!("\n{}", "Import Summary:".blue().bold());
            println!("{}", "=".repeat(50));
            println!("{}: {}", "Commands Added".green(), summary.commands_added);
            println!(
                "{}: {}",
                "Commands Updated".green(),
                summary.commands_updated
            );
            println!(
                "{}: {}",
                "Commands Skipped".green(),
                summary.commands_skipped
            );
            println!("{}: {}", "Workflows Added".green(), summary.workflows_added);
            println!(
                "{}: {}",
                "Workflows Updated".green(),
                summary.workflows_updated
            );
            println!(
                "{}: {}",
                "Workflows Skipped".green(),
                summary.workflows_skipped
            );
            println!("{}", "-".repeat(50));
            println!(
                "{}: {}",
                "Exported By".green(),
                summary.metadata.exported_by
            );
            println!(
                "{}: {}",
                "Export Description".green(),
                summary.metadata.description
            );
        }

        Commands::Completions(completions_args) => {
            let mut app = CliArgs::command();
            let shell = match completions_args.shell {
                Shell::Bash => CompletionShell::Bash,
                Shell::Zsh => CompletionShell::Zsh,
                Shell::Fish => CompletionShell::Fish,
                Shell::PowerShell => CompletionShell::PowerShell,
                Shell::Elvish => CompletionShell::Elvish,
            };

            println!("# Generating shell completions for {:?}", shell);
            generate(shell, &mut app, "clix", &mut io::stdout());
        }

        Commands::Git(git_command) => match git_command {
            GitCommands::AddRepo(add_repo_args) => {
                storage
                    .get_git_manager()
                    .add_repository(add_repo_args.name.clone(), add_repo_args.url.clone())?;

                println!(
                    "{} Repository '{}' added and cloned successfully",
                    "Success:".green().bold(),
                    add_repo_args.name
                );

                // Sync after adding new repository
                storage.sync_with_repositories()?;
            }

            GitCommands::RemoveRepo(remove_repo_args) => {
                storage
                    .get_git_manager()
                    .remove_repository(&remove_repo_args.name)?;

                println!(
                    "{} Repository '{}' removed successfully",
                    "Success:".green().bold(),
                    remove_repo_args.name
                );
            }

            GitCommands::ListRepos => {
                let git_manager = storage.get_git_manager();
                let repos = git_manager.list_repositories();

                if repos.is_empty() {
                    println!("No git repositories configured yet.");
                    println!("Use 'clix git add-repo <name> --url <url>' to add one.");
                    return Ok(());
                }

                println!("{}", "Configured Git Repositories:".blue().bold());
                println!("{}", "=".repeat(50));

                for repo in repos {
                    println!("{}: {}", "Name".green().bold(), repo.name);
                    println!("{}: {}", "URL".green(), repo.url);
                    println!(
                        "{}: {}",
                        "Enabled".green(),
                        if repo.enabled { "✓" } else { "✗" }
                    );

                    // Check if repository is cloned
                    if let Some(git_repo) = git_manager.get_repository(&repo.name) {
                        if git_repo.is_cloned() {
                            println!("{}: ✓ Cloned", "Status".green());
                            println!("{}: {}", "Path".green(), git_repo.get_repo_path().display());
                        } else {
                            println!("{}: ✗ Not cloned", "Status".yellow());
                        }
                    }

                    println!("{}", "-".repeat(50));
                }
            }

            GitCommands::Pull => {
                println!("{} Pulling from all repositories...", "Info:".blue().bold());

                let git_manager = storage.get_git_manager();
                let results = git_manager.pull_all_repositories()?;

                println!("\n{}", "Pull Results:".blue().bold());
                println!("{}", "=".repeat(50));

                for (repo_name, result) in results {
                    match result {
                        Ok(()) => println!("✓ {}: Successfully updated", repo_name),
                        Err(e) => println!("✗ {}: Failed - {}", repo_name, e),
                    }
                }

                // Load changes after pulling
                storage.load_from_repositories()?;
                println!(
                    "\n{} Local commands updated with repository changes",
                    "Success:".green().bold()
                );
            }

            GitCommands::Status => {
                println!("{} Checking repository status...", "Info:".blue().bold());

                // Pull first
                let git_manager = storage.get_git_manager();
                let pull_results = git_manager.pull_all_repositories()?;

                println!("\n{}", "Repository Status:".blue().bold());
                println!("{}", "=".repeat(50));

                let repos = git_manager.list_repositories();
                for repo in repos {
                    println!("{}: {}", "Repository".green().bold(), repo.name);

                    if let Some(git_repo) = git_manager.get_repository(&repo.name) {
                        if git_repo.is_cloned() {
                            // Check pull result
                            if let Some((_, pull_result)) =
                                pull_results.iter().find(|(name, _)| name == &repo.name)
                            {
                                match pull_result {
                                    Ok(()) => println!("  Status: ✓ Up to date"),
                                    Err(e) => println!("  Status: ✗ Sync failed - {}", e),
                                }
                            }
                        } else {
                            println!("  Status: ✗ Not cloned");
                        }
                    }

                    println!("{}", "-".repeat(50));
                }

                // Load changes after status check
                storage.load_from_repositories()?;
            }
        },
    }

    Ok(())
}

fn handle_single_ask(
    question: &str,
    assistant: &ClaudeAssistant,
    storage: &GitIntegratedStorage,
    command_refs: Vec<&Command>,
    workflow_refs: Vec<&Workflow>,
) -> Result<()> {
    // Format question and get response
    println!("{} {}", "Question:".green().bold(), question);

    // Ask Claude (legacy single-shot mode)
    let (response, action) = assistant.ask(question, command_refs, workflow_refs)?;

    // Print Claude's response
    println!("{}", "\nClaude's Response:".blue().bold());
    println!("{}", response);

    // Handle suggested action
    execute_claude_action(action, assistant, storage)?;

    Ok(())
}

fn handle_conversational_ask(
    ask_args: clix::cli::app::AskArgs,
    assistant: &ClaudeAssistant,
    conversation_storage: &ConversationStorage,
    storage: &GitIntegratedStorage,
    command_refs: Vec<&Command>,
    workflow_refs: Vec<&Workflow>,
) -> Result<()> {
    let mut session = if let Some(session_id) = &ask_args.session {
        // Load existing session
        match conversation_storage.get_session(session_id)? {
            Some(session) => {
                println!(
                    "{} Continuing conversation session: {}",
                    "Info:".blue().bold(),
                    session_id
                );
                session
            }
            None => {
                return Err(ClixError::NotFound(format!(
                    "Conversation session '{}' not found",
                    session_id
                )));
            }
        }
    } else {
        // Create new session
        let session =
            ConversationSession::with_context(command_refs.clone(), workflow_refs.clone());
        println!(
            "{} Started new conversation session: {}",
            "Info:".blue().bold(),
            session.id
        );

        if ask_args.interactive {
            println!(
                "{} Interactive mode enabled. Type 'exit' or 'quit' to end the conversation.",
                "Info:".yellow().bold()
            );
        }

        session
    };

    // Add user's initial question to session
    session.add_message(MessageRole::User, ask_args.question.clone());
    let mut current_question = ask_args.question.clone();

    // Main conversation loop
    loop {
        println!("{} {}", "Question:".green().bold(), current_question);

        // Ask Claude in conversational mode
        let (response, action) = assistant.ask_conversational(
            &current_question,
            &session,
            command_refs.clone(),
            workflow_refs.clone(),
        )?;

        // Add Claude's response to session
        session.add_message(MessageRole::Assistant, response.clone());

        // Print Claude's response
        println!("{}", "\nClaude's Response:".blue().bold());
        println!("{}", response);

        // Handle suggested action
        execute_claude_action(action, assistant, storage)?;

        // Save session state
        conversation_storage.save_session(&session)?;

        // Check if we should continue the conversation
        if !ask_args.interactive {
            break; // Single question in session mode
        }

        // Check conversation state
        match session.state {
            ConversationState::Completed => {
                println!("{} Conversation completed.", "Info:".green().bold());
                break;
            }
            _ => {
                // Continue conversation - get next input
                print!(
                    "\n{} ",
                    "Continue conversation (or 'exit'/'quit' to end):"
                        .cyan()
                        .bold()
                );
                io::stdout().flush().map_err(|e| {
                    ClixError::CommandExecutionFailed(format!("Failed to flush stdout: {}", e))
                })?;

                let mut input = String::new();
                io::stdin().read_line(&mut input).map_err(|e| {
                    ClixError::CommandExecutionFailed(format!("Failed to read user input: {}", e))
                })?;

                let input = input.trim();
                if input.is_empty() || input == "exit" || input == "quit" {
                    session.set_state(ConversationState::Completed);
                    conversation_storage.update_session(&session)?;
                    println!(
                        "{} Conversation ended. Session ID: {}",
                        "Info:".green().bold(),
                        session.id
                    );
                    break;
                }

                // Add new user message and continue loop
                session.add_message(MessageRole::User, input.to_string());
                current_question = input.to_string();
            }
        }
    }

    Ok(())
}

fn execute_claude_action(
    action: clix::ai::claude::ClaudeAction,
    assistant: &ClaudeAssistant,
    storage: &GitIntegratedStorage,
) -> Result<()> {
    use clix::ai::claude::ClaudeAction;

    match action {
        ClaudeAction::RunCommand(ref name) => {
            if assistant.confirm_action(&action)? {
                let command = storage.get_command(name)?;
                let output = CommandExecutor::execute_command(&command)?;
                CommandExecutor::print_command_output(&output);

                // Update usage statistics
                storage.update_command_usage(name)?;
            }
        }
        ClaudeAction::RunWorkflow(ref name) => {
            if assistant.confirm_action(&action)? {
                let workflow = storage.get_workflow(name)?;
                let results = CommandExecutor::execute_workflow(&workflow, None, None)?;

                // Print all results
                println!("\n{}", "Workflow Results:".blue().bold());
                println!("{}", "=".repeat(50));

                for (step_name, result) in results {
                    println!("{}: {}", "Step".green().bold(), step_name);

                    match result {
                        Ok(output) => CommandExecutor::print_command_output(&output),
                        Err(e) => println!("{} {}", "Error:".red().bold(), e),
                    }

                    println!("{}", "-".repeat(50));
                }

                // Update usage statistics
                storage.update_workflow_usage(name)?;
            }
        }
        ClaudeAction::CreateCommand {
            ref name,
            ref description,
            ref command,
        } => {
            if assistant.confirm_action(&action)? {
                let command = Command::new(
                    name.clone(),
                    description.clone(),
                    command.clone(),
                    vec!["claude-generated".to_string()],
                );

                storage.add_command(command)?;
                println!(
                    "{} Command '{}' added successfully",
                    "Success:".green().bold(),
                    name
                );
            }
        }
        ClaudeAction::CreateWorkflow {
            ref name,
            ref description,
            ref steps,
        } => {
            if assistant.confirm_action(&action)? {
                let workflow = Workflow::new(
                    name.clone(),
                    description.clone(),
                    steps.clone(),
                    vec!["claude-generated".to_string()],
                );

                storage.add_workflow(workflow)?;
                println!(
                    "{} Workflow '{}' added successfully",
                    "Success:".green().bold(),
                    name
                );
            }
        }
        ClaudeAction::NoAction => {}
    }

    Ok(())
}
