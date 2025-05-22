use crate::commands::{Command, Workflow, WorkflowStep};
use crate::error::{ClixError, Result};
use colored::Colorize;
use dotenv::dotenv;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Write};

const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";

// Claude request models
#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: usize,
    temperature: f32,
    messages: Vec<Message>,
    system: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: Vec<Content>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum Content {
    Text { text: String },
}

// Claude response models
#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: String,
}

// The possible actions Claude might suggest
#[derive(Debug, PartialEq, Eq)]
pub enum ClaudeAction {
    RunCommand(String),
    RunWorkflow(String),
    CreateCommand { name: String, description: String, command: String },
    CreateWorkflow { name: String, description: String, steps: Vec<WorkflowStep> },
    NoAction,
}

pub struct ClaudeAssistant {
    client: Client,
    api_key: String,
}

impl ClaudeAssistant {
    pub fn new() -> Result<Self> {
        // Load .env file if it exists
        dotenv().ok();
        
        // Get API key from environment
        let api_key = env::var("ANTHROPIC_API_KEY").map_err(|_| {
            ClixError::InvalidCommandFormat(
                "ANTHROPIC_API_KEY environment variable not set. Please set it or create a .env file.".to_string(),
            )
        })?;
        
        let client = Client::new();
        
        Ok(ClaudeAssistant { client, api_key })
    }
    
    pub fn ask(&self, question: &str, command_history: Vec<&Command>, workflow_history: Vec<&Workflow>) -> Result<(String, ClaudeAction)> {
        println!("{} Asking Claude...", "Clix:".blue().bold());
        
        // Create system prompt
        let system_prompt = self.create_system_prompt(&command_history, &workflow_history);
        
        // Create user message
        let user_message = Message {
            role: "user".to_string(),
            content: vec![Content::Text { text: question.to_string() }],
        };
        
        // Create request
        let request = ClaudeRequest {
            model: "claude-3-opus-20240229".to_string(),
            max_tokens: 4000,
            temperature: 0.7,
            messages: vec![user_message],
            system: system_prompt,
        };
        
        // Create headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("x-api-key", HeaderValue::from_str(&self.api_key)?);
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        
        // Make request
        let response = self.client
            .post(CLAUDE_API_URL)
            .headers(headers)
            .json(&request)
            .send()
            .map_err(|e| ClixError::CommandExecutionFailed(format!("Failed to call Claude API: {}", e)))?;
        
        // Parse response
        let claude_response: ClaudeResponse = response.json()
            .map_err(|e| ClixError::CommandExecutionFailed(format!("Failed to parse Claude API response: {}", e)))?;
        
        // Extract text and suggested action
        let text = claude_response.content.iter()
            .map(|block| block.text.clone())
            .collect::<Vec<String>>()
            .join("\n");
        
        let action = self.parse_action(&text)?;
        
        Ok((text, action))
    }
    
