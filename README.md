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
- Simple and intuitive CLI interface

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/clix.git

# Build and install
cd clix
cargo install --path .
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

### Managing workflows with the `flow` command

Workflows can be managed using the `flow` subcommand, which has operations for adding, running, listing, and removing workflows.

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


#### Variables in Workflows

Workflows can include variables that are replaced at runtime. Variables are defined using the `{{ variable_name }}` syntax in workflow commands:

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

When running the workflow, you'll be prompted to enter values for each variable. You can also define default values and descriptions for variables:

```bash
# Add a variable to a workflow
clix flow add-var my-workflow --name project_name --description "GCloud project ID" --default "my-default-project"
```

#### Variable Profiles

You can save sets of variable values as profiles for different environments:

```bash
# Create a production profile
clix flow add-profile my-workflow --name prod --description "Production environment" --var project_name=prod-project --var cluster_name=prod-cluster --var zone=us-central1-a --var namespace=production

# Create a development profile
clix flow add-profile my-workflow --name dev --description "Development environment" --var project_name=dev-project --var cluster_name=dev-cluster --var zone=us-central1-a --var namespace=development

# List all profiles for a workflow
clix flow list-profiles my-workflow

# Run a workflow with a specific profile
clix flow run my-workflow --profile prod
```

#### Authentication Steps in Workflows

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