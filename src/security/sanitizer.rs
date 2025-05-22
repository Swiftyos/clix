use crate::error::{ClixError, Result};
use regex::Regex;

pub struct CommandSanitizer;

impl CommandSanitizer {
    /// Sanitize a command string by removing or escaping dangerous elements
    pub fn sanitize_command(command: &str) -> Result<String> {
        let mut sanitized = command.to_string();

        // Remove null bytes
        sanitized = sanitized.replace('\0', "");

        // Escape dangerous shell metacharacters if they're not properly quoted
        sanitized = Self::escape_shell_metacharacters(&sanitized)?;

        // Remove excessive whitespace
        sanitized = Self::normalize_whitespace(&sanitized);

        // Validate length
        if sanitized.len() > 2000 {
            return Err(ClixError::SecurityError(
                "Command too long after sanitization".to_string(),
            ));
        }

        Ok(sanitized)
    }

    /// Escape shell metacharacters that could be dangerous
    fn escape_shell_metacharacters(command: &str) -> Result<String> {
        let mut result = String::new();
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut chars = command.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '\'' if !in_double_quote => {
                    in_single_quote = !in_single_quote;
                    result.push(ch);
                }
                '"' if !in_single_quote => {
                    in_double_quote = !in_double_quote;
                    result.push(ch);
                }
                // Dangerous characters that should be escaped if not in quotes
                ';' | '|' | '&' | '$' | '`' | '(' | ')' | '<' | '>' if !in_single_quote && !in_double_quote => {
                    // Only escape if it looks suspicious (not part of legitimate command)
                    if Self::is_suspicious_metachar(ch, &mut chars) {
                        result.push('\\');
                    }
                    result.push(ch);
                }
                _ => result.push(ch),
            }
        }

        Ok(result)
    }

    /// Check if a metacharacter usage looks suspicious
    fn is_suspicious_metachar(ch: char, chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
        match ch {
            ';' => {
                // Multiple semicolons in a row are suspicious
                chars.peek() == Some(&';')
            }
            '|' => {
                // Pipe to shell commands is suspicious
                if let Some(&next_ch) = chars.peek() {
                    next_ch == ' ' || next_ch.is_alphabetic()
                } else {
                    false
                }
            }
            '&' => {
                // Background processes can be suspicious
                chars.peek() != Some(&'&') // Allow && but flag single &
            }
            '$' => {
                // Command substitution can be dangerous
                chars.peek() == Some(&'(')
            }
            '`' => true, // Backticks are almost always for command substitution
            '<' | '>' => {
                // Redirection to devices or important files
                false // Let the validator handle this
            }
            _ => false,
        }
    }

    /// Normalize whitespace in commands
    fn normalize_whitespace(command: &str) -> String {
        // Replace multiple whitespace with single spaces
        let re = Regex::new(r"\s+").unwrap();
        re.replace_all(command, " ").trim().to_string()
    }

    /// Sanitize variable names to prevent injection
    pub fn sanitize_variable_name(name: &str) -> Result<String> {
        // Variable names should only contain alphanumeric characters and underscores
        let re = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();
        
        if !re.is_match(name) {
            return Err(ClixError::SecurityError(format!(
                "Invalid variable name: {}. Variable names must start with a letter or underscore and contain only alphanumeric characters and underscores.",
                name
            )));
        }

        if name.len() > 64 {
            return Err(ClixError::SecurityError(
                "Variable name too long".to_string(),
            ));
        }

        Ok(name.to_string())
    }

    /// Sanitize variable values to prevent injection
    pub fn sanitize_variable_value(value: &str) -> Result<String> {
        let mut sanitized = value.to_string();

        // Remove null bytes
        sanitized = sanitized.replace('\0', "");

        // Escape newlines to prevent command injection
        sanitized = sanitized.replace('\n', "\\n");
        sanitized = sanitized.replace('\r', "\\r");

        // Limit length
        if sanitized.len() > 1024 {
            return Err(ClixError::SecurityError(
                "Variable value too long".to_string(),
            ));
        }

        Ok(sanitized)
    }

    /// Sanitize file paths to prevent directory traversal
    pub fn sanitize_file_path(path: &str) -> Result<String> {
        let mut sanitized = path.to_string();

        // Remove null bytes
        sanitized = sanitized.replace('\0', "");

        // Check for directory traversal attempts
        if sanitized.contains("..") {
            return Err(ClixError::SecurityError(
                "Path contains directory traversal sequences".to_string(),
            ));
        }

        // Check for absolute paths to sensitive directories
        let sensitive_prefixes = [
            "/etc/",
            "/var/",
            "/sys/",
            "/proc/",
            "/dev/",
            "/boot/",
            "/root/",
        ];

        for prefix in &sensitive_prefixes {
            if sanitized.starts_with(prefix) {
                return Err(ClixError::SecurityError(format!(
                    "Access to sensitive directory not allowed: {}",
                    prefix
                )));
            }
        }

        // Normalize path separators
        sanitized = sanitized.replace("//", "/");

        // Limit length
        if sanitized.len() > 256 {
            return Err(ClixError::SecurityError(
                "File path too long".to_string(),
            ));
        }

        Ok(sanitized)
    }

    /// Remove comments and potential code injection from user input
    pub fn sanitize_user_input(input: &str) -> Result<String> {
        let mut sanitized = input.to_string();

        // Remove null bytes
        sanitized = sanitized.replace('\0', "");

        // Remove or escape potential script tags and similar
        sanitized = sanitized.replace("<script", "&lt;script");
        sanitized = sanitized.replace("javascript:", "");
        sanitized = sanitized.replace("data:", "");

        // Remove excessive newlines
        let re = Regex::new(r"\n{3,}").unwrap();
        sanitized = re.replace_all(&sanitized, "\n\n").to_string();

        // Limit length
        if sanitized.len() > 2048 {
            return Err(ClixError::SecurityError(
                "Input too long".to_string(),
            ));
        }

        Ok(sanitized)
    }

    /// Validate and sanitize JSON input
    pub fn sanitize_json_input(json_str: &str) -> Result<String> {
        // Basic length check
        if json_str.len() > 10_000 {
            return Err(ClixError::SecurityError(
                "JSON input too large".to_string(),
            ));
        }

        // Try to parse JSON to ensure it's valid
        serde_json::from_str::<serde_json::Value>(json_str)
            .map_err(|e| ClixError::SecurityError(format!("Invalid JSON: {}", e)))?;

        // Check for potentially dangerous content in JSON strings
        if json_str.contains("javascript:") || json_str.contains("<script") {
            return Err(ClixError::SecurityError(
                "JSON contains potentially dangerous content".to_string(),
            ));
        }

        Ok(json_str.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_sanitization() {
        // Test basic sanitization
        let result = CommandSanitizer::sanitize_command("echo 'hello'   ").unwrap();
        assert_eq!(result, "echo 'hello'");

        // Test null byte removal
        let result = CommandSanitizer::sanitize_command("echo\0 'hello'").unwrap();
        assert_eq!(result, "echo 'hello'");
    }

    #[test]
    fn test_variable_name_sanitization() {
        // Valid names
        assert!(CommandSanitizer::sanitize_variable_name("valid_name").is_ok());
        assert!(CommandSanitizer::sanitize_variable_name("_private").is_ok());
        assert!(CommandSanitizer::sanitize_variable_name("var123").is_ok());

        // Invalid names
        assert!(CommandSanitizer::sanitize_variable_name("123invalid").is_err());
        assert!(CommandSanitizer::sanitize_variable_name("var-name").is_err());
        assert!(CommandSanitizer::sanitize_variable_name("var name").is_err());
    }

    #[test]
    fn test_path_sanitization() {
        // Safe paths
        assert!(CommandSanitizer::sanitize_file_path("./safe/path").is_ok());
        assert!(CommandSanitizer::sanitize_file_path("relative/path.txt").is_ok());

        // Dangerous paths
        assert!(CommandSanitizer::sanitize_file_path("../../../etc/passwd").is_err());
        assert!(CommandSanitizer::sanitize_file_path("/etc/passwd").is_err());
        assert!(CommandSanitizer::sanitize_file_path("/dev/sda").is_err());
    }

    #[test]
    fn test_json_sanitization() {
        // Valid JSON
        let valid_json = r#"{"key": "value", "number": 42}"#;
        assert!(CommandSanitizer::sanitize_json_input(valid_json).is_ok());

        // Invalid JSON
        let invalid_json = r#"{"key": "value",}"#;
        assert!(CommandSanitizer::sanitize_json_input(invalid_json).is_err());

        // Dangerous JSON
        let dangerous_json = r#"{"script": "<script>alert('xss')</script>"}"#;
        assert!(CommandSanitizer::sanitize_json_input(dangerous_json).is_err());
    }

    #[test]
    fn test_user_input_sanitization() {
        // Test script tag removal
        let input = "Hello <script>alert('xss')</script> World";
        let result = CommandSanitizer::sanitize_user_input(input).unwrap();
        assert!(result.contains("&lt;script"));

        // Test excessive newlines
        let input = "Line1\n\n\n\n\nLine2";
        let result = CommandSanitizer::sanitize_user_input(input).unwrap();
        assert_eq!(result, "Line1\n\nLine2");
    }
}