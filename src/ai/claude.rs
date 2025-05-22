use crate::commands::{Command, Workflow, WorkflowStep};
use crate::error::{ClixError, Result};
use crate::settings::Settings;
use colored::Colorize;
use dotenv::dotenv;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";
const CLAUDE_MODELS_URL: &str = "https://api.anthropic.com/v1/models";

// Rate limiting configuration
const DEFAULT_REQUESTS_PER_MINUTE: u32 = 50;
const DEFAULT_TOKENS_PER_MINUTE: u32 = 40000;
const DEFAULT_MAX_RETRIES: u32 = 3;
const BASE_RETRY_DELAY_MS: u64 = 1000;

pub struct RateLimiter {
    requests_per_minute: u32,
    tokens_per_minute: u32,
    request_times: Arc<Mutex<Vec<Instant>>>,
    token_usage: Arc<Mutex<Vec<(Instant, u32)>>>,
}

impl RateLimiter {
    pub fn new(requests_per_minute: u32, tokens_per_minute: u32) -> Self {
        Self {
            requests_per_minute,
            tokens_per_minute,
            request_times: Arc::new(Mutex::new(Vec::new())),
            token_usage: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(DEFAULT_REQUESTS_PER_MINUTE, DEFAULT_TOKENS_PER_MINUTE)
    }

    pub fn check_and_wait(&self, estimated_tokens: u32) -> Result<()> {
        let now = Instant::now();
        let one_minute_ago = now - Duration::from_secs(60);

        // Check request rate limit
        {
            let mut request_times = self.request_times.lock().unwrap();
            request_times.retain(|&time| time > one_minute_ago);

            if request_times.len() >= self.requests_per_minute as usize {
                let wait_time = request_times[0] + Duration::from_secs(60) - now;
                if wait_time > Duration::from_secs(0) {
                    println!(
                        "{} Rate limit reached. Waiting {} seconds...",
                        "Clix:".yellow().bold(),
                        wait_time.as_secs()
                    );
                    thread::sleep(wait_time);
                }
            }

            request_times.push(now);
        }

        // Check token rate limit
        {
            let mut token_usage = self.token_usage.lock().unwrap();
            token_usage.retain(|(time, _)| *time > one_minute_ago);

            let current_tokens: u32 = token_usage.iter().map(|(_, tokens)| tokens).sum();

            if current_tokens + estimated_tokens > self.tokens_per_minute {
                if let Some((oldest_time, _)) = token_usage.first() {
                    let wait_time = *oldest_time + Duration::from_secs(60) - now;
                    if wait_time > Duration::from_secs(0) {
                        println!(
                            "{} Token rate limit reached. Waiting {} seconds...",
                            "Clix:".yellow().bold(),
                            wait_time.as_secs()
                        );
                        thread::sleep(wait_time);
                    }
                }
            }

            token_usage.push((now, estimated_tokens));
        }

        Ok(())
    }
}

pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub exponential_backoff: bool,
    pub retry_on_rate_limit: bool,
    pub retry_on_network_error: bool,
    pub retry_on_server_error: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            base_delay_ms: BASE_RETRY_DELAY_MS,
            exponential_backoff: true,
            retry_on_rate_limit: true,
            retry_on_network_error: true,
            retry_on_server_error: true,
        }
    }
}

#[derive(Debug, Clone)]
pub enum RetryableError {
    RateLimit,
    NetworkError,
    ServerError(u16),
    Timeout,
}

impl RetryableError {
    pub fn should_retry(&self, config: &RetryConfig) -> bool {
        match self {
            RetryableError::RateLimit => config.retry_on_rate_limit,
            RetryableError::NetworkError => config.retry_on_network_error,
            RetryableError::ServerError(status) => config.retry_on_server_error && *status >= 500,
            RetryableError::Timeout => config.retry_on_network_error,
        }
    }
}

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
    content: Vec<RequestContent>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RequestContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

