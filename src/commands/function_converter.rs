use crate::commands::models::{BranchCase, Condition, Workflow, WorkflowStep, WorkflowVariable};
use crate::error::{ClixError, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;

pub struct FunctionConverter;

pub struct ShellParser {
    variables: HashMap<String, String>,
}

pub struct AstBuilder;

#[derive(Debug, Clone)]
pub enum ShellStatement {
    Command(String),
    If {
        condition: String,
        then_block: Vec<ShellStatement>,
        else_block: Option<Vec<ShellStatement>>,
    },
    Case {
        variable: String,
        cases: Vec<CaseEntry>,
        default_case: Option<Vec<ShellStatement>>,
    },
    For {
        variable: String,
        items: String,
        body: Vec<ShellStatement>,
    },
    While {
        condition: String,
        body: Vec<ShellStatement>,
    },
    Function {
        name: String,
        body: Vec<ShellStatement>,
    },
    Variable {
        name: String,
        value: String,
        local: bool,
    },
}

#[derive(Debug, Clone)]
pub struct CaseEntry {
    pub pattern: String,
    pub commands: Vec<ShellStatement>,
}

impl Default for ShellParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellParser {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn parse_function(&mut self, content: &str) -> Result<Vec<ShellStatement>> {
        let mut statements = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            if line.is_empty() || line.starts_with('#') {
                i += 1;
                continue;
            }

            let (statement, consumed) = self.parse_statement(&lines, i)?;
            if let Some(stmt) = statement {
                statements.push(stmt);
            }
            i += consumed;
        }

        Ok(statements)
    }

    fn parse_statement(
        &mut self,
        lines: &[&str],
        start: usize,
    ) -> Result<(Option<ShellStatement>, usize)> {
        let line = lines[start].trim();

        // Parse if statements
        if line.starts_with("if ") {
            return self.parse_if_statement(lines, start);
        }

        // Parse case statements
        if line.starts_with("case ") {
            return self.parse_case_statement(lines, start);
        }

        // Parse for loops
        if line.starts_with("for ") {
            return self.parse_for_loop(lines, start);
        }

        // Parse while loops
        if line.starts_with("while ") {
            return self.parse_while_loop(lines, start);
        }

        // Parse variable assignments
        if line.contains('=') && !line.contains(' ') {
            return self.parse_variable_assignment(line);
        }

        // Parse local variable assignments
        if line.starts_with("local ") {
            return self.parse_local_variable(line);
        }

        // Default: treat as a command
        Ok((Some(ShellStatement::Command(line.to_string())), 1))
    }

    fn parse_if_statement(
        &mut self,
        lines: &[&str],
        start: usize,
    ) -> Result<(Option<ShellStatement>, usize)> {
        let mut i = start;
        let condition_line = lines[i].trim();

        // Extract condition from "if [condition]; then" or "if [condition]\nthen"
        let condition = if condition_line.ends_with("; then") {
            if let Some(stripped) = condition_line
                .strip_prefix("if ")
                .and_then(|s| s.strip_suffix("; then"))
            {
                stripped.trim().to_string()
            } else {
                return Err(ClixError::InvalidCommandFormat(
                    "Invalid if statement".to_string(),
                ));
            }
        } else if let Some(stripped) = condition_line.strip_prefix("if ") {
            // Look for "then" on next line
            i += 1;
            if i < lines.len() && lines[i].trim() == "then" {
                stripped.trim().to_string()
            } else {
                return Err(ClixError::InvalidCommandFormat(
                    "Malformed if statement".to_string(),
                ));
            }
        } else {
            return Err(ClixError::InvalidCommandFormat(
                "Invalid if statement".to_string(),
            ));
        };

        i += 1; // Move past the "then"

        let mut then_block = Vec::new();
        let mut else_block = None;

        // Parse then block
        while i < lines.len() {
            let line = lines[i].trim();

            if line == "else" {
                // Start of else block
                i += 1;
                let mut else_statements = Vec::new();
                while i < lines.len() {
                    let line = lines[i].trim();
                    if line == "fi" {
                        break;
                    }
                    let (stmt, consumed) = self.parse_statement(lines, i)?;
                    if let Some(statement) = stmt {
                        else_statements.push(statement);
                    }
                    i += consumed;
                }
                else_block = Some(else_statements);
                break;
            } else if line == "fi" {
                break;
            }

            let (stmt, consumed) = self.parse_statement(lines, i)?;
            if let Some(statement) = stmt {
                then_block.push(statement);
            }
            i += consumed;
        }

        // Skip the "fi"
        i += 1;

        Ok((
            Some(ShellStatement::If {
                condition,
                then_block,
                else_block,
            }),
            i - start,
        ))
    }

    fn parse_case_statement(
        &mut self,
        lines: &[&str],
        start: usize,
    ) -> Result<(Option<ShellStatement>, usize)> {
        let mut i = start;
        let case_line = lines[i].trim();

        // Extract variable from "case $var in" or "case "$var" in"
        let variable = if let Some(captures) = Regex::new(r#"case\s+"?(\$?\w+)"?\s+in"#)
            .unwrap()
            .captures(case_line)
        {
            captures.get(1).unwrap().as_str().replace("$", "")
        } else {
            return Err(ClixError::InvalidCommandFormat(
                "Invalid case statement".to_string(),
            ));
        };

        i += 1; // Move past "case ... in"

        let mut cases = Vec::new();
        let mut default_case = None;

        while i < lines.len() {
            let line = lines[i].trim();

            if line == "esac" {
                break;
            }

            // Parse case entry "pattern)"
            if let Some(pattern_str) = line.strip_suffix(')') {
                let pattern = pattern_str.trim().to_string();
                i += 1;

                let mut commands = Vec::new();
                while i < lines.len() {
                    let line = lines[i].trim();
                    if line == ";;" {
                        break;
                    }
                    if line == "esac" {
                        i -= 1; // Back up so outer loop catches esac
                        break;
                    }

                    let (stmt, consumed) = self.parse_statement(lines, i)?;
                    if let Some(statement) = stmt {
                        commands.push(statement);
                    }
                    i += consumed;
                }

                if pattern == "*" {
                    default_case = Some(commands);
                } else {
                    cases.push(CaseEntry { pattern, commands });
                }

                i += 1; // Skip ";;"
            } else {
                i += 1;
            }
        }

        i += 1; // Skip "esac"

        Ok((
            Some(ShellStatement::Case {
                variable,
                cases,
                default_case,
            }),
            i - start,
        ))
    }

    fn parse_for_loop(
        &mut self,
        lines: &[&str],
        start: usize,
    ) -> Result<(Option<ShellStatement>, usize)> {
        let mut i = start;
        let for_line = lines[i].trim();

        // Extract variable and items from "for var in items; do" or "for var in items"
        let (variable, items) = if let Some(captures) =
            Regex::new(r"for\s+(\w+)\s+in\s+(.+?)(?:;\s*do)?$")
                .unwrap()
                .captures(for_line)
        {
            (
                captures.get(1).unwrap().as_str().to_string(),
                captures.get(2).unwrap().as_str().to_string(),
            )
        } else {
            return Err(ClixError::InvalidCommandFormat(
                "Invalid for loop".to_string(),
            ));
        };

        // Check if "do" is on next line
        if !for_line.contains("do") {
            i += 1;
            if i < lines.len() && lines[i].trim() == "do" {
                // Good, continue
            } else {
                return Err(ClixError::InvalidCommandFormat(
                    "Missing 'do' in for loop".to_string(),
                ));
            }
        }

        i += 1; // Move past "do"

        let mut body = Vec::new();

        while i < lines.len() {
            let line = lines[i].trim();

            if line == "done" {
                break;
            }

            let (stmt, consumed) = self.parse_statement(lines, i)?;
            if let Some(statement) = stmt {
                body.push(statement);
            }
            i += consumed;
        }

        i += 1; // Skip "done"

        Ok((
            Some(ShellStatement::For {
                variable,
                items,
                body,
            }),
            i - start,
        ))
    }

    fn parse_while_loop(
        &mut self,
        lines: &[&str],
        start: usize,
    ) -> Result<(Option<ShellStatement>, usize)> {
        let mut i = start;
        let while_line = lines[i].trim();

        // Extract condition from "while [condition]; do" or "while [condition]"
        let condition = if while_line.ends_with("; do") {
            if let Some(stripped) = while_line.strip_suffix("; do") {
                stripped[6..].trim().to_string()
            } else {
                return Err(ClixError::InvalidCommandFormat(
                    "Invalid while loop".to_string(),
                ));
            }
        } else {
            i += 1;
            if i < lines.len() && lines[i].trim() == "do" {
                while_line[6..].trim().to_string()
            } else {
                return Err(ClixError::InvalidCommandFormat(
                    "Missing 'do' in while loop".to_string(),
                ));
            }
        };

        i += 1; // Move past "do"

        let mut body = Vec::new();

        while i < lines.len() {
            let line = lines[i].trim();

            if line == "done" {
                break;
            }

            let (stmt, consumed) = self.parse_statement(lines, i)?;
            if let Some(statement) = stmt {
                body.push(statement);
            }
            i += consumed;
        }

        i += 1; // Skip "done"

        Ok((Some(ShellStatement::While { condition, body }), i - start))
    }

    fn parse_variable_assignment(&mut self, line: &str) -> Result<(Option<ShellStatement>, usize)> {
        if let Some((name, value)) = line.split_once('=') {
            let name = name.trim().to_string();
            let value = value.trim().trim_matches('"').to_string();

            self.variables.insert(name.clone(), value.clone());

            Ok((
                Some(ShellStatement::Variable {
                    name,
                    value,
                    local: false,
                }),
                1,
            ))
        } else {
            Ok((Some(ShellStatement::Command(line.to_string())), 1))
        }
    }

    fn parse_local_variable(&mut self, line: &str) -> Result<(Option<ShellStatement>, usize)> {
        let var_part = if let Some(stripped) = line.strip_prefix("local ") {
            stripped.trim()
        } else {
            return Err(ClixError::InvalidCommandFormat(
                "Invalid local variable".to_string(),
            ));
        };

        if let Some((name, value)) = var_part.split_once('=') {
            let name = name.trim().to_string();
            let value = value.trim().trim_matches('"').to_string();

            self.variables.insert(name.clone(), value.clone());

            Ok((
                Some(ShellStatement::Variable {
                    name,
                    value,
                    local: true,
                }),
                1,
            ))
        } else {
            // Just a local declaration without assignment
            Ok((
                Some(ShellStatement::Variable {
                    name: var_part.to_string(),
                    value: String::new(),
                    local: true,
                }),
                1,
            ))
        }
    }
}

impl AstBuilder {
    #[allow(clippy::only_used_in_recursion)]
    pub fn build_steps(&self, statements: Vec<ShellStatement>) -> Result<Vec<WorkflowStep>> {
        let mut steps = Vec::new();

        for statement in statements {
            match statement {
                ShellStatement::Command(cmd) => {
                    steps.push(WorkflowStep::new_command(
                        format!("Execute: {}", Self::truncate_command(&cmd)),
                        cmd,
                        "Execute shell command".to_string(),
                        false,
                    ));
                }
                ShellStatement::If {
                    condition,
                    then_block,
                    else_block,
                } => {
                    let then_steps = self.build_steps(then_block)?;
                    let else_steps = if let Some(else_block) = else_block {
                        Some(self.build_steps(else_block)?)
                    } else {
                        None
                    };

                    steps.push(WorkflowStep::new_conditional(
                        "Conditional Check".to_string(),
                        format!("Check condition: {}", condition),
                        Condition {
                            expression: condition,
                            variable: None,
                        },
                        then_steps,
                        else_steps,
                        None,
                    ));
                }
                ShellStatement::Case {
                    variable,
                    cases,
                    default_case,
                } => {
                    let mut branch_cases = Vec::new();

                    for case_entry in cases {
                        let case_steps = self.build_steps(case_entry.commands)?;
                        branch_cases.push(BranchCase {
                            value: case_entry.pattern,
                            steps: case_steps,
                        });
                    }

                    let default_steps = if let Some(default_commands) = default_case {
                        Some(self.build_steps(default_commands)?)
                    } else {
                        None
                    };

                    steps.push(WorkflowStep::new_branch(
                        "Branch by Value".to_string(),
                        format!("Branch based on variable: {}", variable),
                        variable,
                        branch_cases,
                        default_steps,
                    ));
                }
                ShellStatement::For {
                    variable,
                    items,
                    body,
                } => {
                    let loop_body = self.build_steps(body)?;

                    // Convert for loop to while loop logic
                    steps.push(WorkflowStep::new_loop(
                        "For Loop".to_string(),
                        format!("Iterate {} over {}", variable, items),
                        Condition {
                            expression: format!("has_more_items({})", items),
                            variable: Some(variable),
                        },
                        loop_body,
                    ));
                }
                ShellStatement::While { condition, body } => {
                    let loop_body = self.build_steps(body)?;

                    steps.push(WorkflowStep::new_loop(
                        "While Loop".to_string(),
                        format!("Loop while: {}", condition),
                        Condition {
                            expression: condition,
                            variable: None,
                        },
                        loop_body,
                    ));
                }
                ShellStatement::Variable { name, value, local } => {
                    let scope = if local { "local" } else { "global" };
                    steps.push(WorkflowStep::new_command(
                        format!("Set {} variable: {}", scope, name),
                        if value.is_empty() {
                            format!("# Declare {} variable {}", scope, name)
                        } else {
                            format!(
                                "{}{}=\"{}\"",
                                if local { "local " } else { "" },
                                name,
                                value
                            )
                        },
                        format!(
                            "Set {} variable {} to {}",
                            scope,
                            name,
                            if value.is_empty() { "unset" } else { &value }
                        ),
                        false,
                    ));
                }
                ShellStatement::Function { .. } => {
                    // Skip nested functions for now
                }
            }
        }

        Ok(steps)
    }

    fn truncate_command(cmd: &str) -> String {
        if cmd.len() > 50 {
            format!("{}...", &cmd[..47])
        } else {
            cmd.to_string()
        }
    }
}

impl FunctionConverter {
    /// Converts a shell function into a workflow using advanced parsing
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

        // Use new advanced parser
        let steps = Self::convert_with_full_parsing(&function_content)?;

        // Extract variables from function parameters
        let variables = Self::extract_function_variables(&function_content)?;

        // Create the workflow with variables
        let workflow = Workflow::with_variables(
            workflow_name.to_string(),
            description.to_string(),
            steps,
            tags,
            variables,
        );

        Ok(workflow)
    }

    /// Convert function using full parsing with AST
    pub fn convert_with_full_parsing(function_content: &str) -> Result<Vec<WorkflowStep>> {
        let mut parser = ShellParser::new();
        let statements = parser.parse_function(function_content)?;

        let ast_builder = AstBuilder;
        ast_builder.build_steps(statements)
    }

    /// Extract function parameters as workflow variables
    fn extract_function_variables(function_content: &str) -> Result<Vec<WorkflowVariable>> {
        let mut variables = Vec::new();

        // Look for parameter references like $1, $2, etc.
        let param_regex = Regex::new(r"\$(\d+)").unwrap();
        let mut max_param = 0;

        for captures in param_regex.captures_iter(function_content) {
            if let Some(param_match) = captures.get(1) {
                if let Ok(param_num) = param_match.as_str().parse::<usize>() {
                    max_param = max_param.max(param_num);
                }
            }
        }

        // Create variables for each parameter
        for i in 1..=max_param {
            variables.push(WorkflowVariable::new(
                format!("param{}", i),
                format!("Function parameter ${}", i),
                None,
                true,
            ));
        }

        // Look for other variable references
        let var_regex = Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)").unwrap();
        let mut found_vars = std::collections::HashSet::new();

        for captures in var_regex.captures_iter(function_content) {
            if let Some(var_match) = captures.get(1) {
                let var_name = var_match.as_str();
                // Skip special variables and positional parameters
                if !var_name.chars().all(|c| c.is_ascii_digit())
                    && !["@", "*", "#", "?", "$", "!", "0"].contains(&var_name)
                    && found_vars.insert(var_name.to_string())
                {
                    variables.push(WorkflowVariable::new(
                        var_name.to_string(),
                        format!("Shell variable: {}", var_name),
                        None,
                        false,
                    ));
                }
            }
        }

        Ok(variables)
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

        // Verify some steps exist
        assert!(!workflow.steps.is_empty());

        // Verify that the new parser creates the expected steps
        assert!(workflow.steps.iter().any(|s| s.name.contains("echo")));
        assert!(
            workflow
                .steps
                .iter()
                .any(|s| s.name.contains("local variable"))
        );
        assert!(
            workflow
                .steps
                .iter()
                .any(|s| s.name.contains("Conditional"))
        );

        // The test should validate that we have at least 4 steps (echo, variable, conditional, echo)
        assert!(workflow.steps.len() >= 4);
    }
}
