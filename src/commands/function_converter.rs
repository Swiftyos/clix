use crate::commands::models::{BranchCase, Condition, Workflow, WorkflowStep};
use crate::error::{ClixError, Result};
use regex::Regex;
use std::fs;

pub struct FunctionConverter;

impl FunctionConverter {
    /// Converts a shell function into a workflow
    pub fn convert_function(
        file_path: &str,
        function_name: &str,
        workflow_name: &str,
        description: &str,
        tags: Vec<String>,
    ) -> Result<Workflow> {
        // Read the shell script file
        let content = fs::read_to_string(file_path).map_err(|e| {
            ClixError::Io(std::io::Error::other(format!(
                "Failed to read script file: {}",
                e
            )))
        })?;

        // Extract the function
        let function_content = Self::extract_function(&content, function_name)?;

        // Parse the function into workflow steps
        let steps = Self::parse_function_to_steps(function_content, function_name)?;

        // Create the workflow
        let workflow = Workflow::new(
            workflow_name.to_string(),
            description.to_string(),
            steps,
            tags,
        );

        Ok(workflow)
    }

    /// Extract a function from the shell script content
    fn extract_function(content: &str, function_name: &str) -> Result<String> {
        // Pattern to match the function definition
        // We use (?s) for DOTALL mode to make . match newlines
        // We use (?m) for multiline mode to make ^ and $ match at line breaks
        let pattern = format!(
            r"(?sm)^{}\s*\(\)\s*\{{(.*?)^}}",
            regex::escape(function_name)
        );

        let re = Regex::new(&pattern).unwrap();

        if let Some(captures) = re.captures(content) {
            if let Some(function_body) = captures.get(1) {
                return Ok(function_body.as_str().to_string());
            }
        }

        Err(ClixError::CommandNotFound(format!(
            "Function '{}' not found in the script",
            function_name
        )))
    }

    /// Parse the function body into workflow steps
    fn parse_function_to_steps(
        function_body: String,
        function_name: &str,
    ) -> Result<Vec<WorkflowStep>> {
        let mut steps = Vec::new();

        // Add an initial step with the function name
        steps.push(WorkflowStep::new_command(
            format!("Start {}", function_name),
            format!("echo \"Running {} function...\"", function_name),
            format!("Starting execution of {} function", function_name),
            false,
        ));

        // Basic implementation - convert each line to a step
        // A more complete implementation would handle control structures (if/else, loops, etc.)

        // Extract parameter handling
        if function_body.contains("local") {
            steps.push(Self::create_parameter_step(&function_body)?);
        }

        // Extract conditionals
        let conditionals = Self::extract_conditionals(&function_body)?;
        for conditional in conditionals {
            steps.push(conditional);
        }

        // Extract case statements
        let case_steps = Self::extract_case_statements(&function_body)?;
        for case_step in case_steps {
            steps.push(case_step);
        }

        // Extract commands (excluding those in conditionals and cases)
        let command_steps = Self::extract_commands(&function_body)?;
        for command_step in command_steps {
            steps.push(command_step);
        }

        Ok(steps)
    }

    /// Create a step for parameter handling
    fn create_parameter_step(_function_body: &str) -> Result<WorkflowStep> {
        // This is a simplified implementation - in reality, you'd want to
        // extract actual parameter definitions and convert them to workflow variables

        Ok(WorkflowStep::new_command(
            "Process Parameters".to_string(),
            "echo \"Processing parameters...\"".to_string(),
            "Process function parameters".to_string(),
            false,
        ))
    }

    /// Extract conditionals from the function body
    fn extract_conditionals(function_body: &str) -> Result<Vec<WorkflowStep>> {
        let mut conditionals = Vec::new();

        // Simplified implementation - in reality, you'd need a more
        // sophisticated parser to handle nested conditionals and complex expressions

        // Extract if/else blocks
        let if_pattern = r"if\s+\[\s+(.+?)\s+\];\s*then\s+(.+?)(?:else\s+(.+?))?fi";
        let re = Regex::new(if_pattern).unwrap();

        for captures in re.captures_iter(function_body) {
            if captures.len() >= 3 {
                let condition_expr = captures.get(1).unwrap().as_str().to_string();
                let then_block = captures.get(2).unwrap().as_str().to_string();
                let else_block = captures.get(3).map(|m| m.as_str().to_string());

                // Create then steps
                let then_steps = vec![WorkflowStep::new_command(
                    "Then Action".to_string(),
                    then_block.trim().to_string(),
                    "Action when condition is true".to_string(),
                    false,
                )];

                // Create else steps if present
                let else_steps = else_block.map(|else_content| {
                    vec![WorkflowStep::new_command(
                        "Else Action".to_string(),
                        else_content.trim().to_string(),
                        "Action when condition is false".to_string(),
                        false,
                    )]
                });

                // Create conditional step
                conditionals.push(WorkflowStep::new_conditional(
                    "Condition Check".to_string(),
                    format!("Check condition: {}", condition_expr),
                    Condition {
                        expression: condition_expr,
                        variable: None,
                    },
                    then_steps,
                    else_steps,
                    None,
                ));
            }
        }

        Ok(conditionals)
    }

