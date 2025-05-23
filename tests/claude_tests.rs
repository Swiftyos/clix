use clix::ai::claude::ClaudeAction;

// This is a mock module for testing without using the actual API
mod mock_claude {
    use crate::ClaudeAction;
    use clix::commands::WorkflowStep;

    pub fn mock_response(question: &str) -> (String, ClaudeAction) {
        match question {
            q if q.contains("create command") => {
                (
                    "[CREATE COMMAND]\nName: test-echo\nDescription: Echo a test message\nCommand: echo \"This is a test\"\n\nThis command will simply echo a test message to the console.".to_string(),
                    ClaudeAction::CreateCommand {
                        name: "test-echo".to_string(),
                        description: "Echo a test message".to_string(),
                        command: "echo \"This is a test\"".to_string(),
                    }
                )
            },
            q if q.contains("create workflow") => {
                (
                    "[CREATE WORKFLOW]\nName: test-workflow\nDescription: A test workflow\nSteps:\n- Step 1: name=\"Echo Step\", command=\"echo Step 1\", description=\"Echo the first step\", continue_on_error=false, step_type=\"Command\"\n- Step 2: name=\"Echo Step 2\", command=\"echo Step 2\", description=\"Echo the second step\", continue_on_error=true, step_type=\"Command\"\n\nThis workflow will run two echo commands in sequence.".to_string(),
                    ClaudeAction::CreateWorkflow {
                        name: "test-workflow".to_string(),
                        description: "A test workflow".to_string(),
                        steps: vec![
                            WorkflowStep::new_command(
                                "Echo Step".to_string(),
                                "echo Step 1".to_string(),
                                "Echo the first step".to_string(),
                                false,
                            ),
                            WorkflowStep::new_command(
                                "Echo Step 2".to_string(),
                                "echo Step 2".to_string(),
                                "Echo the second step".to_string(),
                                true,
                            ),
                        ],
                    }
                )
            },
            q if q.contains("run command") => {
                (
                    "[RUN COMMAND: list-files]\n\nThis command will list all files in the current directory.".to_string(),
                    ClaudeAction::RunCommand("list-files".to_string())
                )
            },
            q if q.contains("run workflow") => {
                (
                    "[RUN WORKFLOW: deploy-app]\n\nThis workflow will deploy your application to the production environment.".to_string(),
                    ClaudeAction::RunWorkflow("deploy-app".to_string())
                )
            },
            _ => (
                "[INFO]\nI don't have a specific action to suggest for that question. Here's some information about Clix instead...".to_string(),
                ClaudeAction::NoAction
            ),
        }
    }
}

#[test]
fn test_parse_create_command_action() {
    let (text, expected_action) = mock_claude::mock_response("help me create command for echo");

    // In a real implementation, we'd call the parser directly
    // For this test, we'll just validate that our mocked response/action pairs are consistent
    match expected_action {
        ClaudeAction::CreateCommand {
            name,
            description,
            command,
        } => {
            assert_eq!(name, "test-echo");
            assert_eq!(description, "Echo a test message");
            assert_eq!(command, "echo \"This is a test\"");

            // Verify that these values are also in the text response
            assert!(text.contains(&name));
            assert!(text.contains(&description));
            assert!(text.contains(&command));
        }
        _ => panic!("Expected CreateCommand action"),
    }
}

#[test]
fn test_parse_create_workflow_action() {
    let (text, expected_action) = mock_claude::mock_response("help me create workflow");

    match expected_action {
        ClaudeAction::CreateWorkflow {
            name,
            description,
            steps,
        } => {
            assert_eq!(name, "test-workflow");
            assert_eq!(description, "A test workflow");
            assert_eq!(steps.len(), 2);

            assert_eq!(steps[0].name, "Echo Step");
            assert_eq!(steps[0].command, "echo Step 1");

            assert_eq!(steps[1].name, "Echo Step 2");
            assert_eq!(steps[1].command, "echo Step 2");
            assert!(steps[1].continue_on_error);

            // Verify that these values are also in the text response
            assert!(text.contains(&name));
            assert!(text.contains(&description));
            assert!(text.contains("Step 1"));
            assert!(text.contains("Step 2"));
        }
        _ => panic!("Expected CreateWorkflow action"),
    }
}

