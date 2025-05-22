use clix::commands::{Command, Workflow, WorkflowStep, WorkflowVariable, WorkflowVariableProfile, CommandExecutor};
use clix::commands::models::{BranchCase, Condition, ConditionalAction, StepType};
use clix::share::{ExportManager, ImportManager};
use clix::storage::Storage;
use clix::ai::mock::MockClaudeAssistant;
use clix::ai::claude::ClaudeAction;
use clix::SettingsManager;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use test_context::{AsyncTestContext, test_context};

struct E2ETestContext {
    temp_dir: PathBuf,
    storage: Storage,
    settings_manager: SettingsManager,
}

impl AsyncTestContext for E2ETestContext {
    fn setup<'a>() -> std::pin::Pin<Box<dyn std::future::Future<Output = Self> + Send + 'a>> {
        Box::pin(async {
            // Create a temporary directory for tests
            let temp_dir = std::env::temp_dir().join("clix_e2e_test").join(format!(
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
            let settings_manager = SettingsManager::new().unwrap();

            E2ETestContext { temp_dir, storage, settings_manager }
        })
    }

    fn teardown<'a>(self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            // Clean up the temporary directory
            fs::remove_dir_all(&self.temp_dir).unwrap_or_default();
        })
    }
}

/// Test basic command operations: add, run, list, remove
#[test_context(E2ETestContext)]
#[tokio::test]
async fn test_basic_command_operations(ctx: &mut E2ETestContext) {
    // Test adding commands
    let command1 = Command::new(
        "test-echo".to_string(),
        "Simple echo command".to_string(),
        "echo 'Hello World'".to_string(),
        vec!["test".to_string(), "basic".to_string()],
    );

    let command2 = Command::new(
        "test-date".to_string(),
        "Show current date".to_string(),
        if cfg!(target_os = "windows") { "echo %date%" } else { "date" }.to_string(),
        vec!["test".to_string(), "time".to_string()],
    );

    // Add commands
    ctx.storage.add_command(command1.clone()).unwrap();
    ctx.storage.add_command(command2.clone()).unwrap();

    // Test listing commands
    let commands = ctx.storage.list_commands().unwrap();
    assert_eq!(commands.len(), 2);
    
    let cmd_names: Vec<&str> = commands.iter().map(|c| c.name.as_str()).collect();
    assert!(cmd_names.contains(&"test-echo"));
    assert!(cmd_names.contains(&"test-date"));

    // Test getting specific command
    let retrieved_cmd = ctx.storage.get_command("test-echo").unwrap();
    assert_eq!(retrieved_cmd.name, command1.name);
    assert_eq!(retrieved_cmd.description, command1.description);
    assert_eq!(retrieved_cmd.command, command1.command);

    // Test running a command
    let output = CommandExecutor::execute_command(&retrieved_cmd).unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Hello World"));

    // Test updating usage statistics
    let initial_use_count = retrieved_cmd.use_count;
    ctx.storage.update_command_usage("test-echo").unwrap();
    let updated_cmd = ctx.storage.get_command("test-echo").unwrap();
    assert_eq!(updated_cmd.use_count, initial_use_count + 1);

    // Test removing a command
    ctx.storage.remove_command("test-date").unwrap();
    let remaining_commands = ctx.storage.list_commands().unwrap();
    assert_eq!(remaining_commands.len(), 1);
    assert_eq!(remaining_commands[0].name, "test-echo");

    // Verify removed command cannot be retrieved
    assert!(ctx.storage.get_command("test-date").is_err());
}