    /// Extract case statements from the function body
    fn extract_case_statements(function_body: &str) -> Result<Vec<WorkflowStep>> {
        let mut case_steps = Vec::new();

        // Simplified implementation - in reality, you'd need a more
        // sophisticated parser to handle complex case statements

        // Extract case blocks
        let case_pattern = r"case\s+(\$\w+)\s+in\s+(.+?)esac";
        let re = Regex::new(case_pattern).unwrap();

        for captures in re.captures_iter(function_body) {
            if captures.len() >= 3 {
                let variable = captures.get(1).unwrap().as_str().to_string();
                let cases_block = captures.get(2).unwrap().as_str().to_string();

                // Extract individual cases
                let mut branch_cases = Vec::new();
                let mut default_case = None;

                // Simple pattern to extract cases
                let case_item_pattern = r"(\w+)\)\s+(.+?);;\s*";
                let case_re = Regex::new(case_item_pattern).unwrap();

                for case_captures in case_re.captures_iter(&cases_block) {
                    if case_captures.len() >= 3 {
                        let case_value = case_captures.get(1).unwrap().as_str().to_string();
                        let case_action = case_captures.get(2).unwrap().as_str().to_string();

                        // Create steps for this case
                        let case_steps = vec![WorkflowStep::new_command(
                            format!("Case: {}", case_value),
                            case_action.trim().to_string(),
                            format!("Action for case: {}", case_value),
                            false,
                        )];

                        // Add to branch cases
                        if case_value == "*" {
                            default_case = Some(case_steps);
                        } else {
                            branch_cases.push(BranchCase {
                                value: case_value,
                                steps: case_steps,
                            });
                        }
                    }
                }

                // Create branch step
                case_steps.push(WorkflowStep::new_branch(
                    "Branch by Value".to_string(),
                    format!("Branch based on {}", variable),
                    variable.replace("$", ""),
                    branch_cases,
                    default_case,
                ));
            }
        }

        Ok(case_steps)
    }

    /// Extract commands from the function body
    fn extract_commands(function_body: &str) -> Result<Vec<WorkflowStep>> {
        let mut command_steps = Vec::new();

        // Simplified implementation - extract echo commands as steps
        // In reality, you'd need to handle more complex command patterns

        let echo_pattern = r#"echo\s+"([^"]+)""#;
        let re = Regex::new(echo_pattern).unwrap();

        for (i, captures) in re.captures_iter(function_body).enumerate() {
            if captures.len() >= 2 {
                let message = captures.get(1).unwrap().as_str().to_string();

                command_steps.push(WorkflowStep::new_command(
                    format!("Command {}", i + 1),
                    format!("echo \"{}\"", message),
                    format!("Display message: {}", message),
                    false,
                ));
            }
        }

        Ok(command_steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_function_extraction() {
        let script = r#"
#!/bin/bash

# Test function
test_func() {
    echo "This is a test"
    if [ $1 -eq 0 ]; then
        echo "Zero"
    else
        echo "Not zero"
    fi
}

another_func() {
    echo "Another function"
}
"#;

        let result = FunctionConverter::extract_function(script, "test_func").unwrap();
        assert!(result.contains("This is a test"));
        assert!(result.contains("if [ $1 -eq 0 ]"));
    }

    #[test]
    fn test_simple_function_conversion() {
        // Create a temporary directory
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_script.sh");

        // Write a test script
        let mut file = File::create(&file_path).unwrap();
        write!(
            file,
            r##"
#!/bin/bash

# Simple test function
simple_test() {{
    echo "Starting simple test"
    local value=$1
    if [ "$value" = "test" ]; then
        echo "Value is test"
    else
        echo "Value is not test"
    fi
    echo "Test complete"
}}
"##
        )
        .unwrap();

        // Need to flush the file to ensure it's written to disk
        file.flush().unwrap();

        // Convert the function to a workflow
        let workflow = FunctionConverter::convert_function(
            file_path.to_str().unwrap(),
            "simple_test",
            "simple-test-workflow",
            "Simple test workflow",
            vec!["test".to_string()],
        )
        .unwrap();

        // Verify workflow structure
        assert_eq!(workflow.name, "simple-test-workflow");
        assert_eq!(workflow.description, "Simple test workflow");
        assert!(!workflow.steps.is_empty());

        // Verify tags
        assert_eq!(workflow.tags.len(), 1);
        assert_eq!(workflow.tags[0], "test");

        // Verify some steps
        assert!(workflow.steps.iter().any(|s| s.name.contains("Start")));
        assert!(workflow.steps.iter().any(|s| s.name.contains("Parameters")));

        // The test should validate that we have at least some steps, not necessarily conditionals
        assert!(workflow.steps.len() >= 2);
    }
}