    fn create_system_prompt(&self, command_history: &[&Command], workflow_history: &[&Workflow]) -> String {
        let mut prompt = r#"You are ClaudeAssistant, an AI assistant integrated with the Clix command-line tool. 
Your role is to help users manage and execute commands and workflows.

Here are the available commands in Clix:
- add: Add a new command
- run: Run a stored command
- list: List all stored commands and workflows
- remove: Remove a stored command
- flow: Workflow management commands
- export: Export commands and workflows to a file
- import: Import commands and workflows from a file
- ask: Ask Claude for help (that's you!)

When a user asks a question, analyze it and determine what they're trying to do. 
Based on their intent, you can suggest:
1. Using an existing command or workflow
2. Creating a new command or workflow
3. Providing information about how to use Clix

Always ask for permission before executing or creating commands/workflows.

Your response should have one of these formats:

1. If suggesting to run an existing command:
[RUN COMMAND: command_name]
Explanation of what this command does and why it's appropriate...

2. If suggesting to run an existing workflow:
[RUN WORKFLOW: workflow_name]
Explanation of what this workflow does and why it's appropriate...

3. If suggesting to create a new command:
[CREATE COMMAND]
Name: command_name
Description: description of what the command does
Command: the actual shell command to run
Explanation of why this new command would be useful...

4. If suggesting to create a new workflow:
[CREATE WORKFLOW]
Name: workflow_name
Description: description of what the workflow does
Steps:
- Step 1: name="Step 1", command="command1", description="step description", continue_on_error=false, step_type="Command"
- Step 2: name="Step 2", command="command2", description="step description", continue_on_error=false, step_type="Command"
...
Explanation of why this new workflow would be useful...

5. If providing information or no action is needed:
[INFO]
Information or help about Clix...

Follow these guidelines:
- Be concise but thorough in your explanations
- Only suggest relevant commands or workflows for the user's needs
- Format your suggestions exactly as shown above so they can be parsed
- Be cautious with destructive operations
- Always prioritize clarity and helpfulness

"#.to_string();
        
        // Add available commands
        if !command_history.is_empty() {
            prompt.push_str("\nAvailable commands:\n");
            for cmd in command_history {
                prompt.push_str(&format!("- {}: {}\n  Command: {}\n", 
                    cmd.name, 
                    cmd.description, 
                    cmd.command
                ));
            }
        }
        
        // Add available workflows
        if !workflow_history.is_empty() {
            prompt.push_str("\nAvailable workflows:\n");
            for wf in workflow_history {
                prompt.push_str(&format!("- {}: {}\n  Steps: {}\n", 
                    wf.name, 
                    wf.description, 
                    wf.steps.len()
                ));
                
                // Add steps
                for (i, step) in wf.steps.iter().enumerate() {
                    prompt.push_str(&format!("  - Step {}: {}\n    Command: {}\n", 
                        i + 1, 
                        step.name, 
                        step.command
                    ));
                }
            }
        }
        
        prompt
    }
    
    fn parse_action(&self, text: &str) -> Result<ClaudeAction> {
        // Check for command execution
        if let Some(captures) = regex::Regex::new(r"\[RUN COMMAND: ([^\]]+)\]").unwrap().captures(text) {
            let command_name = captures.get(1).unwrap().as_str().trim().to_string();
            return Ok(ClaudeAction::RunCommand(command_name));
        }
        
        // Check for workflow execution
        if let Some(captures) = regex::Regex::new(r"\[RUN WORKFLOW: ([^\]]+)\]").unwrap().captures(text) {
            let workflow_name = captures.get(1).unwrap().as_str().trim().to_string();
            return Ok(ClaudeAction::RunWorkflow(workflow_name));
        }
        
        // Check for command creation
        if let Some(_) = regex::Regex::new(r"\[CREATE COMMAND\]").unwrap().find(text) {
            let name_re = regex::Regex::new(r"Name: ([^\n]+)").unwrap();
            let desc_re = regex::Regex::new(r"Description: ([^\n]+)").unwrap();
            let cmd_re = regex::Regex::new(r"Command: ([^\n]+)").unwrap();
            
            if let (Some(name_match), Some(desc_match), Some(cmd_match)) = (
                name_re.captures(text),
                desc_re.captures(text),
                cmd_re.captures(text),
            ) {
                let name = name_match.get(1).unwrap().as_str().trim().to_string();
                let description = desc_match.get(1).unwrap().as_str().trim().to_string();
                let command = cmd_match.get(1).unwrap().as_str().trim().to_string();
                
                return Ok(ClaudeAction::CreateCommand {
                    name,
                    description,
                    command,
                });
            }
        }
        
        // Check for workflow creation
        if let Some(_) = regex::Regex::new(r"\[CREATE WORKFLOW\]").unwrap().find(text) {
            let name_re = regex::Regex::new(r"Name: ([^\n]+)").unwrap();
            let desc_re = regex::Regex::new(r"Description: ([^\n]+)").unwrap();
            let step_re = regex::Regex::new(r"- Step \d+: name=\"([^\"]+)\", command=\"([^\"]+)\", description=\"([^\"]+)\", continue_on_error=(\w+), step_type=\"([^\"]+)\"").unwrap();
            
            if let (Some(name_match), Some(desc_match)) = (
                name_re.captures(text),
                desc_re.captures(text),
            ) {
                let name = name_match.get(1).unwrap().as_str().trim().to_string();
                let description = desc_match.get(1).unwrap().as_str().trim().to_string();
                
                // Parse steps
                let mut steps = Vec::new();
                for step_match in step_re.captures_iter(text) {
                    let step_name = step_match.get(1).unwrap().as_str().to_string();
                    let command = step_match.get(2).unwrap().as_str().to_string();
                    let step_desc = step_match.get(3).unwrap().as_str().to_string();
                    let continue_on_error = step_match.get(4).unwrap().as_str() == "true";
                    let step_type = step_match.get(5).unwrap().as_str();
                    
                    let step = if step_type == "Auth" {
                        WorkflowStep::new_auth(step_name, command, step_desc)
                    } else {
                        WorkflowStep::new_command(step_name, command, step_desc, continue_on_error)
                    };
                    
                    steps.push(step);
                }
                
                if !steps.is_empty() {
                    return Ok(ClaudeAction::CreateWorkflow {
                        name,
                        description,
                        steps,
                    });
                }
            }
        }
        
        // No action found
        Ok(ClaudeAction::NoAction)
    }
    
    pub fn confirm_action(&self, action: &ClaudeAction) -> Result<bool> {
        match action {
            ClaudeAction::RunCommand(name) => {
                print!("{} Run command '{}'? [y/N]: ", "Confirm:".green().bold(), name);
            }
            ClaudeAction::RunWorkflow(name) => {
                print!("{} Run workflow '{}'? [y/N]: ", "Confirm:".green().bold(), name);
            }
            ClaudeAction::CreateCommand { name, .. } => {
                print!("{} Create command '{}'? [y/N]: ", "Confirm:".green().bold(), name);
            }
            ClaudeAction::CreateWorkflow { name, .. } => {
                print!("{} Create workflow '{}'? [y/N]: ", "Confirm:".green().bold(), name);
            }
            ClaudeAction::NoAction => return Ok(false),
        }
        
        io::stdout().flush().map_err(|e| {
            ClixError::CommandExecutionFailed(format!("Failed to flush stdout: {}", e))
        })?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|e| {
            ClixError::CommandExecutionFailed(format!("Failed to read user input: {}", e))
        })?;
        
        let input = input.trim().to_lowercase();
        Ok(input == "y" || input == "yes")
    }
}