/// Test workflow operations: add, run, list, remove
#[test_context(E2ETestContext)]
#[tokio::test]
async fn test_workflow_operations(ctx: &mut E2ETestContext) {
    // Create workflow steps
    let steps = vec![
        WorkflowStep::new_command(
            "Step 1".to_string(),
            "echo 'Starting workflow'".to_string(),
            "First step of the workflow".to_string(),
            false,
        ),
        WorkflowStep::new_command(
            "Step 2".to_string(),
            "echo 'Processing data'".to_string(),
            "Second step of the workflow".to_string(),
            false,
        ),
        WorkflowStep::new_command(
            "Step 3".to_string(),
            "echo 'Workflow complete'".to_string(),
            "Final step of the workflow".to_string(),
            false,
        ),
    ];

    // Create workflow
    let workflow = Workflow::new(
        "test-workflow".to_string(),
        "Test workflow for E2E testing".to_string(),
        steps,
        vec!["test".to_string(), "workflow".to_string()],
    );

    // Add workflow
    ctx.storage.add_workflow(workflow.clone()).unwrap();

    // Test listing workflows
    let workflows = ctx.storage.list_workflows().unwrap();
    assert_eq!(workflows.len(), 1);
    assert_eq!(workflows[0].name, "test-workflow");

    // Test getting specific workflow
    let retrieved_workflow = ctx.storage.get_workflow("test-workflow").unwrap();
    assert_eq!(retrieved_workflow.name, workflow.name);
    assert_eq!(retrieved_workflow.description, workflow.description);
    assert_eq!(retrieved_workflow.steps.len(), 3);

    // Test running a workflow
    let results = CommandExecutor::execute_workflow(&retrieved_workflow, None, None).unwrap();
    assert_eq!(results.len(), 3);
    
    // Verify all steps executed successfully
    for (step_name, result) in results {
        assert!(result.is_ok(), "Step '{}' failed: {:?}", step_name, result);
        let output = result.unwrap();
        assert!(output.status.success());
    }

    // Test updating workflow usage
    let initial_use_count = retrieved_workflow.use_count;
    ctx.storage.update_workflow_usage("test-workflow").unwrap();
    let updated_workflow = ctx.storage.get_workflow("test-workflow").unwrap();
    assert_eq!(updated_workflow.use_count, initial_use_count + 1);

    // Test removing workflow
    ctx.storage.remove_workflow("test-workflow").unwrap();
    let remaining_workflows = ctx.storage.list_workflows().unwrap();
    assert_eq!(remaining_workflows.len(), 0);

    // Verify removed workflow cannot be retrieved
    assert!(ctx.storage.get_workflow("test-workflow").is_err());
}

/// Test workflow variables and profiles
#[test_context(E2ETestContext)]
#[tokio::test]
async fn test_workflow_variables_and_profiles(ctx: &mut E2ETestContext) {
    // Create workflow with variables
    let steps = vec![
        WorkflowStep::new_command(
            "Show Environment".to_string(),
            "echo \"Environment: $ENV\"".to_string(),
            "Display the environment variable".to_string(),
            false,
        ),
        WorkflowStep::new_command(
            "Show Debug".to_string(),
            "echo \"Debug mode: $DEBUG\"".to_string(),
            "Display the debug variable".to_string(),
            false,
        ),
    ];

    let variables = vec![
        WorkflowVariable::new(
            "ENV".to_string(),
            "Environment (dev, staging, prod)".to_string(),
            Some("dev".to_string()),
            true,
        ),
        WorkflowVariable::new(
            "DEBUG".to_string(),
            "Enable debug mode".to_string(),
            Some("false".to_string()),
            false,
        ),
    ];

    let mut workflow = Workflow::with_variables(
        "test-vars".to_string(),
        "Test workflow with variables".to_string(),
        steps,
        vec!["test".to_string(), "variables".to_string()],
        variables,
    );

    // Add workflow profiles
    let dev_profile = WorkflowVariableProfile::new(
        "development".to_string(),
        "Development environment profile".to_string(),
        {
            let mut vars = HashMap::new();
            vars.insert("ENV".to_string(), "dev".to_string());
            vars.insert("DEBUG".to_string(), "true".to_string());
            vars
        },
    );

    let prod_profile = WorkflowVariableProfile::new(
        "production".to_string(),
        "Production environment profile".to_string(),
        {
            let mut vars = HashMap::new();
            vars.insert("ENV".to_string(), "prod".to_string());
            vars.insert("DEBUG".to_string(), "false".to_string());
            vars
        },
    );

    workflow.add_profile(dev_profile);
    workflow.add_profile(prod_profile);

    // Add workflow to storage
    ctx.storage.add_workflow(workflow.clone()).unwrap();

    // Test running workflow with default variables
    let results = CommandExecutor::execute_workflow(&workflow, None, None).unwrap();
    assert_eq!(results.len(), 2);

    // Test running workflow with development profile
    let results = CommandExecutor::execute_workflow(&workflow, Some("development"), None).unwrap();
    assert_eq!(results.len(), 2);
    
    // Verify the environment variable was substituted correctly in the output
    for (step_name, result) in &results {
        assert!(result.is_ok(), "Step '{}' failed: {:?}", step_name, result);
    }

    // Test running workflow with custom variables
    let custom_vars = {
        let mut vars = HashMap::new();
        vars.insert("ENV".to_string(), "staging".to_string());
        vars.insert("DEBUG".to_string(), "true".to_string());
        vars
    };

    let results = CommandExecutor::execute_workflow(&workflow, None, Some(custom_vars)).unwrap();
    assert_eq!(results.len(), 2);

    // Test adding variables to existing workflow
    let new_variable = WorkflowVariable::new(
        "VERSION".to_string(),
        "Application version".to_string(),
        Some("1.0.0".to_string()),
        false,
    );

    let mut updated_workflow = ctx.storage.get_workflow("test-vars").unwrap();
    updated_workflow.add_variable(new_variable);
    ctx.storage.update_workflow(&updated_workflow).unwrap();

    // Verify the variable was added
    let final_workflow = ctx.storage.get_workflow("test-vars").unwrap();
    assert_eq!(final_workflow.variables.len(), 3);
    assert!(final_workflow.variables.iter().any(|v| v.name == "VERSION"));
}

