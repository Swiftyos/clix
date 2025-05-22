use clix::commands::CommandExecutor;
use clix::share::ImportManager;
use clix::storage::Storage;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use test_context::{AsyncTestContext, test_context};

struct ConditionalExecutionContext {
    temp_dir: PathBuf,
    storage: Storage,
    examples_dir: PathBuf,
}

impl AsyncTestContext for ConditionalExecutionContext {
    fn setup<'a>() -> std::pin::Pin<Box<dyn std::future::Future<Output = Self> + Send + 'a>> {
        Box::pin(async {
            // Create a temporary directory for tests
            let temp_dir = std::env::temp_dir().join("clix_test").join(format!(
                "test_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_micros()
            ));

            fs::create_dir_all(&temp_dir).unwrap();

            // Temporarily set HOME environment variable to our test directory
            unsafe {
                env::set_var("HOME", &temp_dir);
            }

            // Create the storage instance that will use our test directory
            let storage = Storage::new().unwrap();

            // Get the path to the examples directory (relative to project root)
            let project_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
            let examples_dir = project_dir.join("examples");

            ConditionalExecutionContext {
                temp_dir,
                storage,
                examples_dir,
            }
        })
    }

    fn teardown<'a>(self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            // Clean up the temporary directory
            fs::remove_dir_all(&self.temp_dir).unwrap_or_default();
        })
    }
}

#[test_context(ConditionalExecutionContext)]
#[tokio::test]
async fn test_time_workflow_execution(ctx: &mut ConditionalExecutionContext) {
    // Create import manager
    let import_manager = ImportManager::new(ctx.storage.clone());

    // Path to the time workflow example
    let example_path = ctx.examples_dir.join("time_workflow.json");
    let example_path_str = example_path.to_str().unwrap();

    // Import the workflow
    let summary = import_manager
        .import_from_file(example_path_str, false)
        .unwrap();

    // Verify import summary
    assert_eq!(summary.workflows_added, 1);
    assert_eq!(summary.commands_added, 0);

    // Verify workflow was imported
    let workflows = ctx.storage.list_workflows().unwrap();
    assert_eq!(workflows.len(), 1);
    assert!(workflows.iter().any(|w| w.name == "time-check"));

    // Get the workflow
    let workflow = ctx.storage.get_workflow("time-check").unwrap();

    // Execute the workflow
    let results = CommandExecutor::execute_workflow(&workflow, None, None).unwrap();

    // Verify all steps executed
    assert_eq!(results.len(), 3);

    // First step gets the current hour
    assert_eq!(results[0].0, "Get Current Hour");
    assert!(results[0].1.is_ok());

    // Second step is the conditional
    assert_eq!(results[1].0, "Check Even or Odd");
    assert!(results[1].1.is_ok());

    // Third step shows the date
    assert_eq!(results[2].0, "Show Date Info");
    assert!(results[2].1.is_ok());

    // In the test execution, we only have the main steps in the results
    // The conditional execution doesn't add the inner steps to the results array
    // This is expected behavior - the inner steps are executed but not tracked at the top level
    // So we don't expect to see "Even Hour" or "Odd Hour" in the results
    let hour_steps = results
        .iter()
        .filter(|(name, _)| name == "Even Hour" || name == "Odd Hour")
        .collect::<Vec<_>>();

    // Since these are executed inside the conditional, they don't appear in the main results
    // They're executed in the context of the conditional step
    assert_eq!(hour_steps.len(), 0);
}

#[test_context(ConditionalExecutionContext)]
#[tokio::test]
async fn test_gke_workflow_execution(ctx: &mut ConditionalExecutionContext) {
    // Create import manager
    let import_manager = ImportManager::new(ctx.storage.clone());

    // Path to the GKE workflow example
    let example_path = ctx.examples_dir.join("gke_workflow.json");
    let example_path_str = example_path.to_str().unwrap();

    // Import the workflow
    let summary = import_manager
        .import_from_file(example_path_str, false)
        .unwrap();

    // Verify import summary
    assert_eq!(summary.workflows_added, 1);
    assert_eq!(summary.commands_added, 0);

    // Verify workflow was imported
    let workflows = ctx.storage.list_workflows().unwrap();
    assert_eq!(workflows.len(), 1);
    assert!(workflows.iter().any(|w| w.name == "gke"));

    // Get the workflow
    let workflow = ctx.storage.get_workflow("gke").unwrap();

    // Set variables for dev environment
    let mut vars = std::collections::HashMap::new();
    vars.insert("env".to_string(), "dev".to_string());
    vars.insert("dev_project_id".to_string(), "test-project".to_string());
    vars.insert("dev_cluster_name".to_string(), "test-cluster".to_string());
    vars.insert("dev_zone".to_string(), "us-central1-a".to_string());
    vars.insert("dev_namespace".to_string(), "test-namespace".to_string());

    // Execute the workflow with variables
    // Skip execution for now as it would try to run actual gcloud commands
    // Just verify that the workflow structure is correct
    assert_eq!(workflow.steps.len(), 3);
    assert_eq!(workflow.steps[0].name, "Check Authentication");
    assert_eq!(workflow.steps[1].name, "Set Environment");
    assert_eq!(workflow.steps[2].name, "Show Current Context");
}
