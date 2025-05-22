use clix::commands::{
    BranchCase, Condition, ConditionalAction, ExpressionEvaluator, StepType, Workflow, WorkflowStep,
};
use clix::error::Result;
use std::collections::HashMap;
use std::process::Command as ProcessCommand;

#[test]
fn test_expression_evaluation_exit_code() -> Result<()> {
    // Create a context with variables
    let mut context = HashMap::new();
    context.insert("TEST_VAR".to_string(), "test_value".to_string());

    // Create a simple shell command to run and capture its output
    let success_command = if cfg!(target_os = "windows") {
        ProcessCommand::new("cmd").args(["/C", "exit 0"]).output()
    } else {
        ProcessCommand::new("sh").args(["-c", "exit 0"]).output()
    }
    .expect("Failed to execute test command");

    let failure_command = if cfg!(target_os = "windows") {
        ProcessCommand::new("cmd").args(["/C", "exit 1"]).output()
    } else {
        ProcessCommand::new("sh").args(["-c", "exit 1"]).output()
    }
    .expect("Failed to execute test command");

    // Test exit code checks
    assert!(ExpressionEvaluator::evaluate(
        "$? -eq 0",
        &context,
        Some(&success_command)
    )?);
    assert!(!ExpressionEvaluator::evaluate(
        "$? -eq 0",
        &context,
        Some(&failure_command)
    )?);
    assert!(!ExpressionEvaluator::evaluate(
        "$? -ne 0",
        &context,
        Some(&success_command)
    )?);
    assert!(ExpressionEvaluator::evaluate(
        "$? -ne 0",
        &context,
        Some(&failure_command)
    )?);

    Ok(())
}

#[test]
fn test_expression_evaluation_variables() -> Result<()> {
    // Create a context with variables
    let mut context = HashMap::new();
    context.insert("ENV".to_string(), "dev".to_string());
    context.insert("DEBUG".to_string(), "true".to_string());

    // Test variable substitution and comparison
    assert!(ExpressionEvaluator::evaluate(
        "[ \"$ENV\" = \"dev\" ]",
        &context,
        None
    )?);
    assert!(!ExpressionEvaluator::evaluate(
        "[ \"$ENV\" = \"prod\" ]",
        &context,
        None
    )?);

    // Test with bash syntax
    assert!(ExpressionEvaluator::evaluate(
        "[ \"${ENV}\" = \"dev\" ]",
        &context,
        None
    )?);
    assert!(!ExpressionEvaluator::evaluate(
        "[ \"${ENV}\" = \"prod\" ]",
        &context,
        None
    )?);

    Ok(())
}

#[test]
fn test_workflow_with_conditionals() {
    // Create a simple workflow with a conditional that depends on a variable
    let conditional_step = WorkflowStep::new_conditional(
        "Check Environment".to_string(),
        "Check if we're in development environment".to_string(),
        Condition {
            expression: "[ \"$ENV\" = \"dev\" ]".to_string(),
            variable: None,
        },
        vec![WorkflowStep::new_command(
            "Dev Action".to_string(),
            "echo 'Running in development mode'".to_string(),
            "Action to take in dev mode".to_string(),
            false,
        )],
        Some(vec![WorkflowStep::new_command(
            "Non-Dev Action".to_string(),
            "echo 'Not in development mode'".to_string(),
            "Action to take in non-dev mode".to_string(),
            false,
        )]),
        None,
    );

    let workflow = Workflow::with_variables(
        "conditional_test".to_string(),
        "Test workflow with conditionals".to_string(),
        vec![conditional_step],
        vec!["test".to_string()],
        vec![clix::commands::WorkflowVariable::new(
            "ENV".to_string(),
            "Environment (dev, staging, prod)".to_string(),
            Some("dev".to_string()),
            true,
        )],
    );

    // Verify workflow structure
    assert_eq!(workflow.name, "conditional_test");
    assert_eq!(workflow.steps.len(), 1);
    assert_eq!(workflow.steps[0].step_type, StepType::Conditional);
    assert!(workflow.steps[0].conditional.is_some());
}

#[test]
fn test_workflow_with_branching() {
    // Create a workflow with branching based on a variable
    let branch_step = WorkflowStep::new_branch(
        "Process by Type".to_string(),
        "Handle different item types".to_string(),
        "ITEM_TYPE".to_string(),
        vec![
            BranchCase {
                value: "document".to_string(),
                steps: vec![WorkflowStep::new_command(
                    "Process Document".to_string(),
                    "echo 'Processing document'".to_string(),
                    "Handle document type".to_string(),
                    false,
                )],
            },
            BranchCase {
                value: "image".to_string(),
                steps: vec![WorkflowStep::new_command(
                    "Process Image".to_string(),
                    "echo 'Processing image'".to_string(),
                    "Handle image type".to_string(),
                    false,
                )],
            },
        ],
        Some(vec![WorkflowStep::new_command(
            "Unknown Type".to_string(),
            "echo 'Unknown item type'".to_string(),
            "Handle unknown type".to_string(),
            false,
        )]),
    );

    let workflow = Workflow::with_variables(
        "branch_test".to_string(),
        "Test workflow with branching".to_string(),
        vec![branch_step],
        vec!["test".to_string()],
        vec![clix::commands::WorkflowVariable::new(
            "ITEM_TYPE".to_string(),
            "Type of item to process (document, image)".to_string(),
            None,
            true,
        )],
    );

    // Verify workflow structure
    assert_eq!(workflow.name, "branch_test");
    assert_eq!(workflow.steps.len(), 1);
    assert_eq!(workflow.steps[0].step_type, StepType::Branch);
    assert!(workflow.steps[0].branch.is_some());

    let branch = workflow.steps[0].branch.as_ref().unwrap();
    assert_eq!(branch.variable, "ITEM_TYPE");
    assert_eq!(branch.cases.len(), 2);
    assert!(branch.default_case.is_some());
}

#[test]
fn test_conditional_action_return() {
    // Create a workflow that has a conditional with a return action
    let conditional_step = WorkflowStep::new_conditional(
        "Check Requirement".to_string(),
        "Check if a required tool is installed".to_string(),
        Condition {
            expression: "! command -v required-tool > /dev/null".to_string(),
            variable: None,
        },
        vec![WorkflowStep::new_command(
            "Show Error".to_string(),
            "echo 'Required tool is not installed'".to_string(),
            "Display error message".to_string(),
            false,
        )],
        None,
        Some(ConditionalAction::Return(1)),
    );

    let workflow = Workflow::new(
        "early_return_test".to_string(),
        "Test workflow with early return".to_string(),
        vec![
            conditional_step,
            WorkflowStep::new_command(
                "Main Task".to_string(),
                "echo 'Performing main task'".to_string(),
                "The main task of the workflow".to_string(),
                false,
            ),
        ],
        vec!["test".to_string()],
    );

    // Verify workflow structure
    assert_eq!(workflow.name, "early_return_test");
    assert_eq!(workflow.steps.len(), 2);
    assert_eq!(workflow.steps[0].step_type, StepType::Conditional);

    let conditional = workflow.steps[0].conditional.as_ref().unwrap();
    assert!(matches!(
        conditional.action,
        Some(ConditionalAction::Return(1))
    ));
}

// Add more integration tests as needed
