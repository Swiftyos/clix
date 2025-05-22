# Clix

A command-line tool for developers to store and execute commands and workflows.

## Features

- Save frequently used commands with descriptions for easy recall
- Create and run complex workflows (e.g., troubleshooting production issues)
- Support for authentication steps in workflows that pause for user interaction
- Support for variables in workflows with templating using `{{ variable_name }}` syntax
- Save and reuse variables with profiles for different environments
- Tag commands and workflows for better organization
- Track command usage statistics
- Export and import commands to share with your team
- AI-powered assistance for creating and running commands using Claude
- Simple and intuitive CLI interface

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/clix.git

# Build and install
cd clix
cargo install --path .
```

## Command Reference

Clix offers a comprehensive set of commands for managing both individual commands and complex workflows. Here's a complete reference of all available commands:

```
USAGE:
    clix [SUBCOMMAND]

SUBCOMMANDS:
    add        Add a new command
    run        Run a stored command
    list       List all stored commands and workflows
    remove     Remove a stored command
    flow       Workflow management commands (see below for subcommands)
    export     Export commands and workflows to a file
    import     Import commands and workflows from a file
    ask        Ask Claude AI for help with creating and running commands
    settings   Settings management commands (see below for subcommands)
    help       Print this help message or help for a specific command
```

Workflow-specific commands:

```
USAGE:
    clix flow [SUBCOMMAND]

SUBCOMMANDS:
    add            Add a new workflow
    run            Run a stored workflow
    remove         Remove a stored workflow
    list           List all stored workflows
    add-var        Add a variable to a workflow
    add-profile    Add a profile to a workflow
    list-profiles  List profiles for a workflow
    help           Print help for flow or a specific subcommand
```

## Usage

### Adding a command

```bash
clix add my-command --description "Description of the command" --command "echo Hello, World!"
```

### Running a command

```bash
clix run my-command
```

### Listing commands

```bash
# List all commands
clix list

# List commands with a specific tag
clix list --tag deployment
```

### Removing a command

```bash
clix remove my-command
```

### Using Claude AI Assistant

Clix integrates with Anthropic's Claude AI to help you create and run commands and workflows. The AI can suggest existing commands or workflows to run, or it can create new ones based on your request.

#### Setup

First, you need to set up your Anthropic API key:

1. Get an API key from [Anthropic Console](https://console.anthropic.com/)
2. Create a `.env` file in your clix directory (copy from `.env.example`)
3. Add your API key to the `.env` file: `ANTHROPIC_API_KEY=your_api_key_here`

#### Using the Ask Command

```bash
# Ask Claude for help with a command
clix ask "How do I create a command to list files with details?"

# Ask for help with a workflow
clix ask "Create a workflow for deploying to AWS with authentication"

# Ask for help finding and running existing commands
clix ask "What command do I have for listing Docker containers?"
```

#### Configuring Claude AI Settings

You can configure various settings for the Claude AI integration:

```bash
# List current settings
clix settings list

# List available AI models
clix settings list-ai-models

# Set AI model to use
clix settings set-ai-model claude-3-haiku-20240307

# Set AI temperature (0.0 to 1.0, lower is more deterministic)
clix settings set-ai-temperature 0.5

# Set AI max tokens (output length limit)
clix settings set-ai-max-tokens 2000
```

The Claude assistant will analyze your question and:
1. Suggest an existing command or workflow to run
2. Propose creating a new command or workflow
3. Provide information or guidance

When Claude suggests running a command or creating a new one, you'll be asked for confirmation before any action is taken.

## Working with Workflows

Workflows allow you to define a sequence of steps that are executed in order. Each step can be a regular command or an authentication step that requires user interaction.

### Creating Workflow Files

Workflows are defined in JSON files. Each workflow is an array of step objects with the following structure:

```json
{
  "name": "Step Name",                        // Required: A descriptive name for the step
  "command": "command to execute",           // Required: The command to run
  "description": "Description of the step",   // Required: A description of what the step does
  "continue_on_error": true/false,          // Optional: Whether to continue if this step fails (default: false)
  "step_type": "Command" or "Auth"           // Optional: The type of step (default: "Command")
}
```

Step types:
- `Command`: Regular command execution
- `Auth`: Executes the command and pauses for user interaction, useful for authentication flows

Example workflow file (gcloud-workflow.json):

```json
[
  {
    "name": "Set GCloud Project",
    "command": "gcloud config set project {{ project_name }}",
    "description": "Set the active GCloud project",
    "continue_on_error": false,
    "step_type": "Command"
  },
  {
    "name": "Authenticate with GCloud",
    "command": "gcloud auth login",
    "description": "Login to Google Cloud",
    "continue_on_error": false,
    "step_type": "Auth"
  },
  {
    "name": "Get Cluster Credentials",
    "command": "gcloud container clusters get-credentials {{ cluster_name }} --zone={{ zone }}",
    "description": "Fetch credentials for the specified GKE cluster",
    "continue_on_error": false,
    "step_type": "Command"
  },
  {
    "name": "Set Kubernetes Namespace",
    "command": "kubectl config set-context --current --namespace={{ namespace }}",
    "description": "Set the default namespace for kubectl commands",
    "continue_on_error": false,
    "step_type": "Command"
  },
  {
    "name": "List Pods",
    "command": "kubectl get pods -n {{ namespace }}",
    "description": "List all pods in the specified namespace",
    "continue_on_error": true,
    "step_type": "Command"
  }
]
```

### Managing workflows with the `flow` command

#### Adding a workflow

Create a JSON file containing the workflow steps:

```json
[
  {
    "name": "Step 1",
    "command": "echo 'Step 1'",
    "description": "First step of the workflow",
    "continue_on_error": false,
    "step_type": "Command"
  },
  {
    "name": "Auth Step",
    "command": "gcloud auth login",
    "description": "Authenticate with Google Cloud",
    "continue_on_error": false,
    "step_type": "Auth"
  },
  {
    "name": "Step 2",
    "command": "echo 'Step 2'",
    "description": "Second step of the workflow",
    "continue_on_error": true,
    "step_type": "Command"
  }
]
```

The `step_type` field can be either `Command` or `Auth`:
- `Command`: Regular command execution (default behavior)
- `Auth`: Executes the command and then pauses for user interaction, waiting for the user to press Enter after completing authentication

Then add the workflow:

```bash
clix flow add my-workflow --description "My workflow" --steps-file workflow.json
```

#### Running a workflow

```bash
# Run a workflow
clix flow run my-workflow