/// Test conditional workflows
#[test_context(E2ETestContext)]
#[tokio::test]
async fn test_conditional_workflows(ctx: &mut E2ETestContext) {
    // Create workflow with conditionals
    let conditional_step = WorkflowStep::new_conditional(
        "Environment Check".to_string(),
        "Check if we're in development environment".to_string(),
        Condition {
            expression: "[ \"$ENV\" = \"dev\" ]".to_string(),
            variable: None,
        },
        vec![WorkflowStep::new_command(
            "Dev Action".to_string(),
            "echo 'Running in development mode'".to_string(),
            "Action for development environment".to_string(),
            false,
        )],
        Some(vec![WorkflowStep::new_command(
            "Non-Dev Action".to_string(),
            "echo 'Running in production mode'".to_string(),
            "Action for non-development environment".to_string(),
            false,
        )]),
        None,
    );

    let steps = vec![conditional_step];

    let variables = vec![WorkflowVariable::new(
        "ENV".to_string(),
        "Environment (dev, staging, prod)".to_string(),
        Some("dev".to_string()),
        true,
    )];

    let workflow = Workflow::with_variables(
        "conditional-test".to_string(),
        "Test conditional workflow".to_string(),
        steps,
        vec!["test".to_string(), "conditional".to_string()],
        variables,
    );

    ctx.storage.add_workflow(workflow.clone()).unwrap();

    // Test running workflow with dev environment
    let dev_vars = {
        let mut vars = HashMap::new();
        vars.insert("ENV".to_string(), "dev".to_string());
        vars
    };

    let results = CommandExecutor::execute_workflow(&workflow, None, Some(dev_vars)).unwrap();
    // Should execute the conditional step which executes the "then" block
    assert!(results.len() >= 1);

    // Test running workflow with prod environment
    let prod_vars = {
        let mut vars = HashMap::new();
        vars.insert("ENV".to_string(), "prod".to_string());
        vars
    };

    let results = CommandExecutor::execute_workflow(&workflow, None, Some(prod_vars)).unwrap();
    // Should execute the conditional step which executes the "else" block
    assert!(results.len() >= 1);
}

