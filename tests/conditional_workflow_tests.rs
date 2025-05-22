use clix::commands::{BranchCase, Condition, ConditionalAction, StepType, Workflow, WorkflowStep};

#[test]
fn test_conditional_step_creation() {
    // Create condition
    let condition = Condition {
        expression: "$? -eq 0".to_string(),
        variable: None,
    };

    // Create then steps
    let then_steps = vec![WorkflowStep::new_command(
        "Success step".to_string(),
        "echo 'Command succeeded'".to_string(),
        "Print success message".to_string(),
        false,
    )];

    // Create else steps
    let else_steps = vec![WorkflowStep::new_command(
        "Failure step".to_string(),
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
    assert_eq!(conditional.then_block.steps[0].name, "Success step");
    assert!(conditional.else_block.is_some());
    assert_eq!(conditional.else_block.unwrap().steps.len(), 1);
    assert!(conditional.action.is_none());
}

#[test]
fn test_branch_step_creation() {
    // Create cases
    let cases = vec![
        BranchCase {
            value: "dev".to_string(),
            steps: vec![WorkflowStep::new_command(
                "Dev setup".to_string(),
                "echo 'Setting up dev environment'".to_string(),
                "Setup dev environment".to_string(),
                false,
            )],
        },
        BranchCase {
            value: "prod".to_string(),
            steps: vec![WorkflowStep::new_command(
                "Prod setup".to_string(),
                "echo 'Setting up prod environment'".to_string(),
                "Setup prod environment".to_string(),
                false,
            )],
        },
    ];

    // Create default case
    let default_case = vec![WorkflowStep::new_command(
        "Default setup".to_string(),
        "echo 'Unknown environment'".to_string(),
        "Handle unknown environment".to_string(),
        false,
    )];

    // Create branch step
    let step = WorkflowStep::new_branch(
        "Environment setup".to_string(),
        "Setup environment based on ENV variable".to_string(),
        "ENV".to_string(),
        cases,
        Some(default_case),
    );

    // Assert step properties
    assert_eq!(step.name, "Environment setup");
    assert_eq!(step.description, "Setup environment based on ENV variable");
    assert_eq!(step.step_type, StepType::Branch);
    assert!(step.branch.is_some());

    // Assert branch properties
    let branch = step.branch.unwrap();
    assert_eq!(branch.variable, "ENV");
    assert_eq!(branch.cases.len(), 2);
    assert_eq!(branch.cases[0].value, "dev");
    assert_eq!(branch.cases[1].value, "prod");
    assert!(branch.default_case.is_some());
    assert_eq!(branch.default_case.as_ref().unwrap().len(), 1);
}

#[test]
fn test_complex_conditional_workflow() {
    // Create a workflow that resembles the gke function

    // Authentication check conditional step
    let auth_check_condition = Condition {
        expression: "! gcloud auth application-default print-access-token > /dev/null 2>&1"
            .to_string(),
        variable: None,
    };

    let auth_steps = vec![
        WorkflowStep::new_auth(
            "Login to GCloud".to_string(),
            "gcloud auth login".to_string(),
            "Log in to Google Cloud".to_string(),
        ),
        WorkflowStep::new_auth(
            "Setup Default Credentials".to_string(),
            "gcloud auth application-default login".to_string(),
            "Set up application default credentials".to_string(),
        ),
    ];

    let auth_check_step = WorkflowStep::new_conditional(
        "Check Authentication".to_string(),
        "Check if already authenticated with GCloud".to_string(),
        auth_check_condition,
        auth_steps,
        None,
        None,
    );

    // Environment selection branch step
    let dev_steps = vec![
        WorkflowStep::new_command(
            "Set Dev Project".to_string(),
            "gcloud config set project dev-project".to_string(),
            "Set project to development".to_string(),
            false,
        ),
        WorkflowStep::new_command(
            "Get Dev Credentials".to_string(),
            "gcloud container clusters get-credentials dev-gke-cluster --zone=us-central1-a"
                .to_string(),
            "Get credentials for dev cluster".to_string(),
            false,
        ),
        WorkflowStep::new_command(
            "Set Dev Namespace".to_string(),
            "kubectl config set-context --current --namespace=dev-namespace".to_string(),
            "Set namespace to dev-namespace".to_string(),
            false,
        ),
    ];

    let prod_steps = vec![
        WorkflowStep::new_command(
            "Set Prod Project".to_string(),
            "gcloud config set project prod-project".to_string(),
            "Set project to production".to_string(),
            false,
        ),
        WorkflowStep::new_command(
            "Get Prod Credentials".to_string(),
            "gcloud container clusters get-credentials prod-gke-cluster --zone=us-central1-a"
                .to_string(),
            "Get credentials for prod cluster".to_string(),
            false,
        ),
        WorkflowStep::new_command(
            "Set Prod Namespace".to_string(),
            "kubectl config set-context --current --namespace=prod-namespace".to_string(),
            "Set namespace to prod-namespace".to_string(),
            false,
        ),
    ];

    let default_steps = vec![
        WorkflowStep::new_command(
            "Show Usage".to_string(),
            "echo \"Usage: gke [dev|prod]\"".to_string(),
            "Show usage message".to_string(),
            false,
        ),
        WorkflowStep::new_conditional(
            "Exit".to_string(),
            "Exit with error code".to_string(),
            Condition {
                expression: "true".to_string(),
                variable: None,
            },
            vec![],
            None,
            Some(ConditionalAction::Return(1)),
        ),
    ];

    let env_cases = vec![
        BranchCase {
            value: "dev".to_string(),
            steps: dev_steps,
        },
        BranchCase {
            value: "prod".to_string(),
            steps: prod_steps,
        },
    ];

    let env_branch_step = WorkflowStep::new_branch(
        "Set Environment".to_string(),
        "Configure environment based on parameter".to_string(),
        "env".to_string(),
        env_cases,
        Some(default_steps),
    );

    // Create the workflow
    let workflow = Workflow::with_variables(
        "gke".to_string(),
        "Switch to a GKE cluster environment".to_string(),
        vec![auth_check_step, env_branch_step],
        vec!["gcloud".to_string(), "kubernetes".to_string()],
        vec![clix::commands::WorkflowVariable::new(
            "env".to_string(),
            "Environment to switch to (dev or prod)".to_string(),
            None,
            true,
        )],
    );

    // Assert workflow properties
    assert_eq!(workflow.name, "gke");
    assert_eq!(workflow.steps.len(), 2);
    assert_eq!(workflow.steps[0].step_type, StepType::Conditional);
    assert_eq!(workflow.steps[1].step_type, StepType::Branch);
    assert_eq!(workflow.variables.len(), 1);
    assert_eq!(workflow.variables[0].name, "env");
}
