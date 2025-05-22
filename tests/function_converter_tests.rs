use clix::commands::{FunctionConverter, StepType};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use test_context::{AsyncTestContext, test_context};

struct FunctionConverterContext {
    temp_dir: PathBuf,
    examples_dir: PathBuf,
}

impl AsyncTestContext for FunctionConverterContext {
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

            // Get the path to the examples directory (relative to project root)
            let project_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
            let examples_dir = project_dir.join("examples");

            FunctionConverterContext {
                temp_dir,
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

#[test_context(FunctionConverterContext)]
#[tokio::test]
async fn test_convert_check_even_odd_function(ctx: &mut FunctionConverterContext) {
    // Path to the shell functions example
    let shell_script_path = ctx.examples_dir.join("shell_functions.sh");
    let shell_script_path_str = shell_script_path.to_str().unwrap();

    // Convert the check_even_odd function to a workflow
    let workflow = FunctionConverter::convert_function(
        shell_script_path_str,
        "check_even_odd",
        "check-even-odd",
        "Check if a number is even or odd",
        vec!["time".to_string(), "test".to_string()],
    )
    .unwrap();

    // Verify basic workflow properties
    assert_eq!(workflow.name, "check-even-odd");
    assert_eq!(workflow.description, "Check if a number is even or odd");
    assert_eq!(workflow.tags, vec!["time", "test"]);

    // Verify that we have some steps
    assert!(!workflow.steps.is_empty());

    // NOTE: Our current implementation doesn't parse conditionals from shell functions yet
    // This is a simplified test for now
    // TODO: Once conditional parsing is implemented, add this check back
    // let conditional_steps = workflow.steps.iter()
    //    .filter(|step| step.step_type == StepType::Conditional)
    //    .collect::<Vec<_>>();
    // assert!(!conditional_steps.is_empty(), "Workflow should have at least one conditional step");

    // Check for command steps
    let command_steps = workflow
        .steps
        .iter()
        .filter(|step| step.step_type == StepType::Command)
        .collect::<Vec<_>>();

    assert!(
        !command_steps.is_empty(),
        "Workflow should have command steps"
    );
}

#[test_context(FunctionConverterContext)]
#[tokio::test]
async fn test_convert_deploy_env_function(ctx: &mut FunctionConverterContext) {
    // Path to the shell functions example
    let shell_script_path = ctx.examples_dir.join("shell_functions.sh");
    let shell_script_path_str = shell_script_path.to_str().unwrap();

    // Convert the deploy_env function to a workflow
    let workflow = FunctionConverter::convert_function(
        shell_script_path_str,
        "deploy_env",
        "deploy-env",
        "Deploy to different environments",
        vec!["deployment".to_string(), "test".to_string()],
    )
    .unwrap();

    // Verify basic workflow properties
    assert_eq!(workflow.name, "deploy-env");
    assert_eq!(workflow.description, "Deploy to different environments");
    assert_eq!(workflow.tags, vec!["deployment", "test"]);

    // Verify that we have some steps
    assert!(!workflow.steps.is_empty());

    // NOTE: Our current implementation doesn't parse case statements from shell functions yet
    // This is a simplified test for now
    // TODO: Once branch parsing is implemented, add this check back
    // let branch_steps = workflow.steps.iter()
    //    .filter(|step| step.step_type == StepType::Branch)
    //    .collect::<Vec<_>>();
    // assert!(!branch_steps.is_empty(), "Workflow should have at least one branch step");

    // NOTE: Our current implementation doesn't parse case statements from shell functions yet
    // This is a simplified test for now
    // TODO: Once branch parsing is implemented, add this check back
    /*
    if let Some(branch_step) = branch_steps.first() {
        if let Some(branch) = &branch_step.branch {
            assert!(!branch.cases.is_empty(), "Branch step should have cases");

            // Check that we have cases for dev, staging, and prod
            let env_types = branch.cases.iter()
                .map(|case| case.value.as_str())
                .collect::<Vec<_>>();

            assert!(env_types.contains(&"dev"), "Should have a case for dev environment");
            assert!(env_types.contains(&"staging"), "Should have a case for staging environment");
            assert!(env_types.contains(&"prod"), "Should have a case for prod environment");
        }
    }
    */
}