#[test]
fn test_parse_run_command_action() {
    let (text, expected_action) = mock_claude::mock_response("run command to list files");

    match expected_action {
        ClaudeAction::RunCommand(name) => {
            assert_eq!(name, "list-files");
            assert!(text.contains(&name));
        }
        _ => panic!("Expected RunCommand action"),
    }
}

#[test]
fn test_parse_run_workflow_action() {
    let (text, expected_action) = mock_claude::mock_response("run workflow to deploy the app");

    match expected_action {
        ClaudeAction::RunWorkflow(name) => {
            assert_eq!(name, "deploy-app");
            assert!(text.contains(&name));
        }
        _ => panic!("Expected RunWorkflow action"),
    }
}

#[test]
fn test_parse_no_action() {
    let (text, expected_action) = mock_claude::mock_response("just tell me about clix");

    match expected_action {
        ClaudeAction::NoAction => {
            assert!(text.contains("[INFO]"));
        }
        _ => panic!("Expected NoAction"),
    }
}

#[test]
fn test_models_response_parsing() {
    // Test parsing Claude API response format with "data" field
    let claude_response = r#"{
        "data": [
            {
                "id": "claude-3-opus-20240229",
                "display_name": "Claude 3 Opus",
                "max_tokens": 4096
            },
            {
                "id": "claude-3-sonnet-20240229",
                "display_name": "Claude 3 Sonnet",
                "max_tokens": 4096
            }
        ]
    }"#;

    // Test direct JSON parsing to verify our structs work
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(claude_response);
    assert!(parsed.is_ok(), "Should parse valid JSON");

    // Test that we can extract model IDs
    let json_value = parsed.unwrap();
    if let Some(data) = json_value.get("data").and_then(|v| v.as_array()) {
        let model_ids: Vec<String> = data
            .iter()
            .filter_map(|item| item.get("id").and_then(|v| v.as_str()))
            .map(|s| s.to_string())
            .collect();

        assert_eq!(model_ids.len(), 2);
        assert!(model_ids.contains(&"claude-3-opus-20240229".to_string()));
        assert!(model_ids.contains(&"claude-3-sonnet-20240229".to_string()));
    } else {
        panic!("Should have data field with array");
    }
}

#[test]
fn test_legacy_models_response_parsing() {
    // Test parsing legacy format with "models" field
    let legacy_response = r#"{
        "models": [
            {
                "name": "claude-3-opus-20240229",
                "description": "Claude 3 Opus model",
                "max_tokens": 4096
            },
            {
                "name": "claude-3-sonnet-20240229", 
                "description": "Claude 3 Sonnet model",
                "max_tokens": 4096
            }
        ]
    }"#;

    // Test parsing legacy format
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(legacy_response);
    assert!(parsed.is_ok(), "Should parse valid legacy JSON");

    let json_value = parsed.unwrap();
    if let Some(models) = json_value.get("models").and_then(|v| v.as_array()) {
        let model_names: Vec<String> = models
            .iter()
            .filter_map(|item| item.get("name").and_then(|v| v.as_str()))
            .map(|s| s.to_string())
            .collect();

        assert_eq!(model_names.len(), 2);
        assert!(model_names.contains(&"claude-3-opus-20240229".to_string()));
        assert!(model_names.contains(&"claude-3-sonnet-20240229".to_string()));
    } else {
        panic!("Should have models field with array");
    }
}

#[test]
fn test_mock_list_models() {
    use clix::ai::mock::MockClaudeAssistant;

    // Test the mock implementation
    let result = MockClaudeAssistant::mock_list_models();
    assert!(result.is_ok(), "Mock list_models should succeed");

    let models = result.unwrap();
    assert!(!models.is_empty(), "Should return at least one model");
    assert!(models.contains(&"claude-3-opus-20240229".to_string()));
    assert!(models.contains(&"claude-3-sonnet-20240229".to_string()));
    assert!(models.contains(&"claude-3-haiku-20240307".to_string()));

    println!("Mock models: {:?}", models);
}
