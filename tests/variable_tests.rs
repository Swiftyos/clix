use clix::commands::variables::{VariableProcessor, WorkflowContext};
use clix::commands::{Workflow, WorkflowStep, WorkflowVariable, WorkflowVariableProfile};
use std::collections::HashMap;

#[test]
fn test_variable_extraction() {
    let command = "gcloud config set project {{ project_name }} and {{ another_var }}";
    let vars = VariableProcessor::extract_variables(command);

    assert_eq!(vars.len(), 2);
    assert!(vars.contains(&"project_name".to_string()));
    assert!(vars.contains(&"another_var".to_string()));
}

#[test]
fn test_variable_processing() {
    let command = "gcloud config set project {{ project_name }} --zone {{ zone }}";

    let mut context = WorkflowContext::new();
    context.add_variable("project_name".to_string(), "my-project".to_string());
    context.add_variable("zone".to_string(), "us-central1-a".to_string());

    let processed = VariableProcessor::process_variables(command, &context);
    assert_eq!(
        processed,
        "gcloud config set project my-project --zone us-central1-a"
    );
}

#[test]
fn test_workflow_variable_scanning() {
    let steps = vec![
        WorkflowStep::new_command(
            "Step 1".to_string(),
            "gcloud config set project {{ project_name }}".to_string(),
            "Set project".to_string(),
            false,
        ),
        WorkflowStep::new_command(
            "Step 2".to_string(),
            "gcloud container clusters get-credentials {{ cluster_name }} --zone={{ zone }}"
                .to_string(),
            "Get cluster credentials".to_string(),
            false,
        ),
    ];

    let workflow = Workflow::new(
        "test-workflow".to_string(),
        "Test workflow".to_string(),
        steps,
        vec!["test".to_string()],
    );

    let vars = VariableProcessor::scan_workflow_variables(&workflow);

    assert_eq!(vars.len(), 3);
    assert!(vars.contains(&"project_name".to_string()));
    assert!(vars.contains(&"cluster_name".to_string()));
    assert!(vars.contains(&"zone".to_string()));
}

#[test]
fn test_workflow_with_variables() {
    let mut workflow = Workflow::new(
        "test-workflow".to_string(),
        "Test workflow".to_string(),
        vec![WorkflowStep::new_command(
            "Step 1".to_string(),
            "gcloud config set project {{ project_name }}".to_string(),
            "Set project".to_string(),
            false,
        )],
        vec!["test".to_string()],
    );

    let variable = WorkflowVariable::new(
        "project_name".to_string(),
        "GCloud project ID".to_string(),
        Some("default-project".to_string()),
        true,
    );

    workflow.add_variable(variable);

    assert_eq!(workflow.variables.len(), 1);
    assert_eq!(workflow.variables[0].name, "project_name");
    assert_eq!(workflow.variables[0].description, "GCloud project ID");
    assert_eq!(
        workflow.variables[0].default_value,
        Some("default-project".to_string())
    );
    assert!(workflow.variables[0].required);
}

#[test]
fn test_workflow_with_profiles() {
    let mut workflow = Workflow::new(
        "test-workflow".to_string(),
        "Test workflow".to_string(),
        vec![WorkflowStep::new_command(
            "Step 1".to_string(),
            "gcloud config set project {{ project_name }}".to_string(),
            "Set project".to_string(),
            false,
        )],
        vec!["test".to_string()],
    );

    // Create a profile
    let mut vars = HashMap::new();
    vars.insert("project_name".to_string(), "prod-project".to_string());
    vars.insert("cluster_name".to_string(), "prod-cluster".to_string());

    let profile = WorkflowVariableProfile::new(
        "prod".to_string(),
        "Production environment".to_string(),
        vars,
    );

    workflow.add_profile(profile);

    assert_eq!(workflow.profiles.len(), 1);
    assert!(workflow.profiles.contains_key("prod"));

    let retrieved_profile = workflow.get_profile("prod").unwrap();
    assert_eq!(retrieved_profile.name, "prod");
    assert_eq!(retrieved_profile.description, "Production environment");
    assert_eq!(retrieved_profile.variables.len(), 2);
    assert_eq!(
        retrieved_profile.variables.get("project_name").unwrap(),
        "prod-project"
    );
    assert_eq!(
        retrieved_profile.variables.get("cluster_name").unwrap(),
        "prod-cluster"
    );
}
