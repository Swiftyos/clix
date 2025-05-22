use clix::commands::{BranchCase, Condition, StepType, WorkflowStep};

#[test]
fn test_conditional_with_approval_steps() {
    // Create a conditional step with approval in the then branch
    let condition = Condition {
        expression: "$? -eq 0".to_string(),
        variable: None,
    };

    // Create then steps with approval
    let then_steps = vec![WorkflowStep::new_command_with_approval(
        "Destructive step".to_string(),
        "rm -rf /tmp/test".to_string(),
        "Remove test directory on success".to_string(),
        false,
    )];

    // Create else steps
    let else_steps = vec![WorkflowStep::new_command(
        "Safe step".to_string(),
        "echo 'Command failed'".to_string(),
        "Print failure message".to_string(),
        false,
    )];

    // Create conditional step
    let step = WorkflowStep::new_conditional(
        "Check result".to_string(),
        "Check if previous command succeeded".to_string(),
        condition,
        then_steps,
        Some(else_steps),
        None,
    );

    // Assert step properties
    assert_eq!(step.name, "Check result");
    assert_eq!(step.description, "Check if previous command succeeded");
    assert_eq!(step.step_type, StepType::Conditional);
    assert!(step.conditional.is_some());

    // Assert conditional properties
    let conditional = step.conditional.unwrap();
    assert_eq!(conditional.condition.expression, "$? -eq 0");
    assert_eq!(conditional.then_block.steps.len(), 1);
    assert!(conditional.then_block.steps[0].require_approval);
    assert!(conditional.else_block.is_some());
    assert_eq!(conditional.else_block.as_ref().unwrap().steps.len(), 1);
    assert!(!conditional.else_block.unwrap().steps[0].require_approval);
}

#[test]
fn test_branch_with_approval_steps() {
    // Create a branch step with approval in one of the cases
    let cases = vec![
        BranchCase {
            value: "prod".to_string(),
            steps: vec![WorkflowStep::new_command_with_approval(
                "Prod deployment".to_string(),
                "kubectl apply -f k8s/prod/".to_string(),
                "Deploy to production".to_string(),
                false,
            )],
        },
        BranchCase {
            value: "dev".to_string(),
            steps: vec![WorkflowStep::new_command(
                "Dev deployment".to_string(),
                "kubectl apply -f k8s/dev/".to_string(),
                "Deploy to development".to_string(),
                false,
            )],
        },
    ];

    // Create branch step
    let step = WorkflowStep::new_branch(
        "Deploy by environment".to_string(),
        "Deploy to the selected environment".to_string(),
        "ENV".to_string(),
        cases,
        None,
    );

    // Assert step properties
    assert_eq!(step.name, "Deploy by environment");
    assert_eq!(step.step_type, StepType::Branch);
    assert!(step.branch.is_some());

    // Assert branch properties
    let branch = step.branch.unwrap();
    assert_eq!(branch.variable, "ENV");
    assert_eq!(branch.cases.len(), 2);

    // Check that the prod case has approval and dev case doesn't
    let prod_case = branch.cases.iter().find(|c| c.value == "prod").unwrap();
    let dev_case = branch.cases.iter().find(|c| c.value == "dev").unwrap();
    assert!(prod_case.steps[0].require_approval);
    assert!(!dev_case.steps[0].require_approval);
}

#[test]
fn test_loop_with_approval_steps() {
    // Create a loop step with approval in one of its steps
    let condition = Condition {
        expression: "[ $COUNTER -lt 3 ]".to_string(),
        variable: None,
    };

    let loop_steps = vec![
        WorkflowStep::new_command(
            "Increment counter".to_string(),
            "COUNTER=$((COUNTER+1))".to_string(),
            "Increment the counter variable".to_string(),
            false,
        ),
        WorkflowStep::new_command_with_approval(
            "Destructive operation".to_string(),
            "rm -f /tmp/test_$COUNTER".to_string(),
            "Remove a test file for the current iteration".to_string(),
            false,
        ),
    ];

    // Create loop step
    let step = WorkflowStep::new_loop(
        "Process items".to_string(),
        "Process multiple items with approval".to_string(),
        condition,
        loop_steps,
    );

    // Assert step properties
    assert_eq!(step.name, "Process items");
    assert_eq!(step.step_type, StepType::Loop);
    assert!(step.loop_data.is_some());

    // Assert loop properties
    let loop_data = step.loop_data.unwrap();
    assert_eq!(loop_data.condition.expression, "[ $COUNTER -lt 3 ]");
    assert_eq!(loop_data.steps.len(), 2);

    // Check that the first step doesn't require approval but the second does
    assert!(!loop_data.steps[0].require_approval);
    assert!(loop_data.steps[1].require_approval);
}