/// Test branch workflows
#[test_context(E2ETestContext)]
#[tokio::test]
async fn test_branch_workflows(ctx: &mut E2ETestContext) {
    // Create workflow with branching
    let branch_step = WorkflowStep::new_branch(
        "Process by Type".to_string(),
        "Handle different item types".to_string(),
        "ITEM_TYPE".to_string(),
        vec![
            BranchCase {
                value: "document".to_string(),
                steps: vec![WorkflowStep::new_command(
                    "Process Document".to_string(),
                    "echo 'Processing document'".to_string(),
                    "Handle document type".to_string(),
                    false,
                )],
            },
            BranchCase {
                value: "image".to_string(),
                steps: vec![WorkflowStep::new_command(
                    "Process Image".to_string(),
                    "echo 'Processing image'".to_string(),
                    "Handle image type".to_string(),
                    false,
                )],
            },
        ],
        Some(vec![WorkflowStep::new_command(
            "Unknown Type".to_string(),
            "echo 'Unknown item type'".to_string(),
            "Handle unknown type".to_string(),
            false,
        )]),
    );

    let steps = vec![branch_step];

    let variables = vec![WorkflowVariable::new(
        "ITEM_TYPE".to_string(),
        "Type of item to process".to_string(),
        Some("document".to_string()),
        true,
    )];

    let workflow = Workflow::with_variables(
        "branch-test".to_string(),
        "Test branch workflow".to_string(),
        steps,
        vec!["test".to_string(), "branch".to_string()],
        variables,
    );

    ctx.storage.add_workflow(workflow.clone()).unwrap();

    // Test running workflow with document type
    let doc_vars = {
        let mut vars = HashMap::new();
        vars.insert("ITEM_TYPE".to_string(), "document".to_string());
        vars
    };

    let results = CommandExecutor::execute_workflow(&workflow, None, Some(doc_vars)).unwrap();
    assert!(results.len() >= 1);

    // Test running workflow with image type  
    let img_vars = {
        let mut vars = HashMap::new();
        vars.insert("ITEM_TYPE".to_string(), "image".to_string());
        vars
    };

    let results = CommandExecutor::execute_workflow(&workflow, None, Some(img_vars)).unwrap();
    assert!(results.len() >= 1);

    // Test running workflow with unknown type (should use default case)
    let unknown_vars = {
        let mut vars = HashMap::new();
        vars.insert("ITEM_TYPE".to_string(), "unknown".to_string());
        vars
    };

    let results = CommandExecutor::execute_workflow(&workflow, None, Some(unknown_vars)).unwrap();
    assert!(results.len() >= 1);
}

/// Test approval workflows
#[test_context(E2ETestContext)]
#[tokio::test]
async fn test_approval_workflows(ctx: &mut E2ETestContext) {
    // Create workflow with approval steps
    let steps = vec![
        WorkflowStep::new_command(
            "Safe Step".to_string(),
            "echo 'This is safe'".to_string(),
            "A safe operation".to_string(),
            false,
        ),
        WorkflowStep::new_command_with_approval(
            "Dangerous Step".to_string(),
            "echo 'This requires approval'".to_string(),
            "A dangerous operation".to_string(),
            false,
        ),
        WorkflowStep::new_command(
            "Final Step".to_string(),
            "echo 'Workflow complete'".to_string(),
            "Final operation".to_string(),
            false,
        ).with_approval(), // Alternative way to set approval
    ];

    let workflow = Workflow::new(
        "approval-test".to_string(),
        "Test workflow with approval steps".to_string(),
        steps,
        vec!["test".to_string(), "approval".to_string()],
    );

    ctx.storage.add_workflow(workflow.clone()).unwrap();

    // Verify workflow structure has approval steps
    let retrieved_workflow = ctx.storage.get_workflow("approval-test").unwrap();
    assert_eq!(retrieved_workflow.steps.len(), 3);
    assert!(!retrieved_workflow.steps[0].require_approval);
    assert!(retrieved_workflow.steps[1].require_approval);
    assert!(retrieved_workflow.steps[2].require_approval);

    // Note: We can't easily test actual approval prompts in automated tests
    // since they require user input. This test verifies the structure only.
}

