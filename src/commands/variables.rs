use crate::commands::models::{
    BranchCase, BranchStep, Condition, ConditionalBlock, ConditionalStep, LoopStep, Workflow,
    WorkflowStep,
};
use crate::error::{ClixError, Result};
use colored::Colorize;
use regex::Regex;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

#[derive(Debug, Clone, Default)]
pub struct WorkflowContext {
    pub variables: HashMap<String, String>,
}

impl WorkflowContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_variable(&mut self, name: String, value: String) {
        self.variables.insert(name, value);
    }

    pub fn merge_variables(&mut self, vars: HashMap<String, String>) {
        for (key, value) in vars {
            self.variables.insert(key, value);
        }
    }

    pub fn has_variable(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }
}

pub struct VariableProcessor;

impl VariableProcessor {
    /// Process variables in a command string, replacing {{ var_name }} with values
    pub fn process_variables(command: &str, context: &WorkflowContext) -> String {
        let re = Regex::new(r"\{\{\s*([\w_]+)\s*\}\}").unwrap();
        let mut result = command.to_string();

        for cap in re.captures_iter(command) {
            let var_name = &cap[1];
            let placeholder = &cap[0];

            if let Some(value) = context.variables.get(var_name) {
                result = result.replace(placeholder, value);
            }
        }

        result
    }

    /// Extract variable names from a command string
    pub fn extract_variables(command: &str) -> Vec<String> {
        let re = Regex::new(r"\{\{\s*([\w_]+)\s*\}\}").unwrap();
        let mut vars = Vec::new();

        for cap in re.captures_iter(command) {
            let var_name = cap[1].to_string();
            if !vars.contains(&var_name) {
                vars.push(var_name);
            }
        }

        vars
    }

    /// Scan the workflow for all variables in commands
    pub fn scan_workflow_variables(workflow: &Workflow) -> Vec<String> {
        let mut vars = Vec::new();

        for step in &workflow.steps {
            let step_vars = Self::extract_variables(&step.command);
            for var in step_vars {
                if !vars.contains(&var) {
                    vars.push(var);
                }
            }
        }

        vars
    }

    /// Prompt the user for any missing variables
    pub fn prompt_for_variables(workflow: &Workflow, context: &mut WorkflowContext) -> Result<()> {
        // Get all variables used in the workflow
        let all_vars = Self::scan_workflow_variables(workflow);

        // Check for variables that are used but not defined in the workflow
        for var_name in &all_vars {
            // Skip if variable is already set in context
            if context.variables.contains_key(var_name) {
                continue;
            }

            // Find variable definition if it exists
            let var_def = workflow.variables.iter().find(|v| &v.name == var_name);

            let description = var_def.map_or_else(
                || format!("Value for {}", var_name),
                |v| v.description.clone(),
            );

            let default = var_def.and_then(|v| v.default_value.clone());

            // Prompt for variable value
            println!("{} {}", "Variable:".blue().bold(), var_name);
            println!("{} {}", "Description:".blue(), description);

            if let Some(ref default_value) = default {
                print!("{} [{}]: ", "Enter value".yellow(), default_value);
            } else {
                print!("{}: ", "Enter value".yellow());
            }

            io::stdout().flush().map_err(|e| {
                ClixError::CommandExecutionFailed(format!("Failed to flush stdout: {}", e))
            })?;

            let stdin = io::stdin();
            let mut handle = stdin.lock();
            let mut input = String::new();

            handle.read_line(&mut input).map_err(|e| {
                ClixError::CommandExecutionFailed(format!("Failed to read variable input: {}", e))
            })?;

            // Trim newline
            let input = input.trim();

            let value = if input.is_empty() && default.is_some() {
                default.unwrap_or_default() // This should never panic since we checked is_some
            } else if input.is_empty() && var_def.is_some_and(|v| v.required) {
                return Err(ClixError::CommandExecutionFailed(format!(
                    "Variable '{}' is required",
                    var_name
                )));
            } else {
                input.to_string()
            };

            context.variables.insert(var_name.clone(), value);
        }

        Ok(())
    }

    /// Process all variables in a workflow step
    pub fn process_step(step: &WorkflowStep, context: &WorkflowContext) -> WorkflowStep {
        let processed_command = Self::process_variables(&step.command, context);

        // Process conditional expressions if they exist
        let processed_conditional = step.conditional.as_ref().map(|conditional| {
            let processed_condition = Condition {
                expression: Self::process_variables(&conditional.condition.expression, context),
                variable: conditional.condition.variable.clone(),
            };

            let processed_then_block = ConditionalBlock {
                steps: conditional
                    .then_block
                    .steps
                    .iter()
                    .map(|step| Self::process_step(step, context))
                    .collect(),
            };

            let processed_else_block =
                conditional
                    .else_block
                    .as_ref()
                    .map(|else_block| ConditionalBlock {
                        steps: else_block
                            .steps
                            .iter()
                            .map(|step| Self::process_step(step, context))
                            .collect(),
                    });

            ConditionalStep {
                condition: processed_condition,
                then_block: processed_then_block,
                else_block: processed_else_block,
                action: conditional.action.clone(),
            }
        });

        // Process branch if it exists
        let processed_branch = step.branch.as_ref().map(|branch| {
            let processed_cases = branch
                .cases
                .iter()
                .map(|case| BranchCase {
                    value: Self::process_variables(&case.value, context),
                    steps: case
                        .steps
                        .iter()
                        .map(|step| Self::process_step(step, context))
                        .collect(),
                })
                .collect();

            let processed_default_case = branch.default_case.as_ref().map(|default_case| {
                default_case
                    .iter()
                    .map(|step| Self::process_step(step, context))
                    .collect()
            });

            BranchStep {
                variable: branch.variable.clone(),
                cases: processed_cases,
                default_case: processed_default_case,
            }
        });

        // Process loop if it exists
        let processed_loop = step.loop_data.as_ref().map(|loop_data| {
            let processed_condition = Condition {
                expression: Self::process_variables(&loop_data.condition.expression, context),
                variable: loop_data.condition.variable.clone(),
            };

            let processed_steps = loop_data
                .steps
                .iter()
                .map(|step| Self::process_step(step, context))
                .collect();

            LoopStep {
                condition: processed_condition,
                steps: processed_steps,
            }
        });

        WorkflowStep {
            name: step.name.clone(),
            command: processed_command,
            description: step.description.clone(),
            continue_on_error: step.continue_on_error,
            step_type: step.step_type.clone(),
            require_approval: step.require_approval,
            conditional: processed_conditional,
            branch: processed_branch,
            loop_data: processed_loop,
        }
    }
}
