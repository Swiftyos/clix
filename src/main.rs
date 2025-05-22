use clap::Parser;
use colored::Colorize;
use std::fs;
use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};

use clix::commands::{Command, CommandExecutor, Workflow, WorkflowStep};
use clix::storage::Storage;
use clix::cli::app::{CliArgs, Commands};
use clix::error::{ClixError, Result};
use clix::share::{ExportManager, ImportManager};

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
            let command = Command::new(
                add_args.name,
                add_args.description,
                add_args.command,
                tags,
            );
            
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
            if !filtered_commands.is_empty() {
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
                        
                        println!("{}: {} ({} days ago)", 
                            "Last used".green(), 
                            cmd.use_count,
                            days_ago
                        );
                    }
                    
                    println!("{}", "-".repeat(50));
                }
            }
            
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
                        
                        println!("{}: {} ({} days ago)", 
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
            println!("{} Command '{}' removed successfully", 
                "Success:".green().bold(), 
                remove_args.name
            );
        }
        
        Commands::AddWorkflow(add_args) => {
            let tags = add_args.tags.unwrap_or_else(Vec::new);
            
            // Read steps from JSON file
            let steps_json = fs::read_to_string(&add_args.steps_file)
                .map_err(|e| ClixError::Io(e))?;
                
            let steps: Vec<WorkflowStep> = serde_json::from_str(&steps_json)
                .map_err(|e| ClixError::Serialization(e))?;
                
            let workflow = Workflow::new(
                add_args.name,
                add_args.description,
                steps,
                tags,
            );
            
            storage.add_workflow(workflow)?;
            println!("{} Workflow added successfully", "Success:".green().bold());
        }
        
        Commands::RunWorkflow(run_args) => {
            let workflow = storage.get_workflow(&run_args.name)?;
            let results = CommandExecutor::execute_workflow(&workflow)?;
            
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
        
        Commands::Export(export_args) => {
            let export_manager = ExportManager::new(storage);
            
            export_manager.export_with_filter(
                &export_args.output,
                export_args.tag,
                export_args.commands_only,
                export_args.workflows_only,
            )?;
            
            println!("{} Commands and workflows exported to: {}", 
                "Success:".green().bold(),
                export_args.output
            );
        }
        
        Commands::Import(import_args) => {
            let import_manager = ImportManager::new(storage);
            
            let summary = import_manager.import_from_file(
                &import_args.input,
                import_args.overwrite,
            )?;
            
            println!("{} Import completed from: {}", 
                "Success:".green().bold(),
                import_args.input
            );
            
            println!("\n{}", "Import Summary:".blue().bold());
            println!("{}", "=".repeat(50));
            println!("{}: {}", "Commands Added".green(), summary.commands_added);
            println!("{}: {}", "Commands Updated".green(), summary.commands_updated);
            println!("{}: {}", "Commands Skipped".green(), summary.commands_skipped);
            println!("{}: {}", "Workflows Added".green(), summary.workflows_added);
            println!("{}: {}", "Workflows Updated".green(), summary.workflows_updated);
            println!("{}: {}", "Workflows Skipped".green(), summary.workflows_skipped);
            println!("{}", "-".repeat(50));
            println!("{}: {}", "Exported By".green(), summary.metadata.exported_by);
            println!("{}: {}", "Export Description".green(), summary.metadata.description);
        }
    }

    Ok(())
}