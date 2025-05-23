use crate::commands::expression::ExpressionEvaluator;
use crate::commands::models::{Command, ConditionalAction, StepType, Workflow, WorkflowStep};
use crate::commands::variables::{VariableProcessor, WorkflowContext};
use crate::error::{ClixError, Result};
use crate::security::{CommandSanitizer, SecurityConfig, SecurityValidator};
use colored::Colorize;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;
use std::process::{Command as ProcessCommand, Output};

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn execute_command(command: &Command) -> Result<Output> {
        let command_str = command.command.as_ref().ok_or_else(|| {
            ClixError::InvalidCommandFormat(
                "Command has no executable command string (it may be a workflow)".to_string(),
            )
        })?;

        println!("{} {}", "Executing:".blue().bold(), command.name);
        println!("{} {}", "Description:".blue().bold(), command.description);
        println!("{} {}", "Command:".blue().bold(), command_str);

        // Security validation
        Self::validate_command_security(command_str)?;

        let output = if cfg!(target_os = "windows") {
            ProcessCommand::new("cmd")
                .args(["/C", command_str])
                .output()
        } else {
            ProcessCommand::new("sh").args(["-c", command_str]).output()
        };

        match output {
            Ok(output) => Ok(output),
            Err(e) => Err(ClixError::CommandExecutionFailed(format!(
                "Failed to execute: {}",
                e
            ))),
        }
    }

    /// Validate command security before execution
    fn validate_command_security(command: &str) -> Result<()> {
        let config = SecurityConfig::default();
        let validator = SecurityValidator::new(config);

        // Sanitize the command first
        let sanitized_command = CommandSanitizer::sanitize_command(command)?;

        // Validate for security issues
        let security_check = validator.validate_command(&sanitized_command)?;

        if !security_check.is_safe {
            println!("{}", "Security Warning:".red().bold());
            for issue in &security_check.issues {
                println!("  ⚠️  {}", issue.yellow());
            }

            // Get recommendations
            let recommendations = validator.get_security_recommendations(&sanitized_command);
            if !recommendations.is_empty() {
                println!("\n{}", "Security Recommendations:".blue().bold());
                for rec in recommendations {
                    println!("  💡 {}", rec);
                }
            }

            // For now, we'll warn but still allow execution
            // In production, you might want to block dangerous commands
            println!(
                "\n{}",
                "⚠️  Command has security concerns but will be executed. Use with caution!"
                    .yellow()
                    .bold()
            );
        }

        if security_check.requires_approval {
            println!(
                "{}",
                "This command requires additional approval due to security concerns."
                    .yellow()
                    .bold()
            );
            Self::request_security_approval(&sanitized_command)?;
        }

        Ok(())
    }

    /// Request security approval from user
    fn request_security_approval(command: &str) -> Result<()> {
        println!("{}", "🔒 Security Approval Required".red().bold());
        println!("{} {}", "Command:".blue().bold(), command);
        println!(
            "{}",
            "This command has been flagged for security review.".yellow()
        );

        print!(
            "{} [y/N]: ",
            "Do you want to proceed with execution?".yellow().bold()
        );
        io::stdout().flush().map_err(|e| {
            ClixError::CommandExecutionFailed(format!("Failed to flush stdout: {}", e))
        })?;

        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut input = String::new();

        handle.read_line(&mut input).map_err(|e| {
            ClixError::CommandExecutionFailed(format!(
                "Failed to read security approval input: {}",
                e
            ))
        })?;

        let input = input.trim().to_lowercase();
        if input == "y" || input == "yes" {
            println!(
                "{}",
                "✅ Security approval granted, proceeding with execution.".green()
            );
            Ok(())
        } else {
            Err(ClixError::SecurityError(
                "Command execution blocked by security policy".to_string(),
            ))
        }
    }

    pub fn execute_workflow(
        workflow: &Workflow,
        profile_name: Option<&str>,
        provided_vars: Option<HashMap<String, String>>,
    ) -> Result<Vec<(String, Result<Output>)>> {
        Self::execute_workflow_with_approval(workflow, profile_name, provided_vars, true)
    }

    /// Execute workflow with optional approval bypass for testing
    pub fn execute_workflow_with_approval(
        workflow: &Workflow,
        profile_name: Option<&str>,
        provided_vars: Option<HashMap<String, String>>,
        require_approval: bool,
    ) -> Result<Vec<(String, Result<Output>)>> {
        println!("{} {}", "Executing workflow:".blue().bold(), workflow.name);
        println!("{} {}", "Description:".blue().bold(), workflow.description);

        // Security validation for the entire workflow
        if require_approval {
            Self::validate_workflow_security(workflow)?;
        }

        let mut context = Self::setup_workflow_context(workflow, profile_name, provided_vars)?;
        let mut results = Vec::new();
        let mut last_output: Option<Output> = None;

        for (index, step) in workflow.steps.iter().enumerate() {
            Self::print_step_header(step, index);

            // Process variables in the step
            let processed_step = VariableProcessor::process_step(step, &context);

            // Check if step requires approval
            if require_approval && processed_step.require_approval {
                Self::request_approval(&processed_step)?;
            }

            // Execute the step
            let result = Self::execute_single_step(
                &processed_step,
                &mut context,
                &mut results,
                last_output.as_ref(),
            );

            // Update the last_output if this step produced an output
            if let Ok(ref output) = result {
                last_output = Some(output.clone());
            }

            // Check if we should continue after this step
            if !Self::should_continue_after_step(&result, &processed_step) {
                println!(
                    "{} Command failed, stopping workflow",
                    "Error:".red().bold()
                );
                break;
            }

            // Store the result
            results.push((step.name.clone(), result));
        }

        Ok(results)
    }

    /// Setup workflow context with variables, profiles, and user input
    fn setup_workflow_context(
        workflow: &Workflow,
        profile_name: Option<&str>,
        provided_vars: Option<HashMap<String, String>>,
    ) -> Result<WorkflowContext> {
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

        Ok(context)
    }

    /// Print step header information
    fn print_step_header(step: &WorkflowStep, index: usize) {
        println!(
            "\n{} {} - {}",
            "Step".blue().bold(),
            (index + 1).to_string().blue().bold(),
            step.name
        );
        println!("{} {}", "Description:".blue().bold(), step.description);

        if !step.command.is_empty() {
            println!("{} {}", "Command:".blue().bold(), step.command);
        }
    }

    /// Execute a single workflow step
    fn execute_single_step(
        step: &WorkflowStep,
        context: &mut WorkflowContext,
        results: &mut Vec<(String, Result<Output>)>,
        last_output: Option<&Output>,
    ) -> Result<Output> {
        match step.step_type {
            StepType::Command => Self::execute_command_step(step),
            StepType::Auth => Self::execute_auth_step(step),
            StepType::Conditional => {
                Self::execute_conditional_step(step, &context.variables, last_output)
            }
            StepType::Branch => Self::execute_branch_step(step, context, results),
            StepType::Loop => Self::execute_loop_step(step, context, results),
        }
    }

    /// Determine if workflow should continue after a step
    fn should_continue_after_step(result: &Result<Output>, step: &WorkflowStep) -> bool {
        match result {
            Ok(_) => true,
            Err(_) => step.continue_on_error,
        }
    }

    /// Validate workflow security before execution
    fn validate_workflow_security(workflow: &Workflow) -> Result<()> {
        let config = SecurityConfig::default();
        let validator = SecurityValidator::new(config);

        let security_report = validator.validate_workflow(workflow)?;

        if !security_report.is_safe {
            println!("{}", "🔒 Workflow Security Warning".red().bold());
            println!(
                "{}: {}",
                "Workflow".blue().bold(),
                security_report.workflow_name
            );

            for issue in &security_report.issues {
                println!("  ⚠️  {}", issue.yellow());
            }

            println!("\n{}", "Step-by-step security report:".blue().bold());
            for step_report in &security_report.step_reports {
                if !step_report.is_safe {
                    println!("  📋 {}: {}", "Step".yellow().bold(), step_report.step_name);
                    for issue in &step_report.issues {
                        println!("    ⚠️  {}", issue.yellow());
                    }
                }
            }

            println!(
                "\n{}",
                "⚠️  Workflow has security concerns but will be executed. Use with caution!"
                    .yellow()
                    .bold()
            );
        }

        if security_report.requires_approval {
            println!(
                "{}",
                "This workflow requires additional security approval."
                    .yellow()
                    .bold()
            );
            Self::request_workflow_security_approval(workflow)?;
        }

        Ok(())
    }

    /// Request workflow-level security approval
    fn request_workflow_security_approval(workflow: &Workflow) -> Result<()> {
        println!("{}", "🔒 Workflow Security Approval Required".red().bold());
        println!("{} {}", "Workflow:".blue().bold(), workflow.name);
        println!("{} {}", "Description:".blue().bold(), workflow.description);
        println!("{} {}", "Steps:".blue().bold(), workflow.steps.len());
        println!(
            "{}",
            "This workflow contains steps that require security review.".yellow()
        );

        print!(
            "{} [y/N]: ",
            "Do you want to proceed with workflow execution?"
                .yellow()
                .bold()
        );
        io::stdout().flush().map_err(|e| {
            ClixError::CommandExecutionFailed(format!("Failed to flush stdout: {}", e))
        })?;

        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut input = String::new();

        handle.read_line(&mut input).map_err(|e| {
            ClixError::CommandExecutionFailed(format!(
                "Failed to read workflow security approval input: {}",
                e
            ))
        })?;

        let input = input.trim().to_lowercase();
        if input == "y" || input == "yes" {
            println!(
                "{}",
                "✅ Workflow security approval granted, proceeding with execution.".green()
            );
            Ok(())
        } else {
            Err(ClixError::SecurityError(
                "Workflow execution blocked by security policy".to_string(),
            ))
        }
    }

    /// Execute a conditional step (if/then/else)
    fn execute_conditional_step(
        step: &WorkflowStep,
        variables: &HashMap<String, String>,
        last_output: Option<&Output>,
    ) -> Result<Output> {
        // Conditional steps must have a conditional property
        let conditional = step.conditional.as_ref().ok_or_else(|| {
            ClixError::CommandExecutionFailed(
                "Conditional step missing conditional property".to_string(),
            )
        })?;

        // Evaluate the condition
        println!(
            "{} {}",
            "Evaluating condition:".blue().bold(),
            conditional.condition.expression
        );

        let condition_result = ExpressionEvaluator::evaluate(
            &conditional.condition.expression,
            variables,
            last_output,
        )?;

        println!("{} {}", "Condition result:".blue().bold(), condition_result);

        // Store the result in a variable if specified
        if let Some(var_name) = &conditional.condition.variable {
            println!(
                "{} {} = {}",
                "Setting variable:".blue().bold(),
                var_name,
                condition_result
            );
            // TODO: Update the context with the result variable
            // For now, we can't update the context because it's not mutable in this method
        }

        // Determine what action to take based on condition result and specified action
        let action = match (&conditional.action, condition_result) {
            (Some(ConditionalAction::RunThen), _) => ConditionalAction::RunThen,
            (Some(ConditionalAction::RunElse), _) => ConditionalAction::RunElse,
            (Some(ConditionalAction::Continue), _) => ConditionalAction::Continue,
            (Some(ConditionalAction::Break), _) => ConditionalAction::Break,
            (Some(ConditionalAction::Return(code)), _) => ConditionalAction::Return(*code),
            (None, true) => ConditionalAction::RunThen,
            (None, false) => {
                if conditional.else_block.is_some() {
                    ConditionalAction::RunElse
                } else {
                    ConditionalAction::Continue
                }
            }
        };

        // Take the appropriate action
        match action {
            ConditionalAction::RunThen => {
                println!("{}", "Executing 'then' block".blue().bold());
                // Execute the steps in the then block
                let mut context = WorkflowContext::new();
                context.variables = variables.clone();

                // We'll execute the steps and use the last step's output as our result
                let mut last_step_output = None;
                let mut results = Vec::new();

                for (index, step) in conditional.then_block.steps.iter().enumerate() {
                    println!(
                        "\n{} {} - {}",
                        "Then Block Step".blue().bold(),
                        (index + 1).to_string().blue().bold(),
                        step.name
                    );

                    // Process variables in the step
                    let processed_step = VariableProcessor::process_step(step, &context);

                    // Check if step requires approval
                    if processed_step.require_approval {
                        Self::request_approval(&processed_step)?;
                    }

                    // Execute the step
                    let result = match processed_step.step_type {
                        StepType::Command => Self::execute_command_step(&processed_step),
                        StepType::Auth => Self::execute_auth_step(&processed_step),
                        StepType::Conditional => Self::execute_conditional_step(
                            &processed_step,
                            &context.variables,
                            last_step_output.as_ref(),
                        ),
                        StepType::Branch => {
                            Self::execute_branch_step(&processed_step, &mut context, &mut results)
                        }
                        StepType::Loop => {
                            Self::execute_loop_step(&processed_step, &mut context, &mut results)
                        }
                    };

                    // Update last_step_output if successful
                    if let Ok(ref output) = result {
                        last_step_output = Some(output.clone());
                    }

                    // Check if we need to continue
                    let should_continue = match &result {
                        Ok(_) => true,
                        Err(_) => processed_step.continue_on_error,
                    };

                    // Store the result
                    results.push((processed_step.name.clone(), result));

                    if !should_continue {
                        println!(
                            "{} Command failed, stopping conditional block execution",
                            "Error:".red().bold()
                        );
                        break;
                    }
                }

                // Return the last output if we have one, or create a success output
                if let Some(output) = last_step_output {
                    Ok(output)
                } else {
                    Ok(Output {
                        status: std::process::ExitStatus::from_raw(0),
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    })
                }
            }
            ConditionalAction::RunElse => {
                if let Some(else_block) = &conditional.else_block {
                    println!("{}", "Executing 'else' block".blue().bold());

                    // Execute the steps in the else block
                    let mut context = WorkflowContext::new();
                    context.variables = variables.clone();

                    // We'll execute the steps and use the last step's output as our result
                    let mut last_step_output = None;
                    let mut results = Vec::new();

                    for (index, step) in else_block.steps.iter().enumerate() {
                        println!(
                            "\n{} {} - {}",
                            "Else Block Step".blue().bold(),
                            (index + 1).to_string().blue().bold(),
                            step.name
                        );

                        // Process variables in the step
                        let processed_step = VariableProcessor::process_step(step, &context);

                        // Check if step requires approval
                        if processed_step.require_approval {
                            Self::request_approval(&processed_step)?;
                        }

                        // Execute the step
                        let result = match processed_step.step_type {
                            StepType::Command => Self::execute_command_step(&processed_step),
                            StepType::Auth => Self::execute_auth_step(&processed_step),
                            StepType::Conditional => Self::execute_conditional_step(
                                &processed_step,
                                &context.variables,
                                last_step_output.as_ref(),
                            ),
                            StepType::Branch => Self::execute_branch_step(
                                &processed_step,
                                &mut context,
                                &mut results,
                            ),
                            StepType::Loop => {
                                Self::execute_loop_step(&processed_step, &mut context, &mut results)
                            }
                        };

                        // Update last_step_output if successful
                        if let Ok(ref output) = result {
                            last_step_output = Some(output.clone());
                        }

                        // Check if we need to continue
                        let should_continue = match &result {
                            Ok(_) => true,
                            Err(_) => processed_step.continue_on_error,
                        };

                        // Store the result
                        results.push((processed_step.name.clone(), result));

                        if !should_continue {
                            println!(
                                "{} Command failed, stopping conditional block execution",
                                "Error:".red().bold()
                            );
                            break;
                        }
                    }

                    // Return the last output if we have one, or create a success output
                    if let Some(output) = last_step_output {
                        Ok(output)
                    } else {
                        Ok(Output {
                            status: std::process::ExitStatus::from_raw(0),
                            stdout: Vec::new(),
                            stderr: Vec::new(),
                        })
                    }
                } else {
                    // No else block, return a success output
                    Ok(Output {
                        status: std::process::ExitStatus::from_raw(0),
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    })
                }
            }
            ConditionalAction::Continue => {
                println!("{}", "Skipping conditional block".blue().bold());
                // Return a success output
                Ok(Output {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            }
            ConditionalAction::Break => {
                println!("{}", "Breaking workflow execution".yellow().bold());
                Err(ClixError::CommandExecutionFailed(
                    "Workflow execution stopped by conditional break".to_string(),
                ))
            }
            ConditionalAction::Return(code) => {
                println!("{} {}", "Returning with exit code:".yellow().bold(), code);
                // Create an output with the specified exit code
                Ok(Output {
                    #[cfg(unix)]
                    status: std::process::ExitStatus::from_raw(code),
                    #[cfg(windows)]
                    status: std::process::ExitStatus::from_raw(code as u32),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            }
        }
    }

    /// Execute a branch step (case/switch)
    fn execute_branch_step(
        step: &WorkflowStep,
        context: &mut WorkflowContext,
        results: &mut Vec<(String, Result<Output>)>,
    ) -> Result<Output> {
        // Branch steps must have a branch property
        let branch = step.branch.as_ref().ok_or_else(|| {
            ClixError::CommandExecutionFailed("Branch step missing branch property".to_string())
        })?;

        // Get the variable value to branch on
        let var_name = &branch.variable;
        let var_value = context.variables.get(var_name).cloned().unwrap_or_default();

        println!(
            "{} {} = {}",
            "Branching on:".blue().bold(),
            var_name,
            var_value
        );

        // Find the matching case
        let matching_case = branch.cases.iter().find(|case| case.value == var_value);

        let steps_to_execute = if let Some(case) = matching_case {
            println!("{} {}", "Matched case:".blue().bold(), case.value);
            &case.steps
        } else if let Some(default_steps) = &branch.default_case {
            println!("{}", "Using default case".blue().bold());
            default_steps
        } else {
            println!(
                "{}",
                "No matching case found and no default case".yellow().bold()
            );
            // Return a success output since we're not treating this as an error
            return Ok(Output {
                status: std::process::ExitStatus::from_raw(0),
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        };

        // Execute the steps in the selected case
        let mut last_step_output = None;

        for (index, step) in steps_to_execute.iter().enumerate() {
            println!(
                "\n{} {} - {}",
                "Branch Step".blue().bold(),
                (index + 1).to_string().blue().bold(),
                step.name
            );

            // Process variables in the step
            let processed_step = VariableProcessor::process_step(step, context);

            // Check if step requires approval
            if processed_step.require_approval {
                Self::request_approval(&processed_step)?;
            }

            // Execute the step
            let result = match processed_step.step_type {
                StepType::Command => Self::execute_command_step(&processed_step),
                StepType::Auth => Self::execute_auth_step(&processed_step),
                StepType::Conditional => Self::execute_conditional_step(
                    &processed_step,
                    &context.variables,
                    last_step_output.as_ref(),
                ),
                StepType::Branch => Self::execute_branch_step(&processed_step, context, results),
                StepType::Loop => Self::execute_loop_step(&processed_step, context, results),
            };

            // Update last_step_output if successful
            if let Ok(ref output) = result {
                last_step_output = Some(output.clone());
            }

            // Check if we need to continue
            let should_continue = match &result {
                Ok(_) => true,
                Err(_) => processed_step.continue_on_error,
            };

            // Store the result
            results.push((processed_step.name.clone(), result));

            if !should_continue {
                println!(
                    "{} Command failed, stopping branch execution",
                    "Error:".red().bold()
                );
                break;
            }
        }

        // Return the last output if we have one, or create a success output
        if let Some(output) = last_step_output {
            Ok(output)
        } else {
            Ok(Output {
                status: std::process::ExitStatus::from_raw(0),
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        }
    }

    /// Execute a loop step (while)
    fn execute_loop_step(
        step: &WorkflowStep,
        context: &mut WorkflowContext,
        results: &mut Vec<(String, Result<Output>)>,
    ) -> Result<Output> {
        // Loop steps must have a loop_data property
        let loop_data = step.loop_data.as_ref().ok_or_else(|| {
            ClixError::CommandExecutionFailed("Loop step missing loop_data property".to_string())
        })?;

        println!(
            "{} {}",
            "Loop condition:".blue().bold(),
            loop_data.condition.expression
        );

        // Create a counter to prevent infinite loops
        let max_iterations = 100; // Reasonable limit to prevent infinite loops
        let mut iterations = 0;
        let mut last_step_output = None;

        // Execute the loop until the condition becomes false or we hit max iterations
        while iterations < max_iterations {
            // Evaluate the loop condition
            let condition_result = ExpressionEvaluator::evaluate(
                &loop_data.condition.expression,
                &context.variables,
                last_step_output.as_ref(),
            )?;

            if !condition_result {
                println!("{}", "Loop condition is false, exiting loop".blue().bold());
                break;
            }

            println!("{} {}", "Loop iteration:".blue().bold(), iterations + 1);

            // Execute the steps in the loop
            for (index, step) in loop_data.steps.iter().enumerate() {
                println!(
                    "\n{} {}.{} - {}",
                    "Loop Step".blue().bold(),
                    iterations + 1,
                    index + 1,
                    step.name
                );

                // Process variables in the step
                let processed_step = VariableProcessor::process_step(step, context);

                // Check if step requires approval
                if processed_step.require_approval {
                    Self::request_approval(&processed_step)?;
                }

                // Execute the step
                let result = match processed_step.step_type {
                    StepType::Command => Self::execute_command_step(&processed_step),
                    StepType::Auth => Self::execute_auth_step(&processed_step),
                    StepType::Conditional => Self::execute_conditional_step(
                        &processed_step,
                        &context.variables,
                        last_step_output.as_ref(),
                    ),
                    StepType::Branch => {
                        Self::execute_branch_step(&processed_step, context, results)
                    }
                    StepType::Loop => Self::execute_loop_step(&processed_step, context, results),
                };

                // Update last_step_output if successful
                if let Ok(ref output) = result {
                    last_step_output = Some(output.clone());
                }

                // Check if we need to continue
                let should_continue = match &result {
                    Ok(_) => true,
                    Err(_) => processed_step.continue_on_error,
                };

                // Store the result
                results.push((
                    format!("Loop[{}].{}", iterations + 1, processed_step.name),
                    result,
                ));

                if !should_continue {
                    println!(
                        "{} Command failed, stopping loop execution",
                        "Error:".red().bold()
                    );
                    break;
                }
            }

            iterations += 1;
        }

        if iterations >= max_iterations {
            println!(
                "{}",
                "Loop reached maximum iterations, stopping".yellow().bold()
            );
        }

        // Return the last output if we have one, or create a success output
        if let Some(output) = last_step_output {
            Ok(output)
        } else {
            Ok(Output {
                status: std::process::ExitStatus::from_raw(0),
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        }
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

    /// Request approval from the user before executing a step
    fn request_approval(step: &WorkflowStep) -> Result<()> {
        println!(
            "{}",
            "⚠️  This step requires approval before execution:"
                .yellow()
                .bold()
        );
        println!("{} {}", "Name:".blue().bold(), step.name);
        println!("{} {}", "Description:".blue().bold(), step.description);

        if !step.command.is_empty() {
            println!("{} {}", "Command:".blue().bold(), step.command);
        }

        print!("{} [y/N]: ", "Do you want to proceed?".yellow().bold());
        io::stdout().flush().map_err(|e| {
            ClixError::CommandExecutionFailed(format!("Failed to flush stdout: {}", e))
        })?;

        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut input = String::new();

        handle.read_line(&mut input).map_err(|e| {
            ClixError::CommandExecutionFailed(format!("Failed to read approval input: {}", e))
        })?;

        let input = input.trim().to_lowercase();
        if input == "y" || input == "yes" {
            println!("{}", "Proceeding with step execution.".green());
            Ok(())
        } else {
            Err(ClixError::CommandExecutionFailed(
                "Step execution canceled by user".to_string(),
            ))
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
