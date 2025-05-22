use crate::commands::models::{StepType, Workflow, WorkflowStep};
use crate::error::Result;
use crate::storage::Storage;
use regex::Regex;
use std::collections::{HashMap, HashSet, VecDeque};

pub struct WorkflowValidator {
    storage: Storage,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub message: String,
    pub step_name: Option<String>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub workflow_name: String,
    pub is_valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub dependency_graph: HashMap<String, Vec<String>>,
}

impl WorkflowValidator {
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }

    /// Validate a single workflow comprehensively
    pub fn validate_workflow(&self, workflow: &Workflow) -> Result<ValidationReport> {
        let mut issues = Vec::new();
        let mut dependency_graph = HashMap::new();

        // Check for circular dependencies
        self.check_circular_dependencies(workflow, &mut issues, &mut dependency_graph)?;

        // Check for unreachable steps
        self.check_unreachable_steps(workflow, &mut issues);

        // Validate variable consistency
        self.validate_variables(workflow, &mut issues);

        // Check step naming and descriptions
        self.validate_step_metadata(workflow, &mut issues);

        // Check for infinite loops
        self.check_infinite_loops(workflow, &mut issues);

        // Check for duplicate step names
        self.check_duplicate_step_names(workflow, &mut issues);

        // Validate command syntax
        self.validate_command_syntax(workflow, &mut issues);

        let is_valid = !issues.iter().any(|issue| issue.severity == Severity::Error);

        Ok(ValidationReport {
            workflow_name: workflow.name.clone(),
            is_valid,
            issues,
            dependency_graph,
        })
    }

    /// Check for circular dependencies in workflow calls
    fn check_circular_dependencies(
        &self,
        workflow: &Workflow,
        issues: &mut Vec<ValidationIssue>,
        dependency_graph: &mut HashMap<String, Vec<String>>,
    ) -> Result<()> {
        // Build dependency graph
        let workflow_calls = self.extract_all_workflow_calls(workflow)?;
        dependency_graph.insert(workflow.name.clone(), workflow_calls.clone());

        // Check for direct self-reference
        if workflow_calls.contains(&workflow.name) {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                message: format!("Workflow '{}' calls itself directly", workflow.name),
                step_name: None,
                suggestion: Some("Remove the self-referencing call or add a condition to prevent infinite recursion".to_string()),
            });
        }

        // Check for indirect circular dependencies
        for called_workflow in &workflow_calls {
            if let Ok(called_wf) = self.storage.get_workflow(called_workflow) {
                if self.has_circular_dependency_to(
                    &called_wf,
                    &workflow.name,
                    &mut HashSet::new(),
                )? {
                    issues.push(ValidationIssue {
                        severity: Severity::Error,
                        message: format!(
                            "Circular dependency detected: '{}' -> '{}' -> ... -> '{}'",
                            workflow.name, called_workflow, workflow.name
                        ),
                        step_name: None,
                        suggestion: Some(
                            "Restructure workflows to eliminate circular calls".to_string(),
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    /// Extract all workflow calls from a workflow (recursive through all steps)
    fn extract_all_workflow_calls(&self, workflow: &Workflow) -> Result<Vec<String>> {
        let mut calls = Vec::new();
        let workflow_call_regex = Regex::new(r"clix\s+flow\s+run\s+([\w-]+)").unwrap();

        for step in &workflow.steps {
            self.extract_workflow_calls_from_step(step, &workflow_call_regex, &mut calls);
        }

        calls.sort();
        calls.dedup();
        Ok(calls)
    }

    /// Extract workflow calls from a single step (handles nested structures)
    #[allow(clippy::only_used_in_recursion)]
    fn extract_workflow_calls_from_step(
        &self,
        step: &WorkflowStep,
        regex: &Regex,
        calls: &mut Vec<String>,
    ) {
        // Check main command
        if !step.command.is_empty() {
            for captures in regex.captures_iter(&step.command) {
                if let Some(workflow_name) = captures.get(1) {
                    calls.push(workflow_name.as_str().to_string());
                }
            }
        }

        // Check conditional blocks
        if let Some(conditional) = &step.conditional {
            for then_step in &conditional.then_block.steps {
                self.extract_workflow_calls_from_step(then_step, regex, calls);
            }
            if let Some(else_block) = &conditional.else_block {
                for else_step in &else_block.steps {
                    self.extract_workflow_calls_from_step(else_step, regex, calls);
                }
            }
        }

        // Check branch blocks
        if let Some(branch) = &step.branch {
            for case in &branch.cases {
                for case_step in &case.steps {
                    self.extract_workflow_calls_from_step(case_step, regex, calls);
                }
            }
            if let Some(default_steps) = &branch.default_case {
                for default_step in default_steps {
                    self.extract_workflow_calls_from_step(default_step, regex, calls);
                }
            }
        }

        // Check loop blocks
        if let Some(loop_data) = &step.loop_data {
            for loop_step in &loop_data.steps {
                self.extract_workflow_calls_from_step(loop_step, regex, calls);
            }
        }
    }

    /// Check if a workflow has a circular dependency to a target workflow
    fn has_circular_dependency_to(
        &self,
        workflow: &Workflow,
        target: &str,
        visited: &mut HashSet<String>,
    ) -> Result<bool> {
        if visited.contains(&workflow.name) {
            return Ok(false); // Already checked this path
        }

        visited.insert(workflow.name.clone());

        let calls = self.extract_all_workflow_calls(workflow)?;

        if calls.contains(&target.to_string()) {
            return Ok(true);
        }

        for called_workflow_name in calls {
            if let Ok(called_workflow) = self.storage.get_workflow(&called_workflow_name) {
                if self.has_circular_dependency_to(&called_workflow, target, visited)? {
                    return Ok(true);
                }
            }
        }

        visited.remove(&workflow.name);
        Ok(false)
    }

    /// Check for unreachable steps in the workflow
    fn check_unreachable_steps(&self, workflow: &Workflow, issues: &mut Vec<ValidationIssue>) {
        let reachable = self.find_reachable_steps(workflow);

        for (index, step) in workflow.steps.iter().enumerate() {
            if !reachable.contains(&index) {
                issues.push(ValidationIssue {
                    severity: Severity::Warning,
                    message: format!("Step '{}' may be unreachable", step.name),
                    step_name: Some(step.name.clone()),
                    suggestion: Some(
                        "Check if this step can be reached through normal execution flow"
                            .to_string(),
                    ),
                });
            }
        }
    }

    /// Find all reachable steps using graph traversal
    fn find_reachable_steps(&self, workflow: &Workflow) -> HashSet<usize> {
        let mut reachable = HashSet::new();
        let mut to_visit = VecDeque::new();

        // Start from the first step
        if !workflow.steps.is_empty() {
            to_visit.push_back(0);
        }

        while let Some(step_index) = to_visit.pop_front() {
            if reachable.contains(&step_index) {
                continue;
            }

            reachable.insert(step_index);

            if let Some(step) = workflow.steps.get(step_index) {
                // Normal flow continues to next step
                if step_index + 1 < workflow.steps.len() {
                    to_visit.push_back(step_index + 1);
                }

                // Check conditional flows
                if let Some(conditional) = &step.conditional {
                    // Then block steps are reachable
                    for then_step in &conditional.then_block.steps {
                        if let Some(idx) = self.find_step_index(workflow, &then_step.name) {
                            to_visit.push_back(idx);
                        }
                    }
                    // Else block steps are reachable
                    if let Some(else_block) = &conditional.else_block {
                        for else_step in &else_block.steps {
                            if let Some(idx) = self.find_step_index(workflow, &else_step.name) {
                                to_visit.push_back(idx);
                            }
                        }
                    }
                }

                // Check branch flows
                if let Some(branch) = &step.branch {
                    for case in &branch.cases {
                        for case_step in &case.steps {
                            if let Some(idx) = self.find_step_index(workflow, &case_step.name) {
                                to_visit.push_back(idx);
                            }
                        }
                    }
                    if let Some(default_steps) = &branch.default_case {
                        for default_step in default_steps {
                            if let Some(idx) = self.find_step_index(workflow, &default_step.name) {
                                to_visit.push_back(idx);
                            }
                        }
                    }
                }
            }
        }

        reachable
    }

    /// Find the index of a step by name
    fn find_step_index(&self, workflow: &Workflow, step_name: &str) -> Option<usize> {
        workflow
            .steps
            .iter()
            .position(|step| step.name == step_name)
    }

    /// Validate variable consistency throughout the workflow
    fn validate_variables(&self, workflow: &Workflow, issues: &mut Vec<ValidationIssue>) {
        let mut defined_vars = HashSet::new();
        let mut used_vars = HashSet::new();

        // Collect defined variables
        for var in &workflow.variables {
            defined_vars.insert(var.name.clone());
        }

        // Collect used variables from all steps
        for step in &workflow.steps {
            self.collect_used_variables_from_step(step, &mut used_vars);
        }

        // Check for undefined variables
        for used_var in &used_vars {
            if !defined_vars.contains(used_var) && !self.is_builtin_variable(used_var) {
                issues.push(ValidationIssue {
                    severity: Severity::Warning,
                    message: format!("Variable '{}' is used but not defined", used_var),
                    step_name: None,
                    suggestion: Some(format!("Add variable '{}' to workflow variables", used_var)),
                });
            }
        }

        // Check for unused variables
        for defined_var in &defined_vars {
            if !used_vars.contains(defined_var) {
                issues.push(ValidationIssue {
                    severity: Severity::Info,
                    message: format!("Variable '{}' is defined but never used", defined_var),
                    step_name: None,
                    suggestion: Some("Consider removing unused variables".to_string()),
                });
            }
        }
    }

    /// Collect used variables from a step and its nested structures
    #[allow(clippy::only_used_in_recursion)]
    fn collect_used_variables_from_step(
        &self,
        step: &WorkflowStep,
        used_vars: &mut HashSet<String>,
    ) {
        let var_regex = Regex::new(r"\$\{(\w+)\}|\$(\w+)").unwrap();

        // Check main command
        for captures in var_regex.captures_iter(&step.command) {
            if let Some(var_name) = captures.get(1).or(captures.get(2)) {
                used_vars.insert(var_name.as_str().to_string());
            }
        }

        // Check conditional blocks
        if let Some(conditional) = &step.conditional {
            for captures in var_regex.captures_iter(&conditional.condition.expression) {
                if let Some(var_name) = captures.get(1).or(captures.get(2)) {
                    used_vars.insert(var_name.as_str().to_string());
                }
            }

            for then_step in &conditional.then_block.steps {
                self.collect_used_variables_from_step(then_step, used_vars);
            }

            if let Some(else_block) = &conditional.else_block {
                for else_step in &else_block.steps {
                    self.collect_used_variables_from_step(else_step, used_vars);
                }
            }
        }

        // Check branch blocks
        if let Some(branch) = &step.branch {
            used_vars.insert(branch.variable.clone());

            for case in &branch.cases {
                for case_step in &case.steps {
                    self.collect_used_variables_from_step(case_step, used_vars);
                }
            }

            if let Some(default_steps) = &branch.default_case {
                for default_step in default_steps {
                    self.collect_used_variables_from_step(default_step, used_vars);
                }
            }
        }

        // Check loop blocks
        if let Some(loop_data) = &step.loop_data {
            for captures in var_regex.captures_iter(&loop_data.condition.expression) {
                if let Some(var_name) = captures.get(1).or(captures.get(2)) {
                    used_vars.insert(var_name.as_str().to_string());
                }
            }

            for loop_step in &loop_data.steps {
                self.collect_used_variables_from_step(loop_step, used_vars);
            }
        }
    }

    /// Check if a variable is a built-in system variable
    fn is_builtin_variable(&self, var_name: &str) -> bool {
        matches!(
            var_name,
            "HOME" | "USER" | "PATH" | "PWD" | "SHELL" | "TERM"
        )
    }

    /// Validate step metadata (names, descriptions)
    fn validate_step_metadata(&self, workflow: &Workflow, issues: &mut Vec<ValidationIssue>) {
        for step in &workflow.steps {
            if step.name.trim().is_empty() {
                issues.push(ValidationIssue {
                    severity: Severity::Error,
                    message: "Step has empty name".to_string(),
                    step_name: None,
                    suggestion: Some("Provide a meaningful name for the step".to_string()),
                });
            }

            if step.description.trim().is_empty() {
                issues.push(ValidationIssue {
                    severity: Severity::Warning,
                    message: format!("Step '{}' has empty description", step.name),
                    step_name: Some(step.name.clone()),
                    suggestion: Some(
                        "Add a description to explain what this step does".to_string(),
                    ),
                });
            }

            if step.name.len() > 100 {
                issues.push(ValidationIssue {
                    severity: Severity::Warning,
                    message: format!("Step '{}' has very long name", step.name),
                    step_name: Some(step.name.clone()),
                    suggestion: Some("Consider using a shorter, more concise name".to_string()),
                });
            }
        }
    }

    /// Check for potential infinite loops
    fn check_infinite_loops(&self, workflow: &Workflow, issues: &mut Vec<ValidationIssue>) {
        for step in &workflow.steps {
            if let Some(loop_data) = &step.loop_data {
                // Check for obvious infinite loop conditions
                if loop_data.condition.expression == "true" || loop_data.condition.expression == "1"
                {
                    issues.push(ValidationIssue {
                        severity: Severity::Error,
                        message: format!(
                            "Step '{}' contains an infinite loop condition",
                            step.name
                        ),
                        step_name: Some(step.name.clone()),
                        suggestion: Some("Add a proper exit condition to the loop".to_string()),
                    });
                }

                // Check if loop modifies its condition variable
                if let Some(var_name) = &loop_data.condition.variable {
                    let mut modifies_condition = false;
                    for loop_step in &loop_data.steps {
                        if self.step_modifies_variable(loop_step, var_name) {
                            modifies_condition = true;
                            break;
                        }
                    }

                    if !modifies_condition {
                        issues.push(ValidationIssue {
                            severity: Severity::Warning,
                            message: format!(
                                "Loop in step '{}' may not modify its condition variable '{}'",
                                step.name, var_name
                            ),
                            step_name: Some(step.name.clone()),
                            suggestion: Some("Ensure the loop modifies the condition variable to eventually exit".to_string()),
                        });
                    }
                }
            }
        }
    }

    /// Check if a step modifies a specific variable
    fn step_modifies_variable(&self, step: &WorkflowStep, var_name: &str) -> bool {
        let assignment_patterns = [
            format!("{}=", var_name),
            format!("export {}=", var_name),
            format!("local {}=", var_name),
            format!("declare {}=", var_name),
        ];

        for pattern in &assignment_patterns {
            if step.command.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Check for duplicate step names
    fn check_duplicate_step_names(&self, workflow: &Workflow, issues: &mut Vec<ValidationIssue>) {
        let mut step_names = HashMap::new();

        for (index, step) in workflow.steps.iter().enumerate() {
            if let Some(first_index) = step_names.get(&step.name) {
                issues.push(ValidationIssue {
                    severity: Severity::Error,
                    message: format!(
                        "Duplicate step name '{}' found at positions {} and {}",
                        step.name,
                        first_index + 1,
                        index + 1
                    ),
                    step_name: Some(step.name.clone()),
                    suggestion: Some("Use unique names for all steps".to_string()),
                });
            } else {
                step_names.insert(step.name.clone(), index);
            }
        }
    }

    /// Validate command syntax for basic issues
    fn validate_command_syntax(&self, workflow: &Workflow, issues: &mut Vec<ValidationIssue>) {
        for step in &workflow.steps {
            if step.step_type == StepType::Command && !step.command.trim().is_empty() {
                // Check for unmatched quotes
                if self.has_unmatched_quotes(&step.command) {
                    issues.push(ValidationIssue {
                        severity: Severity::Error,
                        message: format!("Step '{}' has unmatched quotes", step.name),
                        step_name: Some(step.name.clone()),
                        suggestion: Some("Check that all quotes are properly matched".to_string()),
                    });
                }

                // Check for suspicious patterns
                if step.command.contains("rm -rf /") {
                    issues.push(ValidationIssue {
                        severity: Severity::Warning,
                        message: format!(
                            "Step '{}' contains potentially dangerous command",
                            step.name
                        ),
                        step_name: Some(step.name.clone()),
                        suggestion: Some("Review this command carefully for safety".to_string()),
                    });
                }
            }
        }
    }

    /// Check for unmatched quotes in a command
    fn has_unmatched_quotes(&self, command: &str) -> bool {
        let mut single_quote_count = 0;
        let mut double_quote_count = 0;
        let mut escaped = false;

        for ch in command.chars() {
            if escaped {
                escaped = false;
                continue;
            }

            match ch {
                '\\' => escaped = true,
                '\'' => single_quote_count += 1,
                '"' => double_quote_count += 1,
                _ => {}
            }
        }

        single_quote_count % 2 != 0 || double_quote_count % 2 != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::models::{WorkflowStep, WorkflowVariable};
    use tempfile::tempdir;

    #[test]
    fn test_circular_dependency_detection() {
        let _dir = tempdir().unwrap();
        let storage = Storage::new().unwrap();
        let validator = WorkflowValidator::new(storage);

        let steps = vec![WorkflowStep::new_command(
            "Call self".to_string(),
            "clix flow run test-workflow".to_string(),
            "This calls itself".to_string(),
            false,
        )];

        let workflow = Workflow::new(
            "test-workflow".to_string(),
            "Test workflow".to_string(),
            steps,
            vec![],
        );

        let report = validator.validate_workflow(&workflow).unwrap();
        assert!(!report.is_valid);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.severity == Severity::Error
                    && issue.message.contains("calls itself directly"))
        );
    }

    #[test]
    fn test_duplicate_step_names() {
        let storage = Storage::new().unwrap();
        let validator = WorkflowValidator::new(storage);

        let steps = vec![
            WorkflowStep::new_command(
                "duplicate".to_string(),
                "echo 'first'".to_string(),
                "First step".to_string(),
                false,
            ),
            WorkflowStep::new_command(
                "duplicate".to_string(),
                "echo 'second'".to_string(),
                "Second step".to_string(),
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
        assert!(!report.is_valid);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.severity == Severity::Error
                    && issue.message.contains("Duplicate step name"))
        );
    }

    #[test]
    fn test_variable_validation() {
        let storage = Storage::new().unwrap();
        let validator = WorkflowValidator::new(storage);

        let steps = vec![WorkflowStep::new_command(
            "Use undefined var".to_string(),
            "echo $UNDEFINED_VAR".to_string(),
            "Uses undefined variable".to_string(),
            false,
        )];

        let variables = vec![WorkflowVariable::new(
            "DEFINED_VAR".to_string(),
            "A defined variable".to_string(),
            Some("default".to_string()),
            false,
        )];

        let workflow = Workflow::with_variables(
            "test-workflow".to_string(),
            "Test workflow".to_string(),
            steps,
            vec![],
            variables,
        );

        let report = validator.validate_workflow(&workflow).unwrap();

        // Should have warning about undefined variable and info about unused variable
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.severity == Severity::Warning
                    && issue.message.contains("UNDEFINED_VAR"))
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.severity == Severity::Info
                    && issue.message.contains("DEFINED_VAR"))
        );
    }
}
