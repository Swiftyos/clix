use clix::commands::{StepType, Workflow, WorkflowStep};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn test_workflow_creation() {
    let name = "test-workflow".to_string();
    let description = "Test workflow description".to_string();
    let tags = vec!["test".to_string(), "workflow".to_string()];

    let steps = vec![
        WorkflowStep::new_command(
            "Step 1".to_string(),
            "echo 'Step 1'".to_string(),
            "First step".to_string(),
            false,
        ),
        WorkflowStep::new_command(
            "Step 2".to_string(),
            "echo 'Step 2'".to_string(),
            "Second step".to_string(),
            true,
        ),
    ];

    let workflow = Workflow::new(
        name.clone(),
        description.clone(),
        steps.clone(),
        tags.clone(),
    );

    assert_eq!(workflow.name, name);
    assert_eq!(workflow.description, description);
    assert_eq!(workflow.steps.len(), steps.len());
    assert_eq!(workflow.steps[0].name, steps[0].name);
    assert_eq!(workflow.steps[1].command, steps[1].command);
    assert_eq!(workflow.tags, tags);
    assert_eq!(workflow.use_count, 0);
    assert!(workflow.last_used.is_none());

    // Ensure created_at is reasonably close to now
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    assert!(workflow.created_at <= now);
    assert!(workflow.created_at >= now - 10); // Allow 10 seconds leeway
}

#[test]
fn test_workflow_usage_tracking() {
    let steps = vec![WorkflowStep::new_command(
        "Test Step".to_string(),
        "echo 'Test'".to_string(),
        "Test step".to_string(),
        false,
    )];

    let workflow = Workflow::new(
        "usage-test".to_string(),
        "Test usage tracking".to_string(),
        steps,
        vec!["test".to_string()],
    );

    assert_eq!(workflow.use_count, 0);
    assert!(workflow.last_used.is_none());

    let mut workflow = workflow;
    workflow.mark_used();

    assert_eq!(workflow.use_count, 1);
    assert!(workflow.last_used.is_some());

    let first_usage = workflow.last_used.unwrap();
    workflow.mark_used();

    assert_eq!(workflow.use_count, 2);
    assert!(workflow.last_used.unwrap() >= first_usage);
}

#[test]
fn test_auth_step_creation() {
    let auth_step = WorkflowStep::new_auth(
        "Auth Step".to_string(),
        "gcloud auth login".to_string(),
        "Login to Google Cloud".to_string(),
    );

    assert_eq!(auth_step.name, "Auth Step");
    assert_eq!(auth_step.command, "gcloud auth login");
    assert_eq!(auth_step.description, "Login to Google Cloud");
    assert_eq!(auth_step.step_type, StepType::Auth);
    assert_eq!(auth_step.continue_on_error, false);

    let command_step = WorkflowStep::new_command(
        "Command Step".to_string(),
        "echo 'test'".to_string(),
        "Test command".to_string(),
        true,
    );

    assert_eq!(command_step.step_type, StepType::Command);
    assert_eq!(command_step.continue_on_error, true);
}
