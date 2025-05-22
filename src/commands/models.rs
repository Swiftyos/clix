use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Represents a stored command that can be executed directly
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Command {
    /// Unique name identifier for the command
    pub name: String,
    /// Human-readable description of what the command does
    pub description: String,
    /// The actual shell command to execute
    pub command: String,
    /// Unix timestamp when the command was created
    pub created_at: u64,
    /// Unix timestamp when the command was last used (None if never used)
    pub last_used: Option<u64>,
    /// Number of times the command has been executed
    pub use_count: u32,
    /// List of tags for organizing and filtering commands
    pub tags: Vec<String>,
}

impl Command {
    /// Creates a new command with the given parameters
    ///
    /// # Arguments
    /// * `name` - Unique identifier for the command
    /// * `description` - Human-readable description of what the command does
    /// * `command` - The actual shell command to execute
    /// * `tags` - List of tags for organizing and filtering commands
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

    /// Updates usage statistics when the command is executed
    pub fn mark_used(&mut self) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.last_used = Some(now);
        self.use_count += 1;
    }
}

/// Represents a variable that can be used in workflow steps
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct WorkflowVariable {
    /// Name of the variable
    pub name: String,
    /// Human-readable description of the variable's purpose
    pub description: String,
    /// Optional default value for the variable
    pub default_value: Option<String>,
    /// Whether the variable must be provided when running the workflow
    pub required: bool,
}

impl WorkflowVariable {
    /// Creates a new workflow variable
    ///
    /// # Arguments
    /// * `name` - Name of the variable
    /// * `description` - Human-readable description of the variable's purpose
    /// * `default_value` - Optional default value for the variable
    /// * `required` - Whether the variable must be provided when running the workflow
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

/// Represents a saved set of variable values for reuse across workflow runs
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkflowVariableProfile {
    /// Name of the profile
    pub name: String,
    /// Human-readable description of the profile's purpose
    pub description: String,
    /// Map of variable names to their values in this profile
    pub variables: HashMap<String, String>,
}

impl WorkflowVariableProfile {
    /// Creates a new workflow variable profile
    ///
    /// # Arguments
    /// * `name` - Name of the profile
    /// * `description` - Human-readable description of the profile's purpose
    /// * `variables` - Map of variable names to their values in this profile
    pub fn new(name: String, description: String, variables: HashMap<String, String>) -> Self {
        WorkflowVariableProfile {
            name,
            description,
            variables,
        }
    }
}

/// Represents a sequence of steps that can be executed together
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Workflow {
    /// Unique name identifier for the workflow
    pub name: String,
    /// Human-readable description of what the workflow does
    pub description: String,
    /// Ordered list of steps in the workflow
    pub steps: Vec<WorkflowStep>,
    /// Unix timestamp when the workflow was created
    pub created_at: u64,
    /// Unix timestamp when the workflow was last used (None if never used)
    pub last_used: Option<u64>,
    /// Number of times the workflow has been executed
    pub use_count: u32,
    /// List of tags for organizing and filtering workflows
    pub tags: Vec<String>,
    /// List of variables that can be used in workflow steps
    pub variables: Vec<WorkflowVariable>,
    /// Map of profile names to variable profiles for quick setting of multiple variables
    pub profiles: HashMap<String, WorkflowVariableProfile>,
}

/// The type of a workflow step, determining its behavior
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum StepType {
    /// Regular command execution
    Command,
    /// Command execution with user interaction (pauses for user to complete authentication)
    Auth,
    /// Conditional step that executes different blocks based on a condition
    Conditional,
    /// Branch step that selects a path based on variable value (similar to switch/case)
    Branch,
    /// Loop step that executes a block of steps repeatedly
    Loop,
}

/// A condition used in conditional and loop steps
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Condition {
    /// The condition expression to evaluate (shell syntax)
    pub expression: String,
    /// Optional variable name to capture the output of the expression
    pub variable: Option<String>,
}

/// Possible actions to take after evaluating a conditional
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ConditionalAction {
    /// Run the then block if condition is true
    RunThen,
    /// Run the else block if condition is false
    RunElse,
    /// Continue to the next step regardless of condition
    Continue,
    /// Break out of the current loop
    Break,
    /// Return from the workflow with the specified exit code
    Return(i32),
}

/// A block of steps to execute in a conditional step
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ConditionalBlock {
    /// List of steps to execute in this block
    pub steps: Vec<WorkflowStep>,
}

/// A conditional step that executes different blocks based on a condition
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ConditionalStep {
    /// The condition to evaluate
    pub condition: Condition,
    /// Block to execute if the condition is true
    pub then_block: ConditionalBlock,
    /// Optional block to execute if the condition is false
    pub else_block: Option<ConditionalBlock>,
    /// Optional action to take after evaluating the condition
    pub action: Option<ConditionalAction>,
}

/// A single case in a branch step
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct BranchCase {
    /// The value to match against the variable
    pub value: String,
    /// Steps to execute if the variable matches this value
    pub steps: Vec<WorkflowStep>,
}

/// A branch step that selects a path based on variable value
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct BranchStep {
    /// The variable to evaluate
    pub variable: String,
    /// List of cases to match against the variable
    pub cases: Vec<BranchCase>,
    /// Optional default case to execute if no other case matches
    pub default_case: Option<Vec<WorkflowStep>>,
}

/// A loop step that executes a block of steps repeatedly
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LoopStep {
    /// The condition that controls the loop
    pub condition: Condition,
    /// Steps to execute in each iteration of the loop
    pub steps: Vec<WorkflowStep>,
}

/// A single step in a workflow
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct WorkflowStep {
    /// Name of the step
    pub name: String,
    /// Command to execute (or condition expression for conditional steps)
    pub command: String,
    /// Human-readable description of what the step does
    pub description: String,
    /// Whether to continue to the next step if this one fails
    pub continue_on_error: bool,
    /// Type of step (Command, Auth, Conditional, Branch, Loop)
    pub step_type: StepType,
    /// Whether this step requires explicit user approval before execution
    #[serde(default = "default_require_approval")]
    pub require_approval: bool,
    /// Data for conditional steps (present only if step_type is Conditional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditional: Option<ConditionalStep>,
    /// Data for branch steps (present only if step_type is Branch)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<BranchStep>,
    /// Data for loop steps (present only if step_type is Loop)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_data: Option<LoopStep>,
}

/// Default value function for require_approval (false by default)
fn default_require_approval() -> bool {
    false
}

impl WorkflowStep {
    /// Creates a new command step
    ///
    /// # Arguments
    /// * `name` - Name of the step
    /// * `command` - Command to execute
    /// * `description` - Description of what the step does
    /// * `continue_on_error` - Whether to continue to the next step if this one fails
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

