use clix::ai::ClaudeAssistant;
use clix::settings::{Settings, AiSettings};
use clix::{Command, Workflow};
use std::env;
use dotenv::dotenv;

// This is an integration test that checks if Claude API is accessible
// It requires a valid ANTHROPIC_API_KEY in the environment
// To run this test: ANTHROPIC_API_KEY=your_key cargo test -- --ignored claude_api_test

#[test]
#[ignore] // Ignore by default to avoid API calls in regular test runs
fn test_claude_api_connection() {
    // Skip test if ANTHROPIC_API_KEY is not set
    dotenv().ok();
    if env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!("Skipping Claude API test: ANTHROPIC_API_KEY not set");
        return;
    }

    // Create test settings
    let settings = Settings {
        ai_model: "claude-3-opus-20240229".to_string(),
        ai_settings: AiSettings {
            temperature: 0.7,
            max_tokens: 500,  // Small for testing
        },
    };

    // Initialize the assistant
    let assistant = ClaudeAssistant::new(settings).expect("Failed to create ClaudeAssistant");
    
    // Create test data
    let test_command = Command::new(
        "test-command".to_string(),
        "A test command for Claude API".to_string(),
        "echo 'Hello from Claude'".to_string(),
        vec!["test".to_string()],
    );
    
    let commands = vec![&test_command];
    let workflows: Vec<&Workflow> = vec![];
    
    // Make the API call with a simple question
    let (response, action) = assistant
        .ask("What is the test command for?", commands, workflows)
        .expect("Failed to call Claude API");
    
    // Check if we got a valid response
    assert!(!response.is_empty(), "Response should not be empty");
    println!("Claude response: {}", response);
    
    // Log the action for debugging
    println!("Claude action: {:?}", action);
}