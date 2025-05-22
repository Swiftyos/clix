use clix::commands::{Command, Workflow, WorkflowStep};
use clix::storage::Storage;
use std::env;
use std::fs;
use std::path::PathBuf;
use test_context::{AsyncTestContext, test_context};

struct StorageContext {
    temp_dir: PathBuf,
    storage: Storage,
}

impl AsyncTestContext for StorageContext {
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

            StorageContext { temp_dir, storage }
        })
    }

    fn teardown<'a>(self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            // Clean up the temporary directory
            fs::remove_dir_all(&self.temp_dir).unwrap_or_default();
        })
    }
}

#[test_context(StorageContext)]
#[tokio::test]
async fn test_command_storage(ctx: &mut StorageContext) {
    // Create a test command
    let command = Command::new(
        "test-cmd".to_string(),
        "Test command".to_string(),
        "echo 'test'".to_string(),
        vec!["test".to_string()],
    );

    // Add the command
    ctx.storage.add_command(command.clone()).unwrap();

    // Retrieve the command
    let retrieved = ctx.storage.get_command(&command.name).unwrap();

    // Verify it matches
    assert_eq!(retrieved.name, command.name);
    assert_eq!(retrieved.description, command.description);
    assert_eq!(retrieved.command, command.command);

    // Update usage
    ctx.storage.update_command_usage(&command.name).unwrap();

    // Retrieve again and check usage count
    let updated = ctx.storage.get_command(&command.name).unwrap();
    assert_eq!(updated.use_count, 1);
    assert!(updated.last_used.is_some());

    // List commands
    let commands = ctx.storage.list_commands().unwrap();
    assert_eq!(commands.len(), 1);

    // Remove command
    ctx.storage.remove_command(&command.name).unwrap();

    // List should be empty now
    let commands = ctx.storage.list_commands().unwrap();
    assert_eq!(commands.len(), 0);
}

#[test_context(StorageContext)]
#[tokio::test]
async fn test_workflow_storage(ctx: &mut StorageContext) {
    // Create a test workflow
    let steps = vec![WorkflowStep::new_command(
        "Step 1".to_string(),
        "echo 'Step 1'".to_string(),
        "First step".to_string(),
        false,
    )];

    let workflow = Workflow::new(
        "test-workflow".to_string(),
        "Test workflow".to_string(),
        steps,
        vec!["test".to_string()],
    );

    // Add the workflow
    ctx.storage.add_workflow(workflow.clone()).unwrap();

    // Retrieve the workflow
    let retrieved = ctx.storage.get_workflow(&workflow.name).unwrap();

    // Verify it matches
    assert_eq!(retrieved.name, workflow.name);
    assert_eq!(retrieved.description, workflow.description);
    assert_eq!(retrieved.steps.len(), workflow.steps.len());

    // Update usage
    ctx.storage.update_workflow_usage(&workflow.name).unwrap();

    // Retrieve again and check usage count
    let updated = ctx.storage.get_workflow(&workflow.name).unwrap();
    assert_eq!(updated.use_count, 1);
    assert!(updated.last_used.is_some());

    // List workflows
    let workflows = ctx.storage.list_workflows().unwrap();
    assert_eq!(workflows.len(), 1);
    
    // Remove workflow
    ctx.storage.remove_workflow(&workflow.name).unwrap();
    
    // List should be empty now
    let workflows = ctx.storage.list_workflows().unwrap();
    assert_eq!(workflows.len(), 0);
    
    // Trying to remove a non-existent workflow should fail
    let remove_result = ctx.storage.remove_workflow(&workflow.name);
    assert!(remove_result.is_err());
}
