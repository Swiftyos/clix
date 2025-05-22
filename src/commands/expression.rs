use crate::error::{ClixError, Result};
use regex::Regex;
use std::collections::HashMap;
use std::process::{Command, Output};

pub struct ExpressionEvaluator;

impl ExpressionEvaluator {
    /// Evaluates a conditional expression in a shell-like syntax
    /// Supports basic comparison operators, file tests, and logical operators
    pub fn evaluate(
        expr: &str,
        context: &HashMap<String, String>,
        last_output: Option<&Output>,
    ) -> Result<bool> {
        // Replace variables in the expression
        let expr_with_vars = Self::replace_variables(expr, context);

        // Check for common shell test patterns
        if Self::is_exit_code_check(&expr_with_vars) {
            return Self::evaluate_exit_code(&expr_with_vars, last_output);
        } else if Self::is_file_test(&expr_with_vars) {
            return Self::evaluate_file_test(&expr_with_vars);
        } else if Self::is_string_test(&expr_with_vars) {
            return Self::evaluate_string_test(&expr_with_vars);
        }

        // For complex expressions, execute them with a shell
        // This delegates the expression evaluation to the shell
        Self::execute_as_shell_test(&expr_with_vars)
    }

    /// Replace variables in an expression with their values from the context
    fn replace_variables(expr: &str, context: &HashMap<String, String>) -> String {
        let mut result = expr.to_string();

        // Replace ${var} and $var style variables
        let re_braces = Regex::new(r"\$\{([A-Za-z0-9_]+)\}").unwrap();
        let re_simple = Regex::new(r"\$([A-Za-z0-9_]+)").unwrap();

        // First replace ${var} style
        for cap in re_braces.captures_iter(expr) {
            let var_name = &cap[1];
            let placeholder = &cap[0];

            if let Some(value) = context.get(var_name) {
                result = result.replace(placeholder, value);
            }
        }

        // Then replace $var style (but not $? which is a special case for exit code)
        // We need to create a temporary copy of result to avoid borrowing issues
        let temp_result = result.clone();
        for cap in re_simple.captures_iter(&temp_result) {
            let var_name = &cap[1];
            let placeholder = &cap[0];

            if placeholder != "$?" {
                if let Some(value) = context.get(var_name) {
                    result = result.replace(placeholder, value);
                }
            }
        }

        result
    }

    /// Check if the expression is testing an exit code ($? -eq 0)
    fn is_exit_code_check(expr: &str) -> bool {
        let re =
            Regex::new(r"^\s*\$\?\s*(-eq|-ne|-gt|-lt|-ge|-le|==|!=|>|<|>=|<=)\s*\d+\s*$").unwrap();
        re.is_match(expr)
    }

    /// Check if the expression is a file test ([ -f file ] or [[ -d dir ]])
    fn is_file_test(expr: &str) -> bool {
        let re = Regex::new(r"^\s*(\[|\[\[)\s*-[fderwxs]\s+.+\s*(\]|\]\])\s*$").unwrap();
        re.is_match(expr)
    }

    /// Check if the expression is a string test ([ -z "$var" ] or [[ -n "$var" ]])
    fn is_string_test(expr: &str) -> bool {
        let re = Regex::new(r"^\s*(\[|\[\[)\s*(-z|-n)\s+.+\s*(\]|\]\])\s*$").unwrap();
        re.is_match(expr)
    }

    /// Evaluate an exit code check expression
    fn evaluate_exit_code(expr: &str, last_output: Option<&Output>) -> Result<bool> {
        // Extract the comparison operator and the expected exit code
        let re = Regex::new(r"^\s*\$\?\s*(-eq|-ne|-gt|-lt|-ge|-le|==|!=|>|<|>=|<=)\s*(\d+)\s*$")
            .unwrap();
        let caps = re.captures(expr).ok_or_else(|| {
            ClixError::CommandExecutionFailed("Invalid exit code expression format".to_string())
        })?;

        let operator = &caps[1];
        let expected_code: i32 = caps[2].parse().map_err(|e| {
            ClixError::CommandExecutionFailed(format!("Invalid exit code number: {}", e))
        })?;

        // Get the actual exit code from the last output
        let actual_code = match last_output {
            Some(output) => output.status.code().unwrap_or(0),
            None => {
                return Err(ClixError::CommandExecutionFailed(
                    "No previous command output available for $? evaluation".to_string(),
                ));
            }
        };

        // Compare the exit codes
        match operator {
            "-eq" | "==" => Ok(actual_code == expected_code),
            "-ne" | "!=" => Ok(actual_code != expected_code),
            "-gt" | ">" => Ok(actual_code > expected_code),
            "-lt" | "<" => Ok(actual_code < expected_code),
            "-ge" | ">=" => Ok(actual_code >= expected_code),
            "-le" | "<=" => Ok(actual_code <= expected_code),
            _ => Err(ClixError::CommandExecutionFailed(format!(
                "Unsupported operator: {}",
                operator
            ))),
        }
    }

    /// Evaluate a file test expression
    fn evaluate_file_test(expr: &str) -> Result<bool> {
        // Just delegate to the shell test command since file tests are complex
        Self::execute_as_shell_test(expr)
    }

    /// Evaluate a string test expression
    fn evaluate_string_test(expr: &str) -> Result<bool> {
        // Just delegate to the shell test command since string tests can be complex
        Self::execute_as_shell_test(expr)
    }

    /// Execute the expression as a shell test and return true if it succeeds
    fn execute_as_shell_test(expr: &str) -> Result<bool> {
        let result = if cfg!(target_os = "windows") {
            // Windows doesn't have a test command, so we need to use PowerShell
            // to evaluate the condition
            Command::new("powershell")
                .args([
                    "-Command",
                    &format!("if ({}) {{ exit 0 }} else {{ exit 1 }}", expr),
                ])
                .status()
        } else {
            // On Unix-like systems, we can use bash to evaluate the condition
            Command::new("bash")
                .args(["-c", &format!("if {}; then exit 0; else exit 1; fi", expr)])
                .status()
        };

        match result {
            Ok(status) => Ok(status.success()),
            Err(e) => Err(ClixError::CommandExecutionFailed(format!(
                "Failed to evaluate expression '{}': {}",
                expr, e
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_replace_variables() {
        let mut context = HashMap::new();
        context.insert("FOO".to_string(), "bar".to_string());
        context.insert("NUM".to_string(), "42".to_string());

        let expr = "test $FOO = bar && ${NUM} -eq 42";
        let result = ExpressionEvaluator::replace_variables(expr, &context);
        assert_eq!(result, "test bar = bar && 42 -eq 42");
    }

    #[test]
    fn test_is_exit_code_check() {
        assert!(ExpressionEvaluator::is_exit_code_check("$? -eq 0"));
        assert!(ExpressionEvaluator::is_exit_code_check("$? != 1"));
        assert!(!ExpressionEvaluator::is_exit_code_check("test -f file.txt"));
    }

    #[test]
    fn test_is_file_test() {
        assert!(ExpressionEvaluator::is_file_test("[ -f file.txt ]"));
        assert!(ExpressionEvaluator::is_file_test("[[ -d /tmp ]]"));
        assert!(!ExpressionEvaluator::is_file_test("$? -eq 0"));
    }

    #[test]
    fn test_is_string_test() {
        assert!(ExpressionEvaluator::is_string_test("[ -z \"$var\" ]"));
        assert!(ExpressionEvaluator::is_string_test("[[ -n \"$NAME\" ]]"));
        assert!(!ExpressionEvaluator::is_string_test("$? -eq 0"));
    }
}