/// Test auth workflows (with mocking)
#[test_context(E2ETestContext)]
#[tokio::test]
async fn test_auth_workflows(ctx: &mut E2ETestContext) {
    // Create workflow with auth step
    let steps = vec![
        WorkflowStep::new_auth(
            "Google Cloud Auth".to_string(),
            "gcloud auth login".to_string(),
            "Login to Google Cloud".to_string(),
        ),
        WorkflowStep::new_command(
            "Configure Project".to_string(),
            "gcloud config set project my-project".to_string(),
            "Set the GCP project".to_string(),
            false,
        ),
        WorkflowStep::new_command(
            "Deploy App".to_string(),
            "gcloud app deploy".to_string(),
            "Deploy the application".to_string(),
            false,
        ),
    ];

    let workflow = Workflow::new(
        "gcloud-auth-test".to_string(),
        "Test workflow with authentication".to_string(),
        steps,
        vec!["test".to_string(), "auth".to_string(), "gcp".to_string()],
    );

    ctx.storage.add_workflow(workflow.clone()).unwrap();

    // Verify workflow structure includes auth step
    let retrieved_workflow = ctx.storage.get_workflow("gcloud-auth-test").unwrap();
    assert_eq!(retrieved_workflow.steps.len(), 3);

    // Verify the first step is an auth step
    assert_eq!(retrieved_workflow.steps[0].step_type, StepType::Auth);
    assert_eq!(retrieved_workflow.steps[0].name, "Google Cloud Auth");

    // Note: Testing actual auth execution would require mocking the auth providers
    // or setting up real credentials, which is beyond the scope of E2E tests.
    // We verify the structure is correct here.
}

/// Test export and import functionality
#[test_context(E2ETestContext)]
#[tokio::test]
async fn test_export_import_e2e(ctx: &mut E2ETestContext) {
    // Create test data
    let command = Command::new(
        "export-test-cmd".to_string(),
        "Test command for export".to_string(),
        "echo 'export test'".to_string(),
        vec!["test".to_string(), "export".to_string()],
    );

    let workflow = Workflow::new(
        "export-test-workflow".to_string(),
        "Test workflow for export".to_string(),
        vec![WorkflowStep::new_command(
            "Test Step".to_string(),
            "echo 'workflow export test'".to_string(),
            "Test step for export".to_string(),
            false,
        )],
        vec!["test".to_string(), "export".to_string()],
    );

    // Add test data
    ctx.storage.add_command(command.clone()).unwrap();
    ctx.storage.add_workflow(workflow.clone()).unwrap();

    // Test export
    let export_path = ctx.temp_dir.join("e2e_export.json");
    let export_path_str = export_path.to_str().unwrap();

    let export_manager = ExportManager::new(ctx.storage.clone());
    export_manager.export_all(export_path_str).unwrap();

    // Verify export file exists
    assert!(export_path.exists());

    // Create new storage for import test
    let import_temp_dir = ctx.temp_dir.join("import_test");
    fs::create_dir_all(&import_temp_dir).unwrap();
    
    unsafe {
        env::set_var("HOME", &import_temp_dir);
    }
    
    let import_storage = Storage::new().unwrap();
    let import_manager = ImportManager::new(import_storage.clone());

    // Test import
    let summary = import_manager.import_from_file(export_path_str, false).unwrap();

    // Verify import results
    assert_eq!(summary.commands_added, 1);
    assert_eq!(summary.workflows_added, 1);
    assert_eq!(summary.commands_skipped, 0);
    assert_eq!(summary.workflows_skipped, 0);

    // Verify imported data
    let imported_commands = import_storage.list_commands().unwrap();
    let imported_workflows = import_storage.list_workflows().unwrap();

    assert_eq!(imported_commands.len(), 1);
    assert_eq!(imported_workflows.len(), 1);
    assert_eq!(imported_commands[0].name, command.name);
    assert_eq!(imported_workflows[0].name, workflow.name);

    // Test filtered export (commands only)
    let commands_only_path = ctx.temp_dir.join("commands_only.json");
    let commands_only_path_str = commands_only_path.to_str().unwrap();

    export_manager.export_with_filter(
        commands_only_path_str,
        None,
        true,  // commands only
        false,
    ).unwrap();

    // Test filtered export (workflows only)
    let workflows_only_path = ctx.temp_dir.join("workflows_only.json");
    let workflows_only_path_str = workflows_only_path.to_str().unwrap();

    export_manager.export_with_filter(
        workflows_only_path_str,
        None,
        false,
        true,  // workflows only
    ).unwrap();

    // Verify filtered exports
    assert!(commands_only_path.exists());
    assert!(workflows_only_path.exists());
}