// Claude response models - normal response
#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

// Error response structure
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    #[serde(rename = "type")]
    error_type: String,
    error: ApiError,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    error_type: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: String,
}

// Models list response
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ModelInfo {
    name: String,
    description: String,
    max_tokens: u32,
}

// The possible actions Claude might suggest
#[derive(Debug, PartialEq, Eq)]
pub enum ClaudeAction {
    RunCommand(String),
    RunWorkflow(String),
    CreateCommand {
        name: String,
        description: String,
        command: String,
    },
    CreateWorkflow {
        name: String,
        description: String,
        steps: Vec<WorkflowStep>,
    },
    NoAction,
}

pub struct ClaudeAssistant {
    client: Client,
    api_key: String,
    settings: Settings,
    rate_limiter: RateLimiter,
    retry_config: RetryConfig,
}

impl ClaudeAssistant {
    pub fn new(settings: Settings) -> Result<Self> {
        // Load .env file if it exists
        dotenv().ok();

        // Get API key from environment
        let api_key = env::var("ANTHROPIC_API_KEY").map_err(|_| {
            ClixError::InvalidCommandFormat(
                "ANTHROPIC_API_KEY environment variable not set. Please set it or create a .env file.".to_string(),
            )
        })?;

        let client = Client::new();

        Ok(ClaudeAssistant {
            client,
            api_key,
            settings,
            rate_limiter: RateLimiter::with_defaults(),
            retry_config: RetryConfig::default(),
        })
    }

    pub fn ask(
        &self,
        question: &str,
        command_history: Vec<&Command>,
        workflow_history: Vec<&Workflow>,
    ) -> Result<(String, ClaudeAction)> {
        self.ask_with_retry(
            question,
            command_history,
            workflow_history,
            &self.retry_config,
        )
    }

    pub fn ask_with_retry(
        &self,
        question: &str,
        command_history: Vec<&Command>,
        workflow_history: Vec<&Workflow>,
        retry_config: &RetryConfig,
    ) -> Result<(String, ClaudeAction)> {
        let mut last_error: Option<RetryableError> = None;

        for attempt in 0..=retry_config.max_retries {
            if attempt > 0 {
                if let Some(ref error) = last_error {
                    if !error.should_retry(retry_config) {
                        break;
                    }

                    let delay = if retry_config.exponential_backoff {
                        retry_config.base_delay_ms * (2_u64.pow(attempt - 1))
                    } else {
                        retry_config.base_delay_ms
                    };

                    println!(
                        "{} Retrying in {} seconds... (attempt {}/{})",
                        "Clix:".yellow().bold(),
                        delay / 1000,
                        attempt,
                        retry_config.max_retries
                    );

                    thread::sleep(Duration::from_millis(delay));
                }
            }

            match self.ask_internal(question, &command_history, &workflow_history) {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(self.categorize_error(&e));
                    if attempt == retry_config.max_retries {
                        return Err(e);
                    }
                }
            }
        }

