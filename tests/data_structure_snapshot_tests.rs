use clix::commands::models::{BranchCase, Command, Condition, WorkflowStep, WorkflowVariable};
use clix::share::export::{ExportData, ExportMetadata};
use std::collections::HashMap;
use std::fs;

fn normalize_json(json: &str) -> String {
    // Parse and re-serialize to normalize formatting
    let value: serde_json::Value = serde_json::from_str(json).expect("Invalid JSON");
    serde_json::to_string_pretty(&value).expect("Failed to serialize")
}

#[test]
fn workflow_export_serialization_snapshot() {
    // Create a complex workflow with conditionals, branches, and variables
    let conditional_step = WorkflowStep::new_conditional(
        "Environment Check".to_string(),
        "Check if environment is valid".to_string(),
        Condition {
            expression: "[ \"$ENV\" = \"dev\" -o \"$ENV\" = \"staging\" ]".to_string(),
            variable: None,
        },
        vec![WorkflowStep::new_command(
            "Valid Environment".to_string(),
            "echo \"Valid environment: $ENV\"".to_string(),
            "Print valid environment".to_string(),
            false,
        )],
        Some(vec![WorkflowStep::new_command(
            "Invalid Environment".to_string(),
            "echo \"Invalid environment: $ENV\" && exit 1".to_string(),
            "Print error and exit".to_string(),
            false,
        )]),
        None,
    );

    let branch_step = WorkflowStep::new_branch(
        "Deploy by Environment".to_string(),
        "Deploy based on environment type".to_string(),
        "ENV".to_string(),
        vec![
            BranchCase {
                value: "dev".to_string(),
                steps: vec![WorkflowStep::new_command(
                    "Dev Deploy".to_string(),
                    "echo \"Deploying to dev\"".to_string(),
                    "Deploy to development".to_string(),
                    false,
                )],
            },
            BranchCase {
                value: "staging".to_string(),
                steps: vec![WorkflowStep::new_command_with_approval(
                    "Staging Deploy".to_string(),
                    "echo \"Deploying to staging\"".to_string(),
                    "Deploy to staging with approval".to_string(),
                    false,
                )],
            },
        ],
        Some(vec![WorkflowStep::new_command(
            "Default Deploy".to_string(),
            "echo \"Unknown environment, using defaults\"".to_string(),
            "Default deployment".to_string(),
            true,
        )]),
    );

    let mut workflow_command = Command::with_variables(
        "complex-deploy".to_string(),
        "Complex deployment workflow with conditionals and branches".to_string(),
        vec![
            conditional_step,
            WorkflowStep::new_command(
                "Pre-deploy".to_string(),
                "echo \"Preparing deployment\"".to_string(),
                "Prepare for deployment".to_string(),
                false,
            ),
            branch_step,
            WorkflowStep::new_command(
                "Post-deploy".to_string(),
                "echo \"Deployment complete\"".to_string(),
                "Finalize deployment".to_string(),
                false,
            ),
        ],
        vec!["deployment".to_string(), "complex".to_string()],
        vec![
            WorkflowVariable::new(
                "ENV".to_string(),
                "Deployment environment (dev, staging, prod)".to_string(),
                Some("dev".to_string()),
                true,
            ),
            WorkflowVariable::new(
                "VERSION".to_string(),
                "Version to deploy".to_string(),
                None,
                false,
            ),
        ],
    );
    // Set fixed timestamp for predictable snapshots
    workflow_command.created_at = 1684756234;

    let mut simple_command = Command::new(
        "hello".to_string(),
        "Simple hello world command".to_string(),
        "echo \"Hello, World!\"".to_string(),
        vec!["example".to_string()],
    );
    // Set fixed timestamp for predictable snapshots
    simple_command.created_at = 1684756234;

    // Create export data structure
    let mut commands = HashMap::new();
    commands.insert("hello".to_string(), simple_command);
    commands.insert("complex-deploy".to_string(), workflow_command);

    let export_data = ExportData {
        version: "0.1.0".to_string(),
        metadata: ExportMetadata {
            exported_at: 1684756234,
            exported_by: "test-user".to_string(),
            description: "Test export with complex workflow structures".to_string(),
        },
        commands: Some(commands),
        workflows: None,
    };

    // Serialize to JSON
    let json_output =
        serde_json::to_string_pretty(&export_data).expect("Failed to serialize export data");
    let normalized_output = normalize_json(&json_output);

    // Read expected snapshot - if it doesn't exist, write the output for manual inspection
    let snapshot_path = "tests/snapshots/workflow_export.json";
    if !std::path::Path::new(snapshot_path).exists() {
        fs::write(snapshot_path, &normalized_output).expect("Failed to write snapshot");
        panic!("Created new snapshot file: {}", snapshot_path);
    }

    let expected =
        fs::read_to_string(snapshot_path).expect("Missing workflow export snapshot file");
    let normalized_expected = normalize_json(&expected);

    pretty_assertions::assert_eq!(normalized_expected, normalized_output);
}

#[test]
fn simple_command_export_snapshot() {
    // Test simple command export structure
    let mut command = Command::new(
        "git-status".to_string(),
        "Show git repository status".to_string(),
        "git status --porcelain".to_string(),
        vec!["git".to_string(), "status".to_string()],
    );
    // Set fixed timestamp for predictable snapshots
    command.created_at = 1684756234;

    let mut commands = HashMap::new();
    commands.insert("git-status".to_string(), command);

    let export_data = ExportData {
        version: "0.1.0".to_string(),
        metadata: ExportMetadata {
            exported_at: 1684756234,
            exported_by: "test-user".to_string(),
            description: "Test export with simple command".to_string(),
        },
        commands: Some(commands),
        workflows: None,
    };

    let json_output =
        serde_json::to_string_pretty(&export_data).expect("Failed to serialize export data");
    let normalized_output = normalize_json(&json_output);

    // Read expected snapshot - if it doesn't exist, write the output for manual inspection
    let snapshot_path = "tests/snapshots/simple_command_export.json";
    if !std::path::Path::new(snapshot_path).exists() {
        fs::write(snapshot_path, &normalized_output).expect("Failed to write snapshot");
        panic!("Created new snapshot file: {}", snapshot_path);
    }

    let expected =
        fs::read_to_string(snapshot_path).expect("Missing simple command export snapshot file");
    let normalized_expected = normalize_json(&expected);

    pretty_assertions::assert_eq!(normalized_expected, normalized_output);
}
