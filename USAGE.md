# Clix Usage Guide

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
    add                Add a new workflow
    run                Run a stored workflow
    remove             Remove a stored workflow
    list               List all stored workflows
    add-var            Add a variable to a workflow
    add-profile        Add a profile to a workflow
    list-profiles      List profiles for a workflow
    add-condition      Add a conditional step to a workflow
    add-branch         Add a branch step to a workflow
    add-loop           Add a loop step to a workflow
    convert-function   Convert a shell function to a workflow
    help               Print help for flow or a specific subcommand
```

## Basic Commands

### Adding a command

```bash
clix add list-files --description "Lists files in the current directory" --command "ls -la"

# Add a command with tags
clix add list-files --description "Lists files in the current directory" --command "ls -la" --tags system,files
```

### Running a command

```bash
clix run list-files
```

### Listing all commands

```bash
# List all commands
clix list

# List with specific tag
clix list --tag deploy
```

### Removing a command

```bash
clix remove list-files
```

## Working with Workflows

### Creating a workflow

Create a JSON file with the workflow steps:

```json
[
  {
    "name": "List Files",
    "command": "ls -la",
    "description": "List all files in the current directory",
    "continue_on_error": false,
    "step_type": "Command"
  },
  {
    "name": "Show Current Directory",
    "command": "pwd",
    "description": "Print the current working directory",
    "continue_on_error": true,
    "step_type": "Command"
  }
]
```

### Adding a workflow

```bash
clix flow add my-workflow --description "Basic system info workflow" --steps-file workflow.json

# Add with tags
clix flow add my-workflow --description "Basic system info workflow" --steps-file workflow.json --tags system,info
```

### Running a workflow

```bash
# Run a workflow
clix flow run my-workflow

# Run with variables
clix flow run my-workflow --var project_id=my-project --var zone=us-central1-a

# Run with a profile
clix flow run my-workflow --profile prod
```

### Managing workflow variables

```bash
# Add a required variable
clix flow add-var my-workflow --name project_id --description "GCP Project ID" --required

# Add an optional variable with default value
clix flow add-var my-workflow --name zone --description "GCP Zone" --default "us-central1-a"
```

### Creating variable profiles

```bash
# Create a profile
clix flow add-profile my-workflow --name prod --description "Production environment" \
  --var project_id=prod-project --var zone=us-central1-a

# List profiles
clix flow list-profiles my-workflow
```

## Conditional Workflows

### Adding a conditional step

```bash
# Create a file with 'then' steps
cat > then_steps.json << 'EOF'
[
  {
    "name": "Login to GCloud",
    "command": "gcloud auth login",
    "description": "Login to Google Cloud",
    "continue_on_error": false,
    "step_type": "Auth"
  }
]
EOF

# Add conditional step to workflow
clix flow add-condition my-workflow \
  --name "Check Authentication" \
  --description "Check if already authenticated with GCloud" \
  --condition "! gcloud auth application-default print-access-token > /dev/null 2>&1" \
  --then-file then_steps.json
```

### Adding a branch step

```bash
# Create a file with case steps
cat > cases.json << 'EOF'
{
  "dev": [
    {
      "name": "Set Dev Project",
      "command": "gcloud config set project dev-project",
      "description": "Set development project",
      "continue_on_error": false,
      "step_type": "Command"
    }
  ],
  "prod": [
    {
      "name": "Set Prod Project",
      "command": "gcloud config set project prod-project",
      "description": "Set production project",
      "continue_on_error": false,
      "step_type": "Command"
    }
  ]
}
EOF

# Create default case file
cat > default.json << 'EOF'
[
  {
    "name": "Show Usage",
    "command": "echo \"Usage: Please provide 'dev' or 'prod' as environment\"",
    "description": "Show usage information",
    "continue_on_error": false,
    "step_type": "Command"
  }
]
EOF

# Add branch step to workflow
clix flow add-branch my-workflow \
  --name "Set Environment" \
  --description "Configure environment based on parameter" \
  --variable "env" \
  --cases-file cases.json \
  --default-file default.json
```

### Adding a loop step

```bash
# Create a file with loop steps
cat > loop_steps.json << 'EOF'
[
  {
    "name": "Process File",
    "command": "cat {{ file }} | grep ERROR >> errors.txt",
    "description": "Extract errors from log file",
    "continue_on_error": true,
    "step_type": "Command"
  }
]
EOF

# Add loop step to workflow
clix flow add-loop my-workflow \
  --name "Process Files" \
  --description "Process all log files" \
  --variable "file" \
  --values "$(ls *.log)" \
  --steps-file loop_steps.json
```

### Converting shell functions to workflows

```bash
# Convert a shell function to a workflow
clix flow convert-function \
  --file ~/.zshrc \
  --function deploy \
  --name deployment \
  --description "Deploy application to development or production" \
  --tags deploy,aws

# Add approval requirements for sensitive operations
clix flow convert-function \
  --file ~/.zshrc \
  --function deploy \
  --name deploy \
  --approve-dangerous