        Err(ClixError::ApiError("Max retries exceeded".to_string()))
    }

    fn ask_internal(
        &self,
        question: &str,
        command_history: &[&Command],
        workflow_history: &[&Workflow],
    ) -> Result<(String, ClaudeAction)> {
        println!("{} Asking Claude...", "Clix:".blue().bold());

        // Estimate tokens (rough estimation)
        let estimated_tokens = (question.len() / 4) as u32 + 1000; // Rough token estimation

        // Apply rate limiting
        self.rate_limiter.check_and_wait(estimated_tokens)?;

        // Create system prompt
        let system_prompt = self.create_system_prompt(command_history, workflow_history);

        // Create user message
        let user_message = Message {
            role: "user".to_string(),
            content: vec![RequestContent {
                content_type: "text".to_string(),
                text: question.to_string(),
            }],
        };

        // Create request
        let request = ClaudeRequest {
            model: self.settings.ai_model.clone(),
            max_tokens: self.settings.ai_settings.max_tokens,
            temperature: self.settings.ai_settings.temperature,
            messages: vec![user_message],
            system: system_prompt,
        };

        // Create headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("x-api-key", HeaderValue::from_str(&self.api_key)?);
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

        // Make request
        let response = self
            .client
            .post(CLAUDE_API_URL)
            .headers(headers)
            .json(&request)
            .send()
            .map_err(|e| {
                ClixError::CommandExecutionFailed(format!("Failed to call Claude API: {}", e))
            })?;

        // Get the raw response body first
        let raw_response = response.text().map_err(|e| {
            ClixError::CommandExecutionFailed(format!("Failed to get raw response body: {}", e))
        })?;

        // Print the raw response for debugging
        println!("Raw API response: {}", raw_response);

        // Check if this is an error response
        if raw_response.contains("\"type\":\"error\"") {
            let error_response: ErrorResponse =
                serde_json::from_str(&raw_response).map_err(|e| {
                    ClixError::CommandExecutionFailed(format!(
                        "Failed to parse error response: {}",
                        e
                    ))
                })?;

            return Err(ClixError::CommandExecutionFailed(format!(
                "API Error: {} - {}",
                error_response.error_type, error_response.error.message
            )));
        }

        // Now parse the response as a successful response
        let claude_response: ClaudeResponse = serde_json::from_str(&raw_response).map_err(|e| {
            ClixError::CommandExecutionFailed(format!("Failed to parse Claude API response: {}", e))
        })?;

        // Extract text and suggested action
        let text = claude_response
            .content
            .iter()
            .map(|content| content.text.clone())
            .collect::<Vec<String>>()
            .join("\n");

        let action = self.parse_action(&text)?;

        Ok((text, action))
    }

    fn categorize_error(&self, error: &ClixError) -> RetryableError {
        match error {
            ClixError::ApiError(msg) => {
                if msg.contains("rate_limit") || msg.contains("429") {
                    RetryableError::RateLimit
                } else if msg.contains("500")
                    || msg.contains("502")
                    || msg.contains("503")
                    || msg.contains("504")
                {
                    // Extract status code if possible
                    if let Some(status) = self.extract_status_code(msg) {
                        RetryableError::ServerError(status)
                    } else {
                        RetryableError::ServerError(500)
                    }
                } else {
                    RetryableError::NetworkError
                }
            }
            ClixError::NetworkError(_) => RetryableError::NetworkError,
            ClixError::CommandExecutionFailed(msg) => {
                if msg.contains("timeout") || msg.contains("connection") {
                    RetryableError::NetworkError
                } else if msg.contains("rate") || msg.contains("429") {
                    RetryableError::RateLimit
                } else {
                    RetryableError::NetworkError
                }
            }
            _ => RetryableError::NetworkError,
        }
    }

    fn extract_status_code(&self, message: &str) -> Option<u16> {
        // Try to extract HTTP status code from error message
        for word in message.split_whitespace() {
            if let Ok(code) = word.parse::<u16>() {
                if (400..600).contains(&code) {
                    return Some(code);
                }
            }
        }
        None
    }

    fn create_system_prompt(
        &self,
        command_history: &[&Command],
        workflow_history: &[&Workflow],
    ) -> String {
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
                prompt.push_str(&format!(
                    "- {}: {}\n  Command: {}\n",
                    cmd.name, cmd.description, cmd.command
                ));
            }
        }

        // Add available workflows
        if !workflow_history.is_empty() {
            prompt.push_str("\nAvailable workflows:\n");
            for wf in workflow_history {
                prompt.push_str(&format!(
                    "- {}: {}\n  Steps: {}\n",
                    wf.name,
                    wf.description,
                    wf.steps.len()
                ));

                // Add steps
                for (i, step) in wf.steps.iter().enumerate() {
                    prompt.push_str(&format!(
                        "  - Step {}: {}\n    Command: {}\n",
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
        if let Some(captures) = regex::Regex::new(r"\[RUN COMMAND: ([^\]]+)\]")
            .unwrap()
            .captures(text)
        {
            let command_name = captures.get(1).unwrap().as_str().trim().to_string();
            return Ok(ClaudeAction::RunCommand(command_name));
        }

        // Check for workflow execution
        if let Some(captures) = regex::Regex::new(r"\[RUN WORKFLOW: ([^\]]+)\]")
            .unwrap()
            .captures(text)
        {
            let workflow_name = captures.get(1).unwrap().as_str().trim().to_string();
            return Ok(ClaudeAction::RunWorkflow(workflow_name));
        }

        // Check for command creation
        if regex::Regex::new(r"\[CREATE COMMAND\]")
            .unwrap()
            .find(text)
            .is_some()
        {
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
        if regex::Regex::new(r"\[CREATE WORKFLOW\]")
            .unwrap()
            .find(text)
            .is_some()
        {
            let name_re = regex::Regex::new(r"Name: ([^\n]+)").unwrap();
            let desc_re = regex::Regex::new(r"Description: ([^\n]+)").unwrap();

            // Parse manually for steps using line-by-line approach instead of complex regex
            if let (Some(name_match), Some(desc_match)) =
                (name_re.captures(text), desc_re.captures(text))
            {
                let name = name_match.get(1).unwrap().as_str().trim().to_string();
                let description = desc_match.get(1).unwrap().as_str().trim().to_string();

                // Parse steps using line-by-line approach
                let mut steps = Vec::new();

                // Find the Steps: section and parse each step
                if let Some(steps_section) = text.split("Steps:").nth(1) {
                    for line in steps_section.lines() {
                        let line = line.trim();
                        if line.starts_with("- ")
                            && line.contains("name=")
                            && line.contains("command=")
                        {
                            // Extract step info with string operations instead of regex
                            if let (Some(name_part), Some(rest)) =
                                (line.split("name=").nth(1), line.split("command=").nth(1))
                            {
                                let step_name =
                                    name_part.split('"').nth(1).unwrap_or("").to_string();
                                let command = rest.split('"').nth(1).unwrap_or("").to_string();

                                // Extract description
                                let step_desc =
                                    if let Some(desc_part) = rest.split("description=").nth(1) {
                                        desc_part.split('"').nth(1).unwrap_or("").to_string()
                                    } else {
                                        "Step generated by Claude".to_string()
                                    };

                                // Extract continue_on_error
                                let continue_on_error = rest.contains("continue_on_error=true");

                                // Extract step type
                                let is_auth_step = rest.contains("step_type=\"Auth\"");

                                let step = if is_auth_step {
                                    WorkflowStep::new_auth(step_name, command, step_desc)
                                } else {
                                    WorkflowStep::new_command(
                                        step_name,
                                        command,
                                        step_desc,
                                        continue_on_error,
                                    )
                                };

                                steps.push(step);
                            }
                        }
                    }
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

    pub fn list_models(&self) -> Result<Vec<String>> {
        // Create headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("x-api-key", HeaderValue::from_str(&self.api_key)?);
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

        // Make request
        let response = self
            .client
            .get(CLAUDE_MODELS_URL)
            .headers(headers)
            .send()
            .map_err(|e| {
                ClixError::CommandExecutionFailed(format!("Failed to call Claude API: {}", e))
            })?;

        // Parse response
        let models_response: ModelsResponse = response.json().map_err(|e| {
            ClixError::CommandExecutionFailed(format!("Failed to parse Claude API response: {}", e))
        })?;

        // Extract model names
        let model_names = models_response
            .models
            .into_iter()
            .map(|model| model.name)
            .collect();

        Ok(model_names)
    }

    pub fn confirm_action(&self, action: &ClaudeAction) -> Result<bool> {
        match action {
            ClaudeAction::RunCommand(name) => {
                print!(
                    "{} Run command '{}'? [y/N]: ",
                    "Confirm:".green().bold(),
                    name
                );
            }
            ClaudeAction::RunWorkflow(name) => {
                print!(
                    "{} Run workflow '{}'? [y/N]: ",
                    "Confirm:".green().bold(),
                    name
                );
            }
            ClaudeAction::CreateCommand { name, .. } => {
                print!(
                    "{} Create command '{}'? [y/N]: ",
                    "Confirm:".green().bold(),
                    name
                );
            }
            ClaudeAction::CreateWorkflow { name, .. } => {
                print!(
                    "{} Create workflow '{}'? [y/N]: ",
                    "Confirm:".green().bold(),
                    name
                );
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

    pub fn ask_conversational(
        &self,
        question: &str,
        session: &crate::ai::conversation::ConversationSession,
        command_history: Vec<&Command>,
        workflow_history: Vec<&Workflow>,
    ) -> Result<(String, ClaudeAction)> {
        println!("{} Asking Claude...", "Clix:".blue().bold());

        // Estimate tokens (rough estimation)
        let estimated_tokens = (question.len() / 4) as u32 + 2000; // More tokens for context

        // Apply rate limiting
        self.rate_limiter.check_and_wait(estimated_tokens)?;

        // Create system prompt with conversation context
        let system_prompt = self.create_conversational_system_prompt(session, &command_history, &workflow_history);

        // Build conversation history
        let mut messages = Vec::new();

        // Add recent conversation history
        let recent_messages = session.get_recent_context(10);
        for msg in recent_messages {
            let role = match msg.role {
                crate::ai::conversation::MessageRole::User => "user",
                crate::ai::conversation::MessageRole::Assistant => "assistant",
                crate::ai::conversation::MessageRole::System => continue, // Skip system messages
            };

            messages.push(Message {
                role: role.to_string(),
                content: vec![RequestContent {
                    content_type: "text".to_string(),
                    text: msg.content.clone(),
                }],
            });
        }

        // Add current question
        messages.push(Message {
            role: "user".to_string(),
            content: vec![RequestContent {
                content_type: "text".to_string(),
                text: question.to_string(),
            }],
        });

        // Create request
        let request = ClaudeRequest {
            model: self.settings.ai_model.clone(),
            max_tokens: self.settings.ai_settings.max_tokens,
            temperature: self.settings.ai_settings.temperature,
            messages,
            system: system_prompt,
        };

        // Create headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("x-api-key", HeaderValue::from_str(&self.api_key)?);
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

        // Make request
        let response = self
            .client
            .post(CLAUDE_API_URL)
            .headers(headers)
            .json(&request)
            .send()
            .map_err(|e| {
                ClixError::CommandExecutionFailed(format!("Failed to call Claude API: {}", e))
            })?;

        // Get the raw response body first
        let raw_response = response.text().map_err(|e| {
            ClixError::CommandExecutionFailed(format!("Failed to get raw response body: {}", e))
        })?;

        // Check if this is an error response
        if raw_response.contains("\"type\":\"error\"") {
            let error_response: ErrorResponse =
                serde_json::from_str(&raw_response).map_err(|e| {
                    ClixError::CommandExecutionFailed(format!(
                        "Failed to parse error response: {}",
                        e
                    ))
                })?;

            return Err(ClixError::CommandExecutionFailed(format!(
                "API Error: {} - {}",
                error_response.error_type, error_response.error.message
            )));
        }

        // Now parse the response as a successful response
        let claude_response: ClaudeResponse = serde_json::from_str(&raw_response).map_err(|e| {
            ClixError::CommandExecutionFailed(format!("Failed to parse Claude API response: {}", e))
        })?;

        // Extract text and suggested action
        let text = claude_response
            .content
            .iter()
            .map(|content| content.text.clone())
            .collect::<Vec<String>>()
            .join("\n");

        let action = self.parse_conversational_action(&text, session)?;

        Ok((text, action))
    }

    fn create_conversational_system_prompt(
        &self,
        session: &crate::ai::conversation::ConversationSession,
        command_history: &[&Command],
        workflow_history: &[&Workflow],
    ) -> String {
        let mut prompt = r#"You are ClaudeAssistant, an AI assistant integrated with the Clix command-line tool.
You are currently in a conversation with a user who is working on creating or refining commands and workflows.

This is a CONVERSATIONAL SESSION. You should:
1. Remember the context from previous messages in this conversation
2. Build upon previous discussions and decisions
3. Ask clarifying questions when needed
4. Help refine and improve workflows through back-and-forth discussion
5. Be more interactive and collaborative than in single-shot requests

CURRENT CONVERSATION STATE: "#.to_string();

        // Add conversation state information
        match &session.state {
            crate::ai::conversation::ConversationState::Active => {
                prompt.push_str("Active conversation - ready for any request\n");
            }
            crate::ai::conversation::ConversationState::WaitingForConfirmation => {
                prompt.push_str("Waiting for user confirmation of a suggested action\n");
            }
            crate::ai::conversation::ConversationState::CreatingWorkflow(state) => {
                prompt.push_str("Currently creating a workflow\n");
                if let Some(name) = &state.name {
                    prompt.push_str(&format!("  Workflow name: {}\n", name));
                }
                if let Some(desc) = &state.description {
                    prompt.push_str(&format!("  Description: {}\n", desc));
                }
                prompt.push_str(&format!("  Steps defined so far: {}\n", state.steps.len()));
            }
            crate::ai::conversation::ConversationState::RefiningWorkflow(name) => {
                prompt.push_str(&format!("Currently refining workflow: {}\n", name));
            }
            crate::ai::conversation::ConversationState::Completed => {
                prompt.push_str("Conversation completed\n");
            }
        }

        prompt.push_str(r#"
Your response formats for conversational mode:

1. For continuing conversation (asking questions, clarifications):
[CONTINUE]
Your response text with questions or clarifications...

2. For workflow creation or refinement:
[CREATE WORKFLOW]
Name: workflow_name
Description: description
Steps:
- Step 1: name="Step 1", command="command1", description="step description", continue_on_error=false, step_type="Command"
...

3. For suggesting to run existing items:
[RUN COMMAND: command_name] or [RUN WORKFLOW: workflow_name]
Explanation...

4. For creating commands:
[CREATE COMMAND]
Name: command_name
Description: description
Command: shell_command

5. For when conversation should end:
[COMPLETE]
Final summary or goodbye message...

"#);

        // Add available commands and workflows
        if !command_history.is_empty() {
            prompt.push_str("\nAvailable commands:\n");
            for cmd in command_history {
                prompt.push_str(&format!(
                    "- {}: {}\n  Command: {}\n",
                    cmd.name, cmd.description, cmd.command
                ));
            }
        }

        if !workflow_history.is_empty() {
            prompt.push_str("\nAvailable workflows:\n");
            for wf in workflow_history {
                prompt.push_str(&format!(
                    "- {}: {}\n  Steps: {}\n",
                    wf.name,
                    wf.description,
                    wf.steps.len()
                ));
            }
        }

        prompt
    }

    fn parse_conversational_action(
        &self,
        text: &str,
        session: &crate::ai::conversation::ConversationSession,
    ) -> Result<ClaudeAction> {
        // Check for conversation continuation
        if regex::Regex::new(r"\[CONTINUE\]")
            .unwrap()
            .find(text)
            .is_some()
        {
            return Ok(ClaudeAction::NoAction); // Continue conversation, no specific action
        }

        // Check for conversation completion
        if regex::Regex::new(r"\[COMPLETE\]")
            .unwrap()
            .find(text)
            .is_some()
        {
            return Ok(ClaudeAction::NoAction); // End conversation
        }

        // Use existing parsing logic for other actions
        self.parse_action(text)
    }
}
