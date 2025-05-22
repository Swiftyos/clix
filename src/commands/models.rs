use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub command: String,
    pub created_at: u64,
    pub last_used: Option<u64>,
    pub use_count: u32,
    pub tags: Vec<String>,
}

impl Command {
    pub fn new(name: String, description: String, command: String, tags: Vec<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Command {
            name,
            description,
            command,
            created_at: now,
            last_used: None,
            use_count: 0,
            tags,
        }
    }

    pub fn mark_used(&mut self) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.last_used = Some(now);
        self.use_count += 1;
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Workflow {
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub created_at: u64,
    pub last_used: Option<u64>,
    pub use_count: u32,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum StepType {
    Command,
    Auth,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkflowStep {
    pub name: String,
    pub command: String,
    pub description: String,
    pub continue_on_error: bool,
    pub step_type: StepType,
}

impl WorkflowStep {
    pub fn new_command(
        name: String,
        command: String,
        description: String,
        continue_on_error: bool,
    ) -> Self {
        WorkflowStep {
            name,
            command,
            description,
            continue_on_error,
            step_type: StepType::Command,
        }
    }

    pub fn new_auth(name: String, command: String, description: String) -> Self {
        WorkflowStep {
            name,
            command,
            description,
            continue_on_error: false, // Auth steps should not continue on error
            step_type: StepType::Auth,
        }
    }
}

impl Workflow {
    pub fn new(
        name: String,
        description: String,
        steps: Vec<WorkflowStep>,
        tags: Vec<String>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Workflow {
            name,
            description,
            steps,
            created_at: now,
            last_used: None,
            use_count: 0,
            tags,
        }
    }

    pub fn mark_used(&mut self) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.last_used = Some(now);
        self.use_count += 1;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandStore {
    pub commands: HashMap<String, Command>,
    pub workflows: HashMap<String, Workflow>,
}

impl CommandStore {
    pub fn new() -> Self {
        CommandStore {
            commands: HashMap::new(),
            workflows: HashMap::new(),
        }
    }
}

impl Default for CommandStore {
    fn default() -> Self {
        Self::new()
    }
}
