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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct WorkflowVariable {
    pub name: String,
    pub description: String,
    pub default_value: Option<String>,
    pub required: bool,
}

impl WorkflowVariable {
    pub fn new(
        name: String,
        description: String,
        default_value: Option<String>,
        required: bool,
    ) -> Self {
        WorkflowVariable {
            name,
            description,
            default_value,
            required,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkflowVariableProfile {
    pub name: String,
    pub description: String,
    pub variables: HashMap<String, String>,
}

impl WorkflowVariableProfile {
    pub fn new(name: String, description: String, variables: HashMap<String, String>) -> Self {
        WorkflowVariableProfile {
            name,
            description,
            variables,
        }
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
    pub variables: Vec<WorkflowVariable>,
    pub profiles: HashMap<String, WorkflowVariableProfile>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum StepType {
    Command,
    Auth,
    Conditional,
    Branch,
    Loop,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Condition {
    pub expression: String,
    pub variable: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ConditionalAction {
    RunThen,
    RunElse,
    Continue,
    Break,
    Return(i32),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ConditionalBlock {
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ConditionalStep {
    pub condition: Condition,
    pub then_block: ConditionalBlock,
    pub else_block: Option<ConditionalBlock>,
    pub action: Option<ConditionalAction>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct BranchCase {
    pub value: String,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct BranchStep {
    pub variable: String,
    pub cases: Vec<BranchCase>,
    pub default_case: Option<Vec<WorkflowStep>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LoopStep {
    pub condition: Condition,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct WorkflowStep {
    pub name: String,
    pub command: String,
    pub description: String,
    pub continue_on_error: bool,
    pub step_type: StepType,
    #[serde(default = "default_require_approval")]
    pub require_approval: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditional: Option<ConditionalStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<BranchStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_data: Option<LoopStep>,
}

// Default value function for require_approval
fn default_require_approval() -> bool {
    false
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
            require_approval: false,
            conditional: None,
            branch: None,
            loop_data: None,
        }
    }

    pub fn new_command_with_approval(
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
            require_approval: true,
            conditional: None,
            branch: None,
            loop_data: None,
        }
    }

    pub fn new_auth(name: String, command: String, description: String) -> Self {
        WorkflowStep {
            name,
            command,
            description,
            continue_on_error: false, // Auth steps should not continue on error
            step_type: StepType::Auth,
            require_approval: false,
            conditional: None,
            branch: None,
            loop_data: None,
        }
    }

    pub fn new_conditional(
        name: String,
        description: String,
        condition: Condition,
        then_steps: Vec<WorkflowStep>,
        else_steps: Option<Vec<WorkflowStep>>,
        action: Option<ConditionalAction>,
    ) -> Self {
        let then_block = ConditionalBlock { steps: then_steps };
        let else_block = else_steps.map(|steps| ConditionalBlock { steps });

        WorkflowStep {
            name,
            command: String::new(), // Conditional steps don't have a direct command
            description,
            continue_on_error: false,
            step_type: StepType::Conditional,
            require_approval: false,
            conditional: Some(ConditionalStep {
                condition,
                then_block,
                else_block,
                action,
            }),
            branch: None,
            loop_data: None,
        }
    }

    pub fn new_branch(
        name: String,
        description: String,
        variable: String,
        cases: Vec<BranchCase>,
        default_case: Option<Vec<WorkflowStep>>,
    ) -> Self {
        WorkflowStep {
            name,
            command: String::new(), // Branch steps don't have a direct command
            description,
            continue_on_error: false,
            step_type: StepType::Branch,
            require_approval: false,
            conditional: None,
            branch: Some(BranchStep {
                variable,
                cases,
                default_case,
            }),
            loop_data: None,
        }
    }

    pub fn new_loop(
        name: String,
        description: String,
        condition: Condition,
        steps: Vec<WorkflowStep>,
    ) -> Self {
        WorkflowStep {
            name,
            command: String::new(), // Loop steps don't have a direct command
            description,
            continue_on_error: false,
            step_type: StepType::Loop,
            require_approval: false,
            conditional: None,
            branch: None,
            loop_data: Some(LoopStep { condition, steps }),
        }
    }

    // Method to set approval requirement
    pub fn with_approval(mut self) -> Self {
        self.require_approval = true;
        self
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
            variables: Vec::new(),
            profiles: HashMap::new(),
        }
    }

    pub fn with_variables(
        name: String,
        description: String,
        steps: Vec<WorkflowStep>,
        tags: Vec<String>,
        variables: Vec<WorkflowVariable>,
    ) -> Self {
        let mut workflow = Self::new(name, description, steps, tags);
        workflow.variables = variables;
        workflow
    }

    pub fn add_variable(&mut self, variable: WorkflowVariable) {
        // Replace if exists, add if not
        if let Some(idx) = self.variables.iter().position(|v| v.name == variable.name) {
            self.variables[idx] = variable;
        } else {
            self.variables.push(variable);
        }
    }

    pub fn add_profile(&mut self, profile: WorkflowVariableProfile) {
        self.profiles.insert(profile.name.clone(), profile);
    }

    pub fn get_profile(&self, name: &str) -> Option<&WorkflowVariableProfile> {
        self.profiles.get(name)
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
