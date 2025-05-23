use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add a new command
    Add(AddArgs),

    /// Run a stored command
    Run(RunArgs),

    /// List all stored commands and workflows
    List(ListArgs),

    /// Remove a stored command
    Remove(RemoveArgs),

    /// Add a variable to a workflow
    AddVar(AddWorkflowVarArgs),

    /// Add a profile to a workflow
    AddProfile(AddWorkflowProfileArgs),

    /// List profiles for a workflow
    ListProfiles(ListWorkflowProfilesArgs),

    /// Add a conditional step to a workflow
    AddCondition(AddConditionArgs),

    /// Add a branch step to a workflow
    AddBranch(AddBranchArgs),

    /// Convert a shell function to a workflow
    ConvertFunction(ConvertFunctionArgs),

    /// Export commands and workflows to a file
    Export(ExportArgs),

    /// Import commands and workflows from a file
    Import(ImportArgs),

    /// Ask Claude AI for help with creating and running commands
    Ask(AskArgs),

    /// Settings management commands
    #[command(subcommand)]
    Settings(SettingsCommands),

    /// Generate shell completions
    Completions(CompletionsArgs),

    /// Git repository management commands
    #[command(subcommand)]
    Git(GitCommands),
}

#[derive(Args, Debug)]
pub struct AddArgs {
    /// Name of the command
    pub name: String,

    /// Description of the command
    #[arg(short, long)]
    pub description: String,

    /// The command to execute (for simple commands)
    #[arg(short, long, conflicts_with = "steps_file")]
    pub command: Option<String>,

    /// Path to a JSON file containing workflow steps (for workflows)
    #[arg(short = 'f', long, conflicts_with = "command")]
    pub steps_file: Option<String>,

    /// Optional tags for categorization
    #[arg(short, long)]
    pub tags: Option<Vec<String>>,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// Name of the command to run
    pub name: String,

    /// Profile to use for variables (for workflows)
    #[arg(short, long)]
    pub profile: Option<String>,

    /// Variable values in the format key=value (for workflows)
    #[arg(short, long)]
    pub var: Option<Vec<String>>,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Filter commands by tag
    #[arg(short, long)]
    pub tag: Option<String>,

    /// List only commands (no workflows)
    #[arg(long)]
    pub commands_only: bool,

    /// List only workflows (no commands)
    #[arg(long)]
    pub workflows_only: bool,
}

#[derive(Args, Debug)]
pub struct RemoveArgs {
    /// Name of the command to remove
    pub name: String,
}

#[derive(Args, Debug)]
pub struct ExportArgs {
    /// Output file path
    #[arg(short, long)]
    pub output: String,

    /// Export only commands with specific tag
    #[arg(short, long)]
    pub tag: Option<String>,

    /// Export commands only (no workflows)
    #[arg(long)]
    pub commands_only: bool,

    /// Export workflows only (no commands)
    #[arg(long)]
    pub workflows_only: bool,
}

#[derive(Args, Debug)]
pub struct ImportArgs {
    /// Input file path
    #[arg(short, long)]
    pub input: String,

    /// Overwrite existing commands with the same name
    #[arg(short, long)]
    pub overwrite: bool,
}

#[derive(Args, Debug)]
pub struct AskArgs {
    /// The question or request for Claude
    pub question: String,

    /// Start an interactive conversation with Claude (optional)
    #[arg(short, long)]
    pub interactive: bool,

    /// Continue an existing conversation session by ID
    #[arg(short, long)]
    pub session: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum SettingsCommands {
    /// List all settings
    List,

    /// Set the AI model to use with Claude
    SetAiModel(SetAiModelArgs),

    /// List available AI models from Claude
    ListAiModels,

    /// Set the AI temperature (0.0 to 1.0)
    SetAiTemperature(SetAiTemperatureArgs),

    /// Set the AI max tokens
    SetAiMaxTokens(SetAiMaxTokensArgs),
}

#[derive(Args, Debug)]
pub struct SetAiModelArgs {
    /// The model name (e.g., claude-3-opus-20240229)
    pub model: String,
}

#[derive(Args, Debug)]
pub struct SetAiTemperatureArgs {
    /// The temperature value (0.0 to 1.0)
    pub temperature: f32,
}

#[derive(Args, Debug)]
pub struct SetAiMaxTokensArgs {
    /// The maximum number of tokens
    pub max_tokens: usize,
}

#[derive(Args, Debug)]
pub struct AddWorkflowVarArgs {
    /// Name of the command/workflow to add the variable to
    pub command_name: String,

