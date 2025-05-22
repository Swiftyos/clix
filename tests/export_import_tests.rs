use clix::commands::{Command, Workflow, WorkflowStep};
use clix::storage::Storage;
use clix::share::{ExportManager, ImportManager};
use std::env;
use std::path::PathBuf;
use std::fs;
use test_context::{test_context, AsyncTestContext};

struct ExportImportContext {
    temp_dir: PathBuf,
    storage: Storage,
}

impl AsyncTestContext for ExportImportContext {
    async fn setup() -> Self {
        // Create a temporary directory for tests
        let temp_dir = std::env::temp_dir().join("clix_test").join(format!("test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros()));
        
        fs::create_dir_all(&temp_dir).unwrap();
        
        // Temporarily set HOME environment variable to our test directory
        env::set_var("HOME", &temp_dir);
        
        // Create the storage instance that will use our test directory
        let storage = Storage::new().unwrap();
        
        ExportImportContext {
            temp_dir,
            storage,
        }
    }
    
    async fn teardown(self) {
        // Clean up the temporary directory
        fs::remove_dir_all(&self.temp_dir).unwrap_or_default();
    }
}

#[test_context(ExportImportContext)]
#[tokio::test]
async fn test_export_import(ctx: &mut ExportImportContext) {
    // Set up test data
    let command1 = Command::new(
        "test-cmd1".to_string(),
        "Test command 1".to_string(),
        "echo 'test 1'".to_string(),
        vec!["test".to_string(), "export".to_string()],
    );
    
    let command2 = Command::new(
        "test-cmd2".to_string(),
        "Test command 2".to_string(),
        "echo 'test 2'".to_string(),
        vec!["test".to_string()],
    );
    
    let steps = vec![
        WorkflowStep {
            name: "Step 1".to_string(),
            command: "echo 'Step 1'".to_string(),
            description: "First step".to_string(),
            continue_on_error: false,
        },
    ];
    
    let workflow = Workflow::new(
        "test-workflow".to_string(),
        "Test workflow".to_string(),
        steps,
        vec!["test".to_string(), "export".to_string()],
    );
    
    // Add test data to storage
    ctx.storage.add_command(command1.clone()).unwrap();
    ctx.storage.add_command(command2.clone()).unwrap();
    ctx.storage.add_workflow(workflow.clone()).unwrap();
    
    // Create export file path
    let export_path = ctx.temp_dir.join("export_test.json");
    let export_path_str = export_path.to_str().unwrap();
    
    // Create export manager
    let export_manager = ExportManager::new(ctx.storage.clone());
    
    // Test export all
    export_manager.export_all(export_path_str).unwrap();
    
    // Verify export file exists
    assert!(export_path.exists());
    
    // Create a second storage instance
    env::set_var("HOME", ctx.temp_dir.join("second_storage"));
    fs::create_dir_all(ctx.temp_dir.join("second_storage")).unwrap();
    let second_storage = Storage::new().unwrap();
    
    // Create import manager
    let import_manager = ImportManager::new(second_storage.clone());
    
    // Test import
    let summary = import_manager.import_from_file(export_path_str, false).unwrap();
    
    // Verify import summary
    assert_eq!(summary.commands_added, 2);
    assert_eq!(summary.commands_updated, 0);
    assert_eq!(summary.commands_skipped, 0);
    assert_eq!(summary.workflows_added, 1);
    assert_eq!(summary.workflows_updated, 0);
    assert_eq!(summary.workflows_skipped, 0);
    
    // Verify data was imported
    let imported_commands = second_storage.list_commands().unwrap();
    let imported_workflows = second_storage.list_workflows().unwrap();
    
    assert_eq!(imported_commands.len(), 2);
    assert_eq!(imported_workflows.len(), 1);
    
    // Verify filtered export (only commands with 'export' tag)
    let filtered_export_path = ctx.temp_dir.join("filtered_export.json");
    let filtered_export_path_str = filtered_export_path.to_str().unwrap();
    
    export_manager.export_with_filter(
        filtered_export_path_str,
        Some("export".to_string()),
        false,
        false
    ).unwrap();
    
    // Create a third storage instance
    env::set_var("HOME", ctx.temp_dir.join("third_storage"));
    fs::create_dir_all(ctx.temp_dir.join("third_storage")).unwrap();
    let third_storage = Storage::new().unwrap();
    
    // Import filtered export
    let import_manager_filtered = ImportManager::new(third_storage.clone());
    let filtered_summary = import_manager_filtered.import_from_file(filtered_export_path_str, false).unwrap();
    
    // Verify filtered import
    assert_eq!(filtered_summary.commands_added, 1); // Only command1 has the 'export' tag
    assert_eq!(filtered_summary.workflows_added, 1);
    
    let filtered_commands = third_storage.list_commands().unwrap();
    assert_eq!(filtered_commands.len(), 1);
    assert_eq!(filtered_commands[0].name, command1.name);
}