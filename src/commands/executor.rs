use crate::commands::models::{Command, StepType, Workflow, WorkflowStep};
use crate::commands::variables::{VariableProcessor, WorkflowContext};
use crate::error::{ClixError, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::process::{Command as ProcessCommand, Output};

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn execute_command(command: &Command) -> Result<Output> {
        println!("{} {}", "Executing:".blue().bold(), command.name);
        println!("{} {}", "Description:".blue().bold(), command.description);
        println!("{} {}", "Command:".blue().bold(), command.command);

        let output = if cfg!(target_os = "windows") {
            ProcessCommand::new("cmd")
                .args(["/C", &command.command])
                .output()
        } else {
            ProcessCommand::new("sh")
                .args(["-c", &command.command])
                .output()
        };

        match output {
            Ok(output) => Ok(output),
            Err(e) => Err(ClixError::CommandExecutionFailed(format!(
                "Failed to execute: {}",
                e
            ))),
        }
    }

    pub fn execute_workflow(
        workflow: &Workflow,
        profile_name: Option<&str>,
        provided_vars: Option<HashMap<String, String>>,
    ) -> Result<Vec<(String, Result<Output>)>> {
        println!("{} {}", "Executing workflow:".blue().bold(), workflow.name);
        println!("{} {}", "Description:".blue().bold(), workflow.description);

        // Create workflow context with variables
        let mut context = WorkflowContext::new();

        // Apply profile variables if a profile was specified
        if let Some(profile_name) = profile_name {
            if let Some(profile) = workflow.get_profile(profile_name) {
                println!("{} {}", "Using profile:".blue().bold(), profile.name);
                context.merge_variables(profile.variables.clone());
            } else {
                println!(
                    "{} Profile '{}' not found",
                    "Warning:".yellow().bold(),
                    profile_name
                );
            }
        }

        // Apply provided variables (override profile values)
        if let Some(vars) = provided_vars {
            context.merge_variables(vars);
        }

        // Ask for any missing required variables
        VariableProcessor::prompt_for_variables(workflow, &mut context)?;

        let mut results = Vec::new();

        for (index, step) in workflow.steps.iter().enumerate() {
            println!(
                "\n{} {} - {}",
                "Step".blue().bold(),
                (index + 1).to_string().blue().bold(),
                step.name
            );
            println!("{} {}", "Description:".blue().bold(), step.description);
            println!("{} {}", "Command:".blue().bold(), step.command);

            // Process variables in the step
            let processed_step = VariableProcessor::process_step(step, &context);

            let result = match processed_step.step_type {
                StepType::Command => Self::execute_command_step(&processed_step),
                StepType::Auth => Self::execute_auth_step(&processed_step),
            };

            // Check if we need to continue before moving the result
            let should_continue = match &result {
                Ok(_) => true,
                Err(_) => step.continue_on_error,
            };

            // Store the result
            results.push((step.name.clone(), result));

            if !should_continue {
                println!(
                    "{} Command failed, stopping workflow",
                    "Error:".red().bold()
                );
                break;
            }
        }

        Ok(results)
    }

    fn execute_command_step(step: &WorkflowStep) -> Result<Output> {
        let output = if cfg!(target_os = "windows") {
            ProcessCommand::new("cmd")
                .args(["/C", &step.command])
                .output()
        } else {
            ProcessCommand::new("sh")
                .args(["-c", &step.command])
                .output()
        };

        match output {
            Ok(output) => Ok(output),
            Err(e) => Err(ClixError::CommandExecutionFailed(format!(
                "Failed to execute: {}",
                e
            ))),
        }
    }

    fn execute_auth_step(step: &WorkflowStep) -> Result<Output> {
        // First, execute the command which typically starts an auth flow
        let output = if cfg!(target_os = "windows") {
            ProcessCommand::new("cmd")
                .args(["/C", &step.command])
                .output()
        } else {
            ProcessCommand::new("sh")
                .args(["-c", &step.command])
                .output()
        };

        match output {
            Ok(output) => {
                // Display the output to the user
                if !output.stdout.is_empty() {
                    println!("\n{}", "STDOUT:".green().bold());
                    println!("{}", String::from_utf8_lossy(&output.stdout));
                }

                if !output.stderr.is_empty() {
                    println!("\n{}", "STDERR:".red().bold());
                    println!("{}", String::from_utf8_lossy(&output.stderr));
                }

                println!(
                    "\n{}",
                    "This step requires authentication. Please follow the instructions above."
                        .yellow()
                        .bold()
                );
                println!(
                    "{}",
                    "Press Enter when you have completed the authentication process...".yellow()
                );

                // Wait for user to confirm they've completed the auth process
                let stdin = io::stdin();
                let mut handle = stdin.lock();
                let mut input = String::new();

                // Flush stdout to ensure prompts are displayed
                io::stdout().flush().map_err(|e| {
                    ClixError::CommandExecutionFailed(format!("Failed to flush stdout: {}", e))
                })?;

                handle.read_line(&mut input).map_err(|e| {
                    ClixError::CommandExecutionFailed(format!("Failed to read user input: {}", e))
                })?;

                println!(
                    "{}",
                    "Authentication confirmed, continuing workflow.".green()
                );
                Ok(output)
            }
            Err(e) => Err(ClixError::CommandExecutionFailed(format!(
                "Failed to execute auth command: {}",
                e
            ))),
        }
    }

    pub fn print_command_output(output: &Output) {
        if !output.stdout.is_empty() {
            println!("\n{}", "STDOUT:".green().bold());
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }

        if !output.stderr.is_empty() {
            println!("\n{}", "STDERR:".red().bold());
            println!("{}", String::from_utf8_lossy(&output.stderr));
        }

        println!(
            "\n{} {}",
            "Exit status:".blue().bold(),
            if output.status.success() {
                "Success".green()
            } else {
                format!("Failed ({})", output.status).red()
            }
        );
    }
}