    /// Creates a new command step that requires approval
    ///
    /// # Arguments
    /// * `name` - Name of the step
    /// * `command` - Command to execute
    /// * `description` - Description of what the step does
    /// * `continue_on_error` - Whether to continue to the next step if this one fails
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

    /// Creates a new authentication step (pauses for user interaction)
    ///
    /// # Arguments
    /// * `name` - Name of the step
    /// * `command` - Command to execute
    /// * `description` - Description of what the step does
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

    /// Creates a new conditional step (if/then/else)
    ///
    /// # Arguments
    /// * `name` - Name of the step
    /// * `description` - Description of what the step does
    /// * `condition` - The condition to evaluate
    /// * `then_steps` - Steps to execute if the condition is true
    /// * `else_steps` - Optional steps to execute if the condition is false
    /// * `action` - Optional action to take after evaluating the condition
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

    /// Creates a new branch step (switch/case)
    ///
    /// # Arguments
    /// * `name` - Name of the step
    /// * `description` - Description of what the step does
    /// * `variable` - The variable to evaluate
    /// * `cases` - List of cases to match against the variable
    /// * `default_case` - Optional default case to execute if no other case matches
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

    /// Creates a new loop step
    ///
    /// # Arguments
    /// * `name` - Name of the step
    /// * `description` - Description of what the step does
    /// * `condition` - The condition that controls the loop
    /// * `steps` - Steps to execute in each iteration of the loop
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

    /// Adds approval requirement to this step (builder pattern)
    ///
    /// # Returns
    /// * A new workflow step with approval required
    pub fn with_approval(mut self) -> Self {
        self.require_approval = true;
        self
    }
}

impl Workflow {
    /// Creates a new workflow
    ///
    /// # Arguments
    /// * `name` - Unique name identifier for the workflow
    /// * `description` - Human-readable description of what the workflow does
    /// * `steps` - Ordered list of steps in the workflow
    /// * `tags` - List of tags for organizing and filtering workflows
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

    /// Creates a new workflow with variables
    ///
    /// # Arguments
    /// * `name` - Unique name identifier for the workflow
    /// * `description` - Human-readable description of what the workflow does
    /// * `steps` - Ordered list of steps in the workflow
    /// * `tags` - List of tags for organizing and filtering workflows
    /// * `variables` - List of variables that can be used in workflow steps
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

    /// Adds or updates a variable in the workflow
    ///
    /// # Arguments
    /// * `variable` - The variable to add or update
    pub fn add_variable(&mut self, variable: WorkflowVariable) {
        // Replace if exists, add if not
        if let Some(idx) = self.variables.iter().position(|v| v.name == variable.name) {
            self.variables[idx] = variable;
        } else {
            self.variables.push(variable);
        }
    }

    /// Adds a variable profile to the workflow
    ///
    /// # Arguments
    /// * `profile` - The profile to add
    pub fn add_profile(&mut self, profile: WorkflowVariableProfile) {
        self.profiles.insert(profile.name.clone(), profile);
    }

    /// Gets a variable profile by name
    ///
    /// # Arguments
    /// * `name` - The name of the profile to get
    ///
    /// # Returns
    /// * Some(profile) if found, None otherwise
    pub fn get_profile(&self, name: &str) -> Option<&WorkflowVariableProfile> {
        self.profiles.get(name)
    }

    /// Updates usage statistics when the workflow is executed
    pub fn mark_used(&mut self) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.last_used = Some(now);
        self.use_count += 1;
    }
}

/// Central storage for commands and workflows
#[derive(Debug, Serialize, Deserialize)]
pub struct CommandStore {
    /// Map of command names to commands
    pub commands: HashMap<String, Command>,
    /// Map of workflow names to workflows
    pub workflows: HashMap<String, Workflow>,
}

impl CommandStore {
    /// Creates a new empty command store
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