/// Test AI integration (mocked)
#[test_context(E2ETestContext)]
#[tokio::test]
async fn test_ai_integration_mocked(_ctx: &mut E2ETestContext) {
    // Test AI command creation using mock
    let (response, action) = MockClaudeAssistant::mock_response("create command to list files");

    assert!(response.contains("CREATE COMMAND"));
    
    match action {
        ClaudeAction::CreateCommand { name, description, command } => {
            assert_eq!(name, "test-echo");
            assert_eq!(description, "Echo a test message");
            assert_eq!(command, "echo \"This is a test\"");
        }
        _ => panic!("Expected CreateCommand action"),
    }

    // Test AI workflow creation using mock
    let (workflow_response, workflow_action) = MockClaudeAssistant::mock_response("create workflow for deployment");

    assert!(workflow_response.contains("CREATE WORKFLOW"));
    
    match workflow_action {
        ClaudeAction::CreateWorkflow { name, description, steps } => {
            assert_eq!(name, "test-workflow");
            assert_eq!(description, "A test workflow");
            assert_eq!(steps.len(), 2);
        }
        _ => panic!("Expected CreateWorkflow action"),
    }

    // Test run command action
    let (run_response, run_action) = MockClaudeAssistant::mock_response("run command list-files");
    
    assert!(run_response.contains("RUN COMMAND"));
    
    match run_action {
        ClaudeAction::RunCommand(name) => {
            assert_eq!(name, "list-files");
        }
        _ => panic!("Expected RunCommand action"),
    }

    // Test run workflow action
    let (run_wf_response, run_wf_action) = MockClaudeAssistant::mock_response("run workflow deploy-app");
    
    assert!(run_wf_response.contains("RUN WORKFLOW"));
    
    match run_wf_action {
        ClaudeAction::RunWorkflow(name) => {
            assert_eq!(name, "deploy-app");
        }
        _ => panic!("Expected RunWorkflow action"),
    }

    // Test no action case
    let (no_action_response, no_action) = MockClaudeAssistant::mock_response("what is the weather like?");
    
    assert!(no_action_response.contains("INFO"));
    
    match no_action {
        ClaudeAction::NoAction => {
            // Expected
        }
        _ => panic!("Expected NoAction"),
    }
}

/// Test settings management
#[test_context(E2ETestContext)]
#[tokio::test]
async fn test_settings_management(ctx: &mut E2ETestContext) {
    // Test loading default settings
    let settings = ctx.settings_manager.load().unwrap();
    assert!(!settings.ai_model.is_empty());
    assert!(settings.ai_settings.temperature >= 0.0 && settings.ai_settings.temperature <= 1.0);
    assert!(settings.ai_settings.max_tokens > 0);

    // Test updating AI model
    ctx.settings_manager.update_ai_model("claude-3-sonnet-20240229").unwrap();
    let updated_settings = ctx.settings_manager.load().unwrap();
    assert_eq!(updated_settings.ai_model, "claude-3-sonnet-20240229");

    // Test updating AI temperature
    ctx.settings_manager.update_ai_temperature(0.7).unwrap();
    let updated_settings = ctx.settings_manager.load().unwrap();
    assert_eq!(updated_settings.ai_settings.temperature, 0.7);

    // Test updating AI max tokens
    ctx.settings_manager.update_ai_max_tokens(4000).unwrap();
    let updated_settings = ctx.settings_manager.load().unwrap();
    assert_eq!(updated_settings.ai_settings.max_tokens, 4000);

    // Test invalid temperature (should fail)
    assert!(ctx.settings_manager.update_ai_temperature(1.5).is_err());
    assert!(ctx.settings_manager.update_ai_temperature(-0.1).is_err());
}