# Run a workflow with specific variable values
clix flow run my-workflow --var project_name=my-project --var cluster_name=prod-cluster

# Run a workflow using a saved profile
clix flow run my-workflow --profile prod
```

#### Listing workflows

```bash
clix flow list
clix flow list --tag production
```

#### Removing a workflow

```bash
clix flow remove my-workflow
```


### Variables in Workflows

Variables make your workflows flexible and reusable across different environments. Clix supports template variables in workflow commands, allowing you to define values at runtime or use profiles for consistent environments.

#### Variable Syntax

In your workflow steps, use the `{{ variable_name }}` syntax to include variables:

```json
[
  {
    "name": "Set GCloud Project",
    "command": "gcloud config set project {{ project_name }}",
    "description": "Set the active GCloud project",
    "continue_on_error": false,
    "step_type": "Command"
  },
  {
    "name": "Get Cluster Credentials",
    "command": "gcloud container clusters get-credentials {{ cluster_name }} --zone={{ zone }}",
    "description": "Fetch credentials for the specified GKE cluster",
    "continue_on_error": false,
    "step_type": "Command"
  }
]
```

When running the workflow, you'll be prompted to enter values for each variable that appears in any step's command.

#### Adding Variables to Workflows

You can define variables with descriptions, default values, and requirements to provide better guidance for users:

```bash
# Add a required variable without a default value
clix flow add-var my-workflow --name cluster_name --description "GKE cluster name" --required

# Add an optional variable with a default value
clix flow add-var my-workflow --name zone --description "GCP zone" --default "us-central1-a" 
```

#### Running Workflows with Variables

There are multiple ways to provide variable values when running a workflow:

```bash
# Prompted for variables: You'll be asked for any missing values interactively
clix flow run my-workflow

# Command-line variables: Provide values directly in the command
clix flow run my-workflow --var project_name=my-project --var cluster_name=prod-cluster

# Mixed approach: Provide some values via command line, be prompted for others
clix flow run my-workflow --var project_name=my-project
```

### Variable Profiles

Profiles allow you to save sets of variable values for different environments (like development, staging, production) or different configurations. This eliminates the need to repeatedly enter the same values when running a workflow:

```bash
# Create a production profile
clix flow add-profile my-workflow --name prod --description "Production environment" \
  --var project_name=prod-project \
  --var cluster_name=prod-cluster \
  --var zone=us-central1-a \
  --var namespace=production

# Create a development profile
clix flow add-profile my-workflow --name dev --description "Development environment" \
  --var project_name=dev-project \
  --var cluster_name=dev-cluster \
  --var zone=us-central1-a \
  --var namespace=development

# List all profiles for a workflow
clix flow list-profiles my-workflow

# Run a workflow with a specific profile
clix flow run my-workflow --profile prod

# Override specific profile values
clix flow run my-workflow --profile prod --var cluster_name=prod-cluster-2
```

Profiles make it easy to switch between environments without having to remember and type all the variable values each time.

### Authentication Steps in Workflows

Workflows can include authentication steps that require user interaction. For example, when running `gcloud auth login`, the user needs to follow a browser-based authentication flow. The workflow will pause after executing the auth command and wait for the user to press Enter before continuing:

```
Step 1 - Authenticate with Google Cloud
Description: Login to Google Cloud using your credentials
Command: gcloud auth login

STDOUT:
You are authorizing gcloud CLI to access your Google Cloud resources...
Your browser has been opened to complete the authorization.

This step requires authentication. Please follow the instructions above.
Press Enter when you have completed the authentication process...

```

After the user completes the authentication process and presses Enter, the workflow will continue with the next step.

### Exporting commands and workflows

```bash
# Export all commands and workflows
clix export --output my-commands.json

# Export with filtering
clix export --output deploy-commands.json --tag deploy
```

### Importing commands and workflows

```bash
# Import from a file
clix import --input team-commands.json

# Import and overwrite existing commands
clix import --input team-commands.json --overwrite
```

## Development

### Requirements

- Rust (latest stable version)
- cargo-nextest (`cargo install cargo-nextest`)

### Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/clix.git
cd clix

# Build the project
cargo build

# Run tests
cargo nextest run
```

### Contribution Guidelines

Before submitting a pull request, please ensure:

1. **Tests pass**:
   ```bash
   cargo nextest run
   ```

2. **No clippy warnings**:
   ```bash
   cargo clippy -- -D warnings
   ```

3. **Code is properly formatted**:
   ```bash
   cargo fmt -- --check
   # Fix formatting issues if any
   cargo fmt
   ```

4. **Documentation is updated** if needed

## License

MIT