# Specify default variable values
clix flow convert-function \
  --file ~/.zshrc \
  --function deploy \
  --name deploy \
  --var-defaults env=dev
```

## Sharing Commands and Workflows

### Exporting Commands and Workflows

```bash
# Export all commands and workflows
clix export --output my-commands.json

# Export only commands with a specific tag
clix export --output deploy-commands.json --tag deploy

# Export only commands (no workflows)
clix export --output commands-only.json --commands-only

# Export only workflows (no commands)
clix export --output workflows-only.json --workflows-only
```

### Importing Commands and Workflows

```bash
# Import commands and workflows
clix import --input my-commands.json

# Import and overwrite any existing commands with the same name
clix import --input team-commands.json --overwrite
```

## Examples

### Authentication Workflow with Conditional Logic

```json
[
  {
    "name": "Check Authentication",
    "command": "! gcloud auth application-default print-access-token > /dev/null 2>&1",
    "description": "Check if already authenticated with GCloud",
    "continue_on_error": false,
    "step_type": "Conditional",
    "conditional": {
      "then_steps": [
        {
          "name": "Login to GCloud",
          "command": "gcloud auth login",
          "description": "Login to Google Cloud",
          "continue_on_error": false,
          "step_type": "Auth"
        },
        {
          "name": "Setup Application Default Credentials",
          "command": "gcloud auth application-default login",
          "description": "Setup application default credentials",
          "continue_on_error": false,
          "step_type": "Auth"
        }
      ]
    }
  },
  {
    "name": "Set Project",
    "command": "gcloud config set project {{ project_id }}",
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

### Port Killing Workflow with Conditional Logic

```json
[
  {
    "name": "Find Process",
    "command": "lsof -ti tcp:{{ port }}",
    "description": "Find process running on specified port",
    "continue_on_error": true,
    "step_type": "Command"
  },
  {
    "name": "Check Process Exists",
    "command": "$? -eq 0",
    "description": "Check if process exists on port",
    "continue_on_error": false,
    "step_type": "Conditional",
    "conditional": {
      "then_steps": [
        {
          "name": "Kill Process",
          "command": "kill -9 $(lsof -ti tcp:{{ port }})",
          "description": "Kill process on port",
          "continue_on_error": false,
          "step_type": "Command",
          "require_approval": true
        },
        {
          "name": "Confirm",
          "command": "echo \"Process on port {{ port }} killed successfully\"",
          "description": "Confirm process was killed",
          "continue_on_error": false,
          "step_type": "Command"
        }
      ],
      "else_steps": [
        {
          "name": "No Process",
          "command": "echo \"No process found running on port {{ port }}\"",
          "description": "Report no process found",
          "continue_on_error": false,
          "step_type": "Command"
        }
      ]
    }
  }
]
```

### Environment Selection with Branch Logic

```json
[
  {
    "name": "Set Environment",
    "command": "",
    "description": "Configure environment based on parameter",
    "continue_on_error": false,
    "step_type": "Branch",
    "branch": {
      "variable": "env",
      "cases": {
        "dev": [
          {
            "name": "Set Dev Project",
            "command": "gcloud config set project dev-project",
            "description": "Set development project",
            "continue_on_error": false,
            "step_type": "Command"
          },
          {
            "name": "Get Dev Credentials",
            "command": "gcloud container clusters get-credentials dev-cluster --zone=us-central1-a",
            "description": "Get development cluster credentials",
            "continue_on_error": false,
            "step_type": "Command"
          }
        ],
        "prod": [
          {
            "name": "Set Prod Project",
            "command": "gcloud config set project prod-project",
            "description": "Set production project",
            "continue_on_error": false,
            "step_type": "Command",
            "require_approval": true
          },
          {
            "name": "Get Prod Credentials",
            "command": "gcloud container clusters get-credentials prod-cluster --zone=us-central1-a",
            "description": "Get production cluster credentials",
            "continue_on_error": false,
            "step_type": "Command"
          }
        ]
      },
      "default_case": [
        {
          "name": "Show Usage",
          "command": "echo \"Usage: Please provide 'dev' or 'prod' as environment\"",
          "description": "Show usage information",
          "continue_on_error": false,
          "step_type": "Command"
        }
      ]
    }
  }
]
```

### Loop Example for Processing Files

```json
[
  {
    "name": "Process Files",
    "command": "",
    "description": "Process all log files",
    "continue_on_error": false,
    "step_type": "Loop",
    "loop_data": {
      "variable": "file",
      "values": "$(ls *.log)",
      "steps": [
        {
          "name": "Process File",
          "command": "cat {{ file }} | grep ERROR >> errors.txt",
          "description": "Extract errors from log file",
          "continue_on_error": true,
          "step_type": "Command"
        },
        {
          "name": "Report",
          "command": "echo \"Processed {{ file }}\"",
          "description": "Report progress",
          "continue_on_error": false,
          "step_type": "Command"
        }
      ]
    }
  },
  {
    "name": "Summarize",
    "command": "wc -l errors.txt",
    "description": "Count total errors found",
    "continue_on_error": false,
    "step_type": "Command"
  }
]
```