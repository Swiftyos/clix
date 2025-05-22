use clix::commands::{Workflow, WorkflowStep};

#[test]
fn test_step_with_approval() {
    // Create a command step that requires approval
    let step = WorkflowStep::new_command(
        "Destructive Action".to_string(),
        "rm -rf /tmp/test".to_string(),
        "Remove test directory".to_string(),
        false,
    )
    .with_approval();

    // Verify the step has require_approval set to true
    assert!(step.require_approval);
}

#[test]
fn test_step_with_approval_constructor() {
    // Create a command step that requires approval using the dedicated constructor
    let step = WorkflowStep::new_command_with_approval(
        "Destructive Action".to_string(),
        "rm -rf /tmp/test".to_string(),
        "Remove test directory".to_string(),
        false,
    );

    // Verify the step has require_approval set to true
    assert!(step.require_approval);
}

#[test]
fn test_workflow_with_approval_steps() {
    // Create a workflow with steps that require approval
    let steps = vec![
        WorkflowStep::new_command(
            "Safe Step".to_string(),
            "echo 'This is safe'".to_string(),
            "A safe command".to_string(),
            false,
        ),
        WorkflowStep::new_command_with_approval(
            "Dangerous Step".to_string(),
            "rm -rf /tmp/test".to_string(),
            "A dangerous command".to_string(),
            false,
        ),
        WorkflowStep::new_command(
            "Another Safe Step".to_string(),
            "echo 'This is also safe'".to_string(),
            "Another safe command".to_string(),
            false,
        )
        .with_approval(), // Alternative way to set approval
    ];

    let workflow = Workflow::new(
        "approval_test".to_string(),
        "Test workflow with approval steps".to_string(),
        steps,
        vec!["test".to_string()],
    );

    // Verify the workflow structure
    assert_eq!(workflow.name, "approval_test");
    assert_eq!(workflow.steps.len(), 3);
    assert!(!workflow.steps[0].require_approval);
    assert!(workflow.steps[1].require_approval);
    assert!(workflow.steps[2].require_approval);
}

// Note: We can't easily test the actual approval flow in an automated test
// since it requires user input. This would be better tested manually or with
// a mock that simulates user input.