    /// Name of the variable
    #[arg(short, long)]
    pub name: String,

    /// Description of the variable
    #[arg(short, long)]
    pub description: String,

    /// Default value for the variable
    #[arg(short = 'd', long)]
    pub default: Option<String>,

    /// Whether the variable is required
    #[arg(short, long)]
    pub required: bool,
}

#[derive(Args, Debug)]
pub struct AddWorkflowProfileArgs {
    /// Name of the command/workflow to add the profile to
    pub command_name: String,

    /// Name of the profile
    #[arg(short, long)]
    pub name: String,

    /// Description of the profile
    #[arg(short, long)]
    pub description: String,

    /// Variable values in the format key=value
    #[arg(short, long)]
    pub var: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ListWorkflowProfilesArgs {
    /// Name of the command/workflow to list profiles for
    pub command_name: String,
}

#[derive(Args, Debug)]
pub struct AddConditionArgs {
    /// Name of the command/workflow to add the condition to
    pub command_name: String,

    /// Name of the conditional step
    #[arg(short, long)]
    pub name: String,

    /// Description of the conditional step
    #[arg(short, long)]
    pub description: String,

    /// The condition expression to evaluate
    #[arg(short, long)]
    pub condition: String,

    /// Variable to store result in (optional)
    #[arg(short, long)]
    pub variable: Option<String>,

    /// Steps file for the 'then' block
    #[arg(long)]
    pub then_file: String,

    /// Steps file for the 'else' block (optional)
    #[arg(long)]
    pub else_file: Option<String>,

    /// Action to take (run_then, run_else, continue, break, return)
    #[arg(short, long)]
    pub action: Option<String>,

    /// Return code if action is 'return'
    #[arg(short, long)]
    pub return_code: Option<i32>,
}

#[derive(Args, Debug)]
pub struct AddBranchArgs {
    /// Name of the command/workflow to add the branch to
    pub command_name: String,

    /// Name of the branch step
    #[arg(short, long)]
    pub name: String,

    /// Description of the branch step
    #[arg(short, long)]
    pub description: String,

    /// Variable name to branch on
    #[arg(short, long)]
    pub variable: String,

    /// Cases file in JSON format
    #[arg(short, long)]
    pub cases_file: String,

    /// Default case steps file (optional)
    #[arg(long)]
    pub default_file: Option<String>,
}

#[derive(Args, Debug)]
pub struct ConvertFunctionArgs {
    /// Name for the new command/workflow
    pub command_name: String,

    /// Path to the shell script file containing the function
    #[arg(short, long)]
    pub file: String,

    /// Name of the function to convert
    #[arg(long)]
    pub function: String,

    /// Description of the workflow
    #[arg(short, long)]
    pub description: String,

    /// Optional tags for categorization
    #[arg(short, long)]
    pub tags: Option<Vec<String>>,
}

#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// The shell to generate completions for
    #[arg(value_enum)]
    pub shell: Shell,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

#[derive(Subcommand, Debug)]
pub enum GitCommands {
    /// Add a git repository for sharing commands
    AddRepo(AddRepoArgs),

    /// Remove a git repository
    RemoveRepo(RemoveRepoArgs),

    /// List all configured git repositories
    ListRepos,

    /// Pull latest changes from all repositories
    Pull,

    /// Sync (pull) and show status of all repositories
    Status,
}

#[derive(Args, Debug)]
pub struct AddRepoArgs {
    /// Name for the repository (used as local directory name)
    pub name: String,

    /// Git repository URL
    #[arg(short, long)]
    pub url: String,
}

#[derive(Args, Debug)]
pub struct RemoveRepoArgs {
    /// Name of the repository to remove
    pub name: String,
}
