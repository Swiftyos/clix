use clap::Parser;
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};

use clix::cli::app::{AskArgs, CliArgs, Commands, FlowCommands};
use clix::commands::{
    Command, CommandExecutor, Workflow, WorkflowStep, WorkflowVariable, WorkflowVariableProfile,
};
use clix::ClaudeAssistant;
use clix::error::{ClixError, Result};
use clix::share::{ExportManager, ImportManager};
use clix::storage::Storage;

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {}", "Error:".red().bold(), e);
        exit(1);
    }
}

fn run() -> Result<()> {
    let args = CliArgs::parse();
    let storage = Storage::new()?;

    match args.command {
        Commands::Add(add_args) => {
            let tags = add_args.tags.unwrap_or_else(Vec::new);
            let command = Command::new(add_args.name, add_args.description, add_args.command, tags);

            storage.add_command(command)?;
            println!("{} Command added successfully", "Success:".green().bold());
        }

        Commands::Run(run_args) => {
            let command = storage.get_command(&run_args.name)?;
            let output = CommandExecutor::execute_command(&command)?;
            CommandExecutor::print_command_output(&output);

            // Update usage statistics
            storage.update_command_usage(&run_args.name)?;
        }

        Commands::List(list_args) => {
            let commands = storage.list_commands()?;
            let workflows = storage.list_workflows()?;

            if commands.is_empty() && workflows.is_empty() {
                println!("No commands or workflows stored yet.");
                return Ok(());
            }

            // Skip workflows if commands_only is set
            let show_workflows = !list_args.commands_only;
            // Skip commands if workflows_only is set
            let show_commands = !list_args.workflows_only;

            // Filter by tag if provided
            let filtered_commands = if let Some(ref tag) = list_args.tag {
                commands
                    .into_iter()
                    .filter(|cmd| cmd.tags.contains(tag))
                    .collect::<Vec<_>>()
            } else {
                commands
            };

            let filtered_workflows = if let Some(ref tag) = list_args.tag {
                workflows
                    .into_iter()
                    .filter(|wf| wf.tags.contains(tag))
                    .collect::<Vec<_>>()
            } else {
                workflows
            };

            // Print commands
            if show_commands && !filtered_commands.is_empty() {
                println!("\n{}", "Commands:".blue().bold());
                println!("{}", "=".repeat(50));

                for cmd in filtered_commands {
                    println!("{}: {}", "Name".green().bold(), cmd.name);
                    println!("{}: {}", "Description".green(), cmd.description);
                    println!("{}: {}", "Command".green(), cmd.command);

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

            // Print workflows
            if show_workflows && !filtered_workflows.is_empty() {
                println!("\n{}", "Workflows:".blue().bold());
                println!("{}", "=".repeat(50));

                for wf in filtered_workflows {
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

        // New flow subcommand handling
        Commands::Flow(flow_command) => match flow_command {
            FlowCommands::Add(add_args) => {
                let tags = add_args.tags.unwrap_or_else(Vec::new);

                // Read steps from JSON file
                let steps_json = fs::read_to_string(&add_args.steps_file).map_err(ClixError::Io)?;

                let steps: Vec<WorkflowStep> =
                    serde_json::from_str(&steps_json).map_err(ClixError::Serialization)?;

                let workflow = Workflow::new(add_args.name, add_args.description, steps, tags);

                storage.add_workflow(workflow)?;
                println!("{} Workflow added successfully", "Success:".green().bold());
            }

            FlowCommands::Run(run_args) => {
                let workflow = storage.get_workflow(&run_args.name)?;

                // Parse variable values if provided
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

                // Update usage statistics
                storage.update_workflow_usage(&run_args.name)?;
            }

            FlowCommands::Remove(remove_args) => {
                storage.remove_workflow(&remove_args.name)?;
                println!(
                    "{} Workflow '{}' removed successfully",
                    "Success:".green().bold(),
                    remove_args.name
                );
            }

            FlowCommands::AddVar(add_var_args) => {
                let mut workflow = storage.get_workflow(&add_var_args.workflow_name)?;

                let variable = WorkflowVariable::new(
                    add_var_args.name,
                    add_var_args.description,
                    add_var_args.default,
                    add_var_args.required,
                );

                workflow.add_variable(variable);
                storage.update_workflow(&workflow)?;

                println!(
                    "{} Variable added to workflow '{}'",
                    "Success:".green().bold(),
                    add_var_args.workflow_name
                );
            }

            FlowCommands::AddProfile(add_profile_args) => {
                let mut workflow = storage.get_workflow(&add_profile_args.workflow_name)?;

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

                workflow.add_profile(profile);
                storage.update_workflow(&workflow)?;

                println!(
                    "{} Profile added to workflow '{}'",
                    "Success:".green().bold(),
                    add_profile_args.workflow_name
                );
            }

            FlowCommands::ListProfiles(list_profiles_args) => {
                let workflow = storage.get_workflow(&list_profiles_args.workflow_name)?;

                if workflow.profiles.is_empty() {
                    println!(
                        "No profiles defined for workflow '{}'.",
                        list_profiles_args.workflow_name
                    );
                    return Ok(());
                }

                println!("{}", "Workflow Profiles:".blue().bold());
                println!("{}", "=".repeat(50));

                for (name, profile) in &workflow.profiles {
                    println!("{}: {}", "Profile".green().bold(), name);
                    println!("{}: {}", "Description".green(), profile.description);
                    println!("{}: {}", "Variables".green(), profile.variables.len());

                    for (var_name, var_value) in &profile.variables {
                        println!("{}: {} = {}", "  Variable".yellow(), var_name, var_value);
                    }

                    println!("{}", "-".repeat(50));
                }
            }

            FlowCommands::List(list_args) => {
                let workflows = storage.list_workflows()?;

                if workflows.is_empty() {
                    println!("No workflows stored yet.");
                    return Ok(());
                }

                // Filter by tag if provided
                let filtered_workflows = if let Some(ref tag) = list_args.tag {
                    workflows
                        .into_iter()
                        .filter(|wf| wf.tags.contains(tag))
                        .collect::<Vec<_>>()
                } else {
                    workflows
                };

                // Print workflows
                if !filtered_workflows.is_empty() {
                    println!("\n{}", "Workflows:".blue().bold());
                    println!("{}", "=".repeat(50));

                    for wf in filtered_workflows {
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
        },

        Commands::Export(export_args) => {
            let export_manager = ExportManager::new(storage);

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
            // Initialize Claude Assistant
            let assistant = ClaudeAssistant::new()?;
            
            // Get all commands and workflows for context
            let commands = storage.list_commands()?;
            let workflows = storage.list_workflows()?;
            
            // Format question and get response
            println!("{} {}", "Question:".green().bold(), ask_args.question);
            
            // Convert to references for the assistant
            let command_refs: Vec<&Command> = commands.iter().collect();
            let workflow_refs: Vec<&Workflow> = workflows.iter().collect();
            
            // Ask Claude
            let (response, action) = assistant.ask(&ask_args.question, command_refs, workflow_refs)?;
            
            // Print Claude's response
            println!("{}", "\nClaude's Response:".blue().bold());
            println!("{}", response);
            
            // Handle suggested action
            match action {
                clix::ai::claude::ClaudeAction::RunCommand(name) => {
                    if assistant.confirm_action(&action)? {
                        let command = storage.get_command(&name)?;
                        let output = CommandExecutor::execute_command(&command)?;
                        CommandExecutor::print_command_output(&output);
                        
                        // Update usage statistics
                        storage.update_command_usage(&name)?;
                    }
                },
                clix::ai::claude::ClaudeAction::RunWorkflow(name) => {
                    if assistant.confirm_action(&action)? {
                        let workflow = storage.get_workflow(&name)?;
                        let results = CommandExecutor::execute_workflow(
                            &workflow,
                            None,
                            None,
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
                        
                        // Update usage statistics
                        storage.update_workflow_usage(&name)?;
                    }
                },
                clix::ai::claude::ClaudeAction::CreateCommand { name, description, command } => {
                    if assistant.confirm_action(&action)? {
                        let command = Command::new(name.clone(), description, command, vec!["claude-generated".to_string()]);
                        
                        storage.add_command(command)?;
                        println!("{} Command '{}' added successfully", "Success:".green().bold(), name);
                    }
                },
                clix::ai::claude::ClaudeAction::CreateWorkflow { name, description, steps } => {
                    if assistant.confirm_action(&action)? {
                        let workflow = Workflow::new(
                            name.clone(),
                            description,
                            steps,
                            vec!["claude-generated".to_string()],
                        );
                        
                        storage.add_workflow(workflow)?;
                        println!("{} Workflow '{}' added successfully", "Success:".green().bold(), name);
                    }
                },
                clix::ai::claude::ClaudeAction::NoAction => {},
            }
        },
        
        Commands::Import(import_args) => {
            let import_manager = ImportManager::new(storage);

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
    }

    Ok(())
}
