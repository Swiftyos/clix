use clap::{Args, Parser, Subcommand};

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

    /// Workflow management commands
    #[command(subcommand)]
    Flow(FlowCommands),

    /// Export commands and workflows to a file
    Export(ExportArgs),

    /// Import commands and workflows from a file
    Import(ImportArgs),
}

#[derive(Subcommand, Debug)]
pub enum FlowCommands {
    /// Add a new workflow
    Add(AddWorkflowArgs),

    /// Run a stored workflow
    Run(RunWorkflowArgs),

    /// Remove a stored workflow
    Remove(RemoveWorkflowArgs),

    /// List all stored workflows
    List(FlowListArgs),

    /// Add a variable to a workflow
    AddVar(AddWorkflowVarArgs),

    /// Add a profile to a workflow
    AddProfile(AddWorkflowProfileArgs),

    /// List profiles for a workflow
    ListProfiles(ListWorkflowProfilesArgs),
}

#[derive(Args, Debug)]
pub struct AddArgs {
    /// Name of the command
    pub name: String,

    /// Description of the command
    #[arg(short, long)]
    pub description: String,

    /// The command to execute
    #[arg(short, long)]
    pub command: String,

    /// Optional tags for categorization
    #[arg(short, long)]
    pub tags: Option<Vec<String>>,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// Name of the command to run
    pub name: String,
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
pub struct FlowListArgs {
    /// Filter workflows by tag
    #[arg(short, long)]
    pub tag: Option<String>,
}

#[derive(Args, Debug)]
pub struct RemoveArgs {
    /// Name of the command to remove
    pub name: String,
}

#[derive(Args, Debug)]
pub struct AddWorkflowArgs {
    /// Name of the workflow
    pub name: String,

    /// Description of the workflow
    #[arg(short, long)]
    pub description: String,

    /// Path to a JSON file containing workflow steps
    #[arg(short, long)]
    pub steps_file: String,

    /// Optional tags for categorization
    #[arg(short, long)]
    pub tags: Option<Vec<String>>,
}

#[derive(Args, Debug)]
pub struct RunWorkflowArgs {
    /// Name of the workflow to run
    pub name: String,

    /// Profile to use for variables
    #[arg(short, long)]
    pub profile: Option<String>,

    /// Variable values in the format key=value
    #[arg(short, long)]
    pub var: Option<Vec<String>>,
}

#[derive(Args, Debug)]
pub struct RemoveWorkflowArgs {
    /// Name of the workflow to remove
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
pub struct AddWorkflowVarArgs {
    /// Name of the workflow to add the variable to
    pub workflow_name: String,

    /// Name of the variable
    #[arg(short, long)]
    pub name: String,

    /// Description of the variable
    #[arg(short, long)]
    pub description: String,

    /// Default value for the variable
    #[arg(short, long)]
    pub default: Option<String>,

    /// Whether the variable is required
    #[arg(short, long)]
    pub required: bool,
}

#[derive(Args, Debug)]
pub struct AddWorkflowProfileArgs {
    /// Name of the workflow to add the profile to
    pub workflow_name: String,

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
    /// Name of the workflow to list profiles for
    pub workflow_name: String,
}