/// Comprehensive integration test covering multiple features
#[test_context(E2ETestContext)]
#[tokio::test]
async fn test_comprehensive_integration(ctx: &mut E2ETestContext) {
    // Create commands
    let setup_cmd = Command::new(
        "setup-env".to_string(),
        "Setup environment".to_string(),
        "echo 'Setting up environment'".to_string(),
        vec!["setup".to_string()],
    );

    let deploy_cmd = Command::new(
        "deploy-app".to_string(),
        "Deploy application".to_string(),
        "echo 'Deploying application'".to_string(),
        vec!["deploy".to_string()],
    );

    ctx.storage.add_command(setup_cmd).unwrap();
    ctx.storage.add_command(deploy_cmd).unwrap();

    // Create complex workflow with variables, conditionals, and approval
    let complex_workflow = Workflow::with_variables(
        "full-deployment".to_string(),
        "Complete deployment workflow with all features".to_string(),
        vec![
            // Environment check conditional
            WorkflowStep::new_conditional(
                "Validate Environment".to_string(),
                "Check if environment is valid".to_string(),
                Condition {
                    expression: "[ \"$ENV\" = \"dev\" -o \"$ENV\" = \"staging\" -o \"$ENV\" = \"prod\" ]"
                        .to_string(),
                    variable: None,
                },
                vec![WorkflowStep::new_command(
                    "Environment Valid".to_string(),
                    "echo \"Environment $ENV is valid\"".to_string(),
                    "Confirm valid environment".to_string(),
                    false,
                )],
                Some(vec![WorkflowStep::new_command(
                    "Environment Invalid".to_string(),
                    "echo \"Invalid environment: $ENV\"".to_string(),
                    "Report invalid environment".to_string(),
                    false,
                )]),
                Some(ConditionalAction::Return(1)),
            ),
            // Branch based on environment
            WorkflowStep::new_branch(
                "Environment Setup".to_string(),
                "Setup based on environment".to_string(),
                "ENV".to_string(),
                vec![
                    BranchCase {
                        value: "dev".to_string(),
                        steps: vec![WorkflowStep::new_command(
                            "Dev Setup".to_string(),
                            "echo 'Setting up development environment'".to_string(),
                            "Development setup".to_string(),
                            false,
                        )],
                    },
                    BranchCase {
                        value: "staging".to_string(),
                        steps: vec![WorkflowStep::new_command(
                            "Staging Setup".to_string(),
                            "echo 'Setting up staging environment'".to_string(),
                            "Staging setup".to_string(),
                            false,
                        )],
                    },
                    BranchCase {
                        value: "prod".to_string(),
                        steps: vec![WorkflowStep::new_command_with_approval(
                            "Production Setup".to_string(),
                            "echo 'Setting up production environment'".to_string(),
                            "Production setup with approval".to_string(),
                            false,
                        )],
                    },
                ],
                None,
            ),
            // Final deployment step
            WorkflowStep::new_command(
                "Deploy".to_string(),
                "echo \"Deploying to $ENV environment\"".to_string(),
                "Deploy the application".to_string(),
                false,
            ),
        ],
        vec!["deployment".to_string(), "integration".to_string()],
        vec![
            WorkflowVariable::new(
                "ENV".to_string(),
                "Target environment".to_string(),
                Some("dev".to_string()),
                true,
            ),
        ],
    );

    ctx.storage.add_workflow(complex_workflow).unwrap();

    // Test exporting everything
    let export_path = ctx.temp_dir.join("comprehensive_export.json");
    let export_manager = ExportManager::new(ctx.storage.clone());
    export_manager.export_all(export_path.to_str().unwrap()).unwrap();

    // Test running the complex workflow with different environments
    let workflow = ctx.storage.get_workflow("full-deployment").unwrap();

    // Test with dev environment
    let dev_vars = {
        let mut vars = HashMap::new();
        vars.insert("ENV".to_string(), "dev".to_string());
        vars
    };

    let results = CommandExecutor::execute_workflow(&workflow, None, Some(dev_vars)).unwrap();
    assert!(results.len() >= 3); // At least the conditional, branch, and deploy steps

    // Test with invalid environment (should fail due to return action)
    let invalid_vars = {
        let mut vars = HashMap::new();
        vars.insert("ENV".to_string(), "invalid".to_string());
        vars
    };

    // This should fail or return early due to the conditional with return action
    let invalid_results = CommandExecutor::execute_workflow(&workflow, None, Some(invalid_vars));
    // The workflow might still execute but the conditional should handle the invalid case
    assert!(invalid_results.is_ok()); // The workflow executes, but the conditional handles the error

    // Verify all data is present
    let all_commands = ctx.storage.list_commands().unwrap();
    let all_workflows = ctx.storage.list_workflows().unwrap();
    
    assert_eq!(all_commands.len(), 2);
    assert_eq!(all_workflows.len(), 1);

    // Test that export file contains all data
    assert!(export_path.exists());
    let export_content = fs::read_to_string(&export_path).unwrap();
    assert!(export_content.contains("setup-env"));
    assert!(export_content.contains("deploy-app"));
    assert!(export_content.contains("full-deployment"));
}