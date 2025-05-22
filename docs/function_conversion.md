# Shell Function Conversion

Clix can automatically convert your shell functions (from `.bashrc`, `.zshrc`, or other shell script files) into workflows. This document explains how the function conversion works and how to use it effectively.

## Overview

Shell functions are a powerful way to create reusable command sequences, but they have limitations:

1. They're specific to a particular shell (bash, zsh, etc.)
2. They're difficult to share with team members
3. They don't provide built-in features like variable management or approval requirements
4. They don't track usage statistics or provide organization features like tagging

Clix's function conversion feature addresses these limitations by transforming your shell functions into workflows that can:

- Run consistently across different environments
- Be easily shared through import/export
- Leverage Clix's variable management, profiles, and approval features
- Be organized with tags and usage tracking

## Basic Usage

The basic syntax for converting a shell function is:

```bash
clix flow convert-function --file <path-to-script-file> --function <function-name> --name <workflow-name> --description <description>
```

### Required Parameters

- `--file`: The path to the shell script file containing the function
- `--function`: The name of the function to convert
- `--name`: The name to give the resulting workflow
- `--description`: A description of what the workflow does

### Optional Parameters

- `--tags`: Comma-separated list of tags for the workflow
- `--approve-dangerous`: Automatically add approval requirements for potentially destructive operations
- `--var-defaults`: Comma-separated list of default values for variables in format `name=value`

## How It Works

The function converter performs these steps:

1. **Parse the shell script** to locate the specified function
2. **Analyze the function structure** to identify control flow, variables, and commands
3. **Map shell constructs to workflow steps**:
   - Command sequences → Command steps
   - If/then/else → Conditional steps
   - Case statements → Branch steps
   - For/while loops → Loop steps
   - Function parameters → Workflow variables
4. **Create a new workflow** with the mapped steps

## Supported Shell Constructs

### 1. Command Sequences

Simple command sequences are converted to sequential Command steps:

```bash
deploy_frontend() {
  npm test
  npm run build
  aws s3 sync ./build s3://my-website
}
```

Converts to:

```json
[
  {
    "name": "Run Tests",
    "command": "npm test",
    "description": "Run the test suite",
    "continue_on_error": false,
    "step_type": "Command"
  },
  {
    "name": "Build",
    "command": "npm run build",
    "description": "Build the application",
    "continue_on_error": false,
    "step_type": "Command"
  },
  {
    "name": "Deploy",
    "command": "aws s3 sync ./build s3://my-website",
    "description": "Deploy to S3",
    "continue_on_error": false,
    "step_type": "Command"
  }
]
```

### 2. If/Then/Else Statements

If/then/else structures are converted to Conditional steps:

```bash
deploy_backend() {
  if [ ! -f .env ]; then
    echo "Error: .env file not found"
    return 1
  fi
  
  npm run build
  docker build -t backend:latest .
}
```

Converts to:

```json
[
  {
    "name": "Check Environment File",
    "command": "[ ! -f .env ]",
    "description": "Check if .env file exists",
    "continue_on_error": false,
    "step_type": "Conditional",
    "conditional": {
      "then_steps": [
        {
          "name": "Error Message",
          "command": "echo \"Error: .env file not found\"",
          "description": "Display error message",
          "continue_on_error": false,
          "step_type": "Command"
        }
      ],
      "action": "return",
      "exit_code": 1
    }
  },
  {
    "name": "Build Application",
    "command": "npm run build",
    "description": "Build the application",
    "continue_on_error": false,
    "step_type": "Command"
  },
  {
    "name": "Build Docker Image",
    "command": "docker build -t backend:latest .",
    "description": "Build Docker image",
    "continue_on_error": false,
    "step_type": "Command"
  }
]
```

### 3. Case Statements

Case statements are converted to Branch steps:

```bash
set_environment() {
  local env="$1"
  
  case "$env" in
    dev)
      export API_URL="https://dev-api.example.com"
      ;;
    staging)
      export API_URL="https://staging-api.example.com"
      ;;
    prod)
      export API_URL="https://api.example.com"
      ;;
    *)
      echo "Unknown environment: $env"
      echo "Usage: set_environment [dev|staging|prod]"
      return 1
      ;;
  esac
  
  echo "Environment set to $env"
}
```

Converts to:

```json
[
  {
    "name": "Set Environment",
    "command": "",
    "description": "Configure environment settings",
    "continue_on_error": false,
    "step_type": "Branch",
    "branch": {
      "variable": "env",
      "cases": {
        "dev": [
          {
            "name": "Set Dev API URL",
            "command": "export API_URL=\"https://dev-api.example.com\"",
            "description": "Set development API URL",
            "continue_on_error": false,
            "step_type": "Command"
          }
        ],
        "staging": [
          {
            "name": "Set Staging API URL",
            "command": "export API_URL=\"https://staging-api.example.com\"",
            "description": "Set staging API URL",
            "continue_on_error": false,
            "step_type": "Command"
          }
        ],
        "prod": [
          {
            "name": "Set Production API URL",
            "command": "export API_URL=\"https://api.example.com\"",
            "description": "Set production API URL",
            "continue_on_error": false,
            "step_type": "Command"
          }
        ]
      },
      "default_case": [
        {
          "name": "Unknown Environment",
          "command": "echo \"Unknown environment: $env\"\necho \"Usage: set_environment [dev|staging|prod]\"",
          "description": "Show usage information",
          "continue_on_error": false,
          "step_type": "Command"
        }
      ],
      "action": "return",
      "exit_code": 1
    }
  },
  {
    "name": "Confirm Environment",
    "command": "echo \"Environment set to {{ env }}\"",
    "description": "Display confirmation message",
    "continue_on_error": false,
    "step_type": "Command"
  }
]
```

### 4. For and While Loops

Loops are converted to Loop steps:

```bash
process_logs() {
  for logfile in *.log; do
    echo "Processing $logfile..."
    grep ERROR "$logfile" >> errors.txt
    grep WARNING "$logfile" >> warnings.txt
  done
  
  echo "Processing complete"
}
```

Converts to:

```json
[
  {
    "name": "Process Log Files",
    "command": "",
    "description": "Process all log files",
    "continue_on_error": false,
    "step_type": "Loop",
    "loop_data": {
      "variable": "logfile",
      "values": "$(ls *.log)",
      "steps": [
        {
          "name": "Processing Notification",
          "command": "echo \"Processing {{ logfile }}...\"",
          "description": "Display processing notification",
          "continue_on_error": false,
          "step_type": "Command"
        },
        {
          "name": "Extract Errors",
          "command": "grep ERROR \"{{ logfile }}\" >> errors.txt",
          "description": "Extract errors to file",
          "continue_on_error": true,
          "step_type": "Command"
        },
        {
          "name": "Extract Warnings",
          "command": "grep WARNING \"{{ logfile }}\" >> warnings.txt",
          "description": "Extract warnings to file",
          "continue_on_error": true,
          "step_type": "Command"
        }
      ]
    }
  },
  {
    "name": "Completion Message",
    "command": "echo \"Processing complete\"",
    "description": "Display completion message",
    "continue_on_error": false,
    "step_type": "Command"
  }
]
```

### 5. Function Parameters

Function parameters are converted to workflow variables:

```bash
deploy_service() {
  local service_name="$1"
  local version="$2"
  local env="${3:-dev}"
  
  echo "Deploying $service_name version $version to $env..."
  kubectl set image deployment/$service_name $service_name=$service_name:$version -n $env
}
```

Converts to a workflow with variables:

```json
{
  "name": "deploy_service",
  "description": "Deploy service to Kubernetes",
  "steps": [
    {
      "name": "Deploy Service",
      "command": "echo \"Deploying {{ service_name }} version {{ version }} to {{ env }}...\"\nkubectl set image deployment/{{ service_name }} {{ service_name }}={{ service_name }}:{{ version }} -n {{ env }}",
      "description": "Deploy service to Kubernetes",
      "continue_on_error": false,
      "step_type": "Command"
    }
  ],
  "variables": [
    {
      "name": "service_name",
      "description": "Name of the service to deploy",
      "required": true
    },
    {
      "name": "version",
      "description": "Version tag to deploy",
      "required": true
    },
    {
      "name": "env",
      "description": "Environment to deploy to",
      "default_value": "dev",
      "required": false
    }
  ]
}
```

## Automatic Approval Requirements

When using the `--approve-dangerous` flag, the function converter automatically adds approval requirements to steps that:

1. Delete files or directories (using `rm`, `rmdir`, etc.)
2. Stop, delete, or restart services (`systemctl stop`, `docker rm`, etc.)
3. Modify production environments (when the command contains "prod" and actions like "apply", "update", etc.)
4. Execute potentially harmful commands (`kubectl delete`, `gcloud projects delete`, etc.)

Example:

```bash
cleanup() {
  rm -rf ./build
  docker rmi -f myapp:latest
  kubectl delete namespace test
}
```

With `--approve-dangerous`, converts to:

```json
[
  {
    "name": "Remove Build Directory",
    "command": "rm -rf ./build",
    "description": "Delete build artifacts",
    "continue_on_error": false,
    "step_type": "Command",
    "require_approval": true
  },
  {
    "name": "Remove Docker Image",
    "command": "docker rmi -f myapp:latest",
    "description": "Delete Docker image",
    "continue_on_error": false,
    "step_type": "Command",
    "require_approval": true
  },
  {
    "name": "Delete Kubernetes Namespace",
    "command": "kubectl delete namespace test",
    "description": "Delete test namespace",
    "continue_on_error": false,
    "step_type": "Command",
    "require_approval": true
  }
]
```

## Limitations and Considerations

1. **Complex Shell Scripts**: Very complex shell scripts with advanced features may not convert perfectly. In such cases, you may need to manually adjust the workflow after conversion.

2. **Shell-Specific Features**: Shell-specific features (like bash arrays or zsh associative arrays) might not convert correctly to workflows.

3. **External Dependencies**: Functions that rely on other functions or external scripts will need those dependencies available when running the workflow.

4. **Environment Variables**: Functions that rely on environment variables will need those variables to be set when running the workflow.

5. **Interactive Commands**: Functions that require interactive input should be converted to use the Auth step type for proper interaction.

## Best Practices

1. **Start Simple**: Begin by converting simpler functions before attempting complex ones.

2. **Test After Conversion**: Always test the converted workflow to ensure it behaves as expected.

3. **Review and Refine**: Review the generated workflow and refine it to take advantage of Clix features.

4. **Add Variables**: Consider adding more descriptive variables to make the workflow more flexible.

5. **Add Approvals**: Add approval requirements for sensitive operations, especially in production environments.

6. **Use Profiles**: Create profiles for different environments to make the workflow more reusable.

## Examples

### Example 1: Simple Deployment Function

```bash
# Original Shell Function
deploy_frontend() {
  npm test
  if [ $? -ne 0 ]; then
    echo "Tests failed, aborting deployment"
    return 1
  fi
  
  npm run build
  aws s3 sync ./build s3://my-website
  aws cloudfront create-invalidation --distribution-id ABCDEF12345 --paths "/*"
  echo "Deployment complete"
}

# Conversion Command
clix flow convert-function --file ./scripts.sh --function deploy_frontend --name frontend-deploy --description "Deploy frontend to S3 and invalidate CloudFront" --tags aws,frontend
```

### Example 2: Function with Parameters

```bash
# Original Shell Function
kube_logs() {
  local pod_prefix="$1"
  local namespace="${2:-default}"
  local tail="${3:-100}"
  
  if [ -z "$pod_prefix" ]; then
    echo "Usage: kube_logs <pod-prefix> [namespace] [tail-lines]"
    return 1
  fi
  
  local pod=$(kubectl get pods -n "$namespace" | grep "^$pod_prefix" | head -1 | awk '{print $1}')
  
  if [ -z "$pod" ]; then
    echo "No pod found with prefix '$pod_prefix' in namespace '$namespace'"
    return 1
  fi
  
  echo "Showing logs for pod $pod in namespace $namespace..."
  kubectl logs -n "$namespace" "$pod" --tail="$tail" -f
}

# Conversion Command
clix flow convert-function --file ./k8s_functions.sh --function kube_logs --name kubernetes-logs --description "View logs from Kubernetes pods" --tags kubernetes,logs --var-defaults namespace=default,tail=100
```

### Example 3: Complex Function with Case Statement

```bash
# Original Shell Function
db_connect() {
  local env="$1"
  local db_name="${2:-main}"
  
  if [ -z "$env" ]; then
    echo "Usage: db_connect [dev|staging|prod] [db-name]"
    return 1
  fi
  
  case "$env" in
    dev)
      echo "Connecting to dev database $db_name..."
      psql "postgresql://dev_user:dev_pass@localhost:5432/$db_name"
      ;;
    staging)
      echo "Connecting to staging database $db_name..."
      psql "postgresql://staging_user:staging_pass@staging-db:5432/$db_name"
      ;;
    prod)
      echo "WARNING: Connecting to PRODUCTION database $db_name"
      echo "Are you sure? (y/N)"
      read confirm
      if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
        echo "Connection cancelled"
        return 1
      fi
      psql "postgresql://prod_user:prod_pass@prod-db:5432/$db_name"
      ;;
    *)
      echo "Unknown environment: $env"
      echo "Usage: db_connect [dev|staging|prod] [db-name]"
      return 1
      ;;
  esac
}

# Conversion Command
clix flow convert-function --file ./db_functions.sh --function db_connect --name database-connect --description "Connect to database in different environments" --tags database,psql --approve-dangerous
```

In this example, the approval requirement for the production case is automatically added due to the `--approve-dangerous` flag detecting the production environment.