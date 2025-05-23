# Clix Usage Guide

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/clix.git

# Build and install
cd clix
cargo install --path .
```

## Git Repository Integration (Team Sharing)

### Quick start for team sharing

```bash
# Add your team's shared command repository
clix git add-repo team-commands --url https://github.com/your-team/shared-commands.git

# Check repository status
clix git status

# List all configured repositories
clix git list-repos

# Manually pull latest changes
clix git pull
```

Note: Clix automatically syncs with all configured repositories when you run any command.

## Basic Commands

### Adding a command

```bash
clix add list-files --description "Lists files in the current directory" --command "ls -la"
```

### Running a command

```bash
clix run list-files
```

### Listing all commands

```bash
clix list
```

### Filtering commands by tag

```bash
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
    "continue_on_error": false
  },
  {
    "name": "Show Current Directory",
    "command": "pwd",
    "description": "Print the current working directory",
    "continue_on_error": true
  }
]
```

### Adding a workflow

```bash
clix flow add my-workflow --description "Basic system info workflow" --steps-file workflow.json
```

### Running a workflow

```bash
clix flow run my-workflow
```

## Sharing Commands and Workflows

### Exporting Commands and Workflows

You can export your commands and workflows to share with your team:

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

You can import commands and workflows that were exported by another user:

```bash
# Import commands and workflows
clix import --input my-commands.json

# Import and overwrite any existing commands with the same name
clix import --input team-commands.json --overwrite
```

## Examples

### Cloud Service Troubleshooting Workflow

Create a workflow JSON file:

```json
[
  {
    "name": "Get GCP Service Status",
    "command": "gcloud app services list",
    "description": "List all GCP App Engine services",
    "continue_on_error": false
  },
  {
    "name": "Fetch Recent Logs",
    "command": "gcloud app logs read --service=my-service --limit=50",
    "description": "Fetch the last 50 log entries for the service",
    "continue_on_error": true
  },
  {
    "name": "Restart Service",
    "command": "gcloud app services stop my-service && gcloud app services start my-service",
    "description": "Restart the service by stopping and starting it",
    "continue_on_error": true
  }
]
```

Add and run the workflow:

```bash
clix flow add gcp-troubleshoot --description "GCP service troubleshooting" --steps-file gcp-workflow.json --tags cloud,gcp
clix flow run gcp-troubleshoot
```
