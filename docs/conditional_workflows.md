# Conditional Workflows

Clix supports advanced workflow features including conditionals, branching, and loops. This document explains how to use these features to create more powerful and flexible workflows.

## Overview

Conditional workflows enable you to create sophisticated automation by adding logic to your workflows. You can:

1. Execute steps only if certain conditions are met (conditional steps)
2. Choose different execution paths based on variable values (branch steps)
3. Create loops that repeat steps multiple times (loop steps)
4. Return early from workflows with specific exit codes
5. Build complex, function-like workflows that accept parameters

These features allow you to build powerful, reusable workflows that can adapt to different inputs and situations.

## Conditional Steps (If/Then/Else)

A conditional step executes a block of steps only if a condition is true. It's similar to an "if/then/else" statement in programming.

### Conditional Step JSON Structure

```json
{
  "name": "Check File Exists",
  "command": "[ -f config.yaml ]",
  "description": "Check if config file exists",
  "continue_on_error": false,
  "step_type": "Conditional",
  "conditional": {
    "then_steps": [
      {
        "name": "Read Config",
        "command": "cat config.yaml",
        "description": "Display config contents",
        "continue_on_error": false,
        "step_type": "Command"
      }
    ],
    "else_steps": [
      {
        "name": "Create Config",
        "command": "echo 'default: true' > config.yaml",
        "description": "Create default config file",
        "continue_on_error": false,
        "step_type": "Command"
      }
    ]
  }
}
```

Key elements:
- `command`: Contains the condition to evaluate
- `step_type`: Must be set to `"Conditional"`
- `conditional`: Contains `then_steps` and optional `else_steps`
- `then_steps`: Array of steps to execute if the condition is true
- `else_steps`: Array of steps to execute if the condition is false (optional)

### Example: Authentication Check

```json
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
}
```

This workflow step checks if the user is already authenticated with Google Cloud. If not, it executes the steps to log in.

### Adding a Conditional Step via CLI

You can add a conditional step to a workflow using the CLI:

```bash
# First, create files containing the then/else steps
cat > then_steps.json << 'EOF'
[
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
EOF

# Add the conditional step to the workflow
clix flow add-condition my-workflow \
  --name "Check Authentication" \
  --description "Check if already authenticated with GCloud" \
  --condition "! gcloud auth application-default print-access-token > /dev/null 2>&1" \
  --then-file then_steps.json
```

### Adding an Else Block

You can also add an else block that executes when the condition is false:

```bash
# Create the else steps file
cat > create_config.json << 'EOF'
[
  {
    "name": "Create Config",
    "command": "echo 'default: true' > config.yaml",
    "description": "Create default config file",
    "continue_on_error": false,
    "step_type": "Command"
  }
]
EOF

# Add the conditional step with both then and else blocks
clix flow add-condition my-workflow \
  --name "Check File Exists" \
  --description "Check if config file exists" \
  --condition "[ -f config.yaml ]" \
  --then-file config_exists.json \
  --else-file create_config.json
```

## Branch Steps (Switch/Case)

Branch steps execute different blocks of steps based on a variable value, similar to switch/case statements in programming languages.

### Branch Step JSON Structure

```json
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
```

Key elements:
- `step_type`: Must be set to `"Branch"`
- `branch`: Contains the branching configuration
- `variable`: The variable to evaluate
- `cases`: Object mapping variable values to arrays of steps
- `default_case`: Array of steps to execute if no case matches (optional)

### Example: Environment Selection

```json
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
          "command": "gcloud container clusters get-credentials dev-gke-cluster --zone=us-central1-a",
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
          "command": "gcloud container clusters get-credentials prod-gke-cluster --zone=us-central1-a",
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
```

This workflow step selects different commands to run based on the `env` variable value.

### Adding a Branch Step via CLI

You can add a branch step using the CLI:

```bash
# First, create a file with case steps
cat > cases.json << 'EOF'
{
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
      "command": "gcloud container clusters get-credentials dev-gke-cluster --zone=us-central1-a",
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
      "command": "gcloud container clusters get-credentials prod-gke-cluster --zone=us-central1-a",
      "description": "Get production cluster credentials",
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

## Loop Steps

Loop steps allow you to repeat a set of commands multiple times, processing lists of items or iterating over patterns.

### Loop Step JSON Structure

```json
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
      }
    ]
  }
}
```

Key elements:
- `step_type`: Must be set to `"Loop"`
- `loop_data`: Contains the loop configuration
- `variable`: The iteration variable name
- `values`: The values to iterate over (can be a shell command in `$()` syntax)
- `steps`: Array of steps to execute for each value

### Example: Process Multiple Files

```json
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
        "name": "Processing Notification",
        "command": "echo \"Processing {{ file }}...\"",
        "description": "Display processing notification",
        "continue_on_error": false,
        "step_type": "Command"
      },
      {
        "name": "Extract Errors",
        "command": "cat {{ file }} | grep ERROR >> errors.txt",
        "description": "Extract errors from log file",
        "continue_on_error": true,
        "step_type": "Command"
      },
      {
        "name": "Extract Warnings",
        "command": "cat {{ file }} | grep WARNING >> warnings.txt",
        "description": "Extract warnings from log file",
        "continue_on_error": true,
        "step_type": "Command"
      }
    ]
  }
}
```

This workflow step processes all `.log` files in the current directory, extracting errors and warnings.

### Adding a Loop Step via CLI

You can add a loop step using the CLI:

```bash
# First, create a file with loop steps
cat > loop_steps.json << 'EOF'
[
  {
    "name": "Processing Notification",
    "command": "echo \"Processing {{ file }}...\"",
    "description": "Display processing notification",
    "continue_on_error": false,
    "step_type": "Command"
  },
  {
    "name": "Extract Errors",
    "command": "cat {{ file }} | grep ERROR >> errors.txt",
    "description": "Extract errors from log file",
    "continue_on_error": true,
    "step_type": "Command"
  },
  {
    "name": "Extract Warnings",
    "command": "cat {{ file }} | grep WARNING >> warnings.txt",
    "description": "Extract warnings from log file",
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

## Condition Expressions

Conditions in Clix support a wide range of expressions for conditional steps:

### Exit Code Checks

- `$? -eq 0` - Check if previous command succeeded
- `$? -ne 0` - Check if previous command failed
- `$? -gt 0` - Check if previous command failed with exit code greater than 0
- `$? -lt 128` - Check if previous command failed with exit code less than 128

### File Tests

- `[ -f file.txt ]` - Check if file exists
- `[ -d directory ]` - Check if directory exists
- `[ -r file.txt ]` - Check if file is readable
- `[ -w file.txt ]` - Check if file is writable
- `[ -x file.txt ]` - Check if file is executable
- `[ -s file.txt ]` - Check if file exists and is not empty
- `[ -L file.txt ]` - Check if file is a symbolic link

### String Tests

- `[ -z "$var" ]` - Check if variable is empty
- `[ -n "$var" ]` - Check if variable is not empty
- `[ "$var" = "value" ]` - Check if variable equals a value
- `[ "$var" != "value" ]` - Check if variable does not equal a value
- `[[ "$var" =~ pattern ]]` - Check if variable matches a regex pattern

### Numeric Comparisons

- `[ "$num" -eq 10 ]` - Check if number equals 10
- `[ "$num" -ne 10 ]` - Check if number does not equal 10
- `[ "$num" -gt 10 ]` - Check if number is greater than 10
- `[ "$num" -lt 10 ]` - Check if number is less than 10
- `[ "$num" -ge 10 ]` - Check if number is greater than or equal to 10
- `[ "$num" -le 10 ]` - Check if number is less than or equal to 10

### Logical Operators

- `condition1 && condition2` - Logical AND
- `condition1 || condition2` - Logical OR
- `! condition` - Logical NOT

### Command Execution

- `command && echo "Command succeeded"` - Run command and check success
- `command || echo "Command failed"` - Run command and check failure
- `$(command) = "expected"` - Check if command output equals expected value

## Converting Shell Functions

You can convert existing shell functions to Clix workflows with conditionals:

```bash
clix flow convert-function \
  --file ~/.zshrc \
  --function gke \
  --name gke-workflow \
  --description "Switch to a GKE cluster environment" \
  --tags gcloud,kubernetes
```

This will analyze the shell function and create a workflow that includes any conditional logic, branching, or loops in the original function. See the [Function Conversion Documentation](function_conversion.md) for more details.

## Complete Workflow Examples

### Example 1: Port Killing Workflow with Conditional Logic

```json
{
  "name": "killport",
  "description": "Kill process running on a specific port",
  "variables": [
    {
      "name": "port",
      "description": "Port number to kill",
      "required": true
    }
  ],
  "steps": [
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
}
```

This workflow:
1. Attempts to find a process running on the specified port
2. Checks if a process was found
3. If found, asks for approval and kills the process
4. If not found, displays a message that no process was found

### Example 2: Deployment Workflow with Authentication Check

```json
{
  "name": "deploy-app",
  "description": "Deploy application to GKE with authentication check",
  "variables": [
    {
      "name": "env",
      "description": "Environment to deploy to (dev, staging, prod)",
      "required": true
    },
    {
      "name": "version",
      "description": "Version to deploy",
      "required": true
    }
  ],
  "steps": [
    {
      "name": "Check Authentication",
      "command": "! gcloud auth application-default print-access-token > /dev/null 2>&1",
      "description": "Check if authenticated with GCloud",
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
      "name": "Select Environment",
      "command": "",
      "description": "Configure environment settings",
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
          "staging": [
            {
              "name": "Set Staging Project",
              "command": "gcloud config set project staging-project",
              "description": "Set staging project",
              "continue_on_error": false,
              "step_type": "Command"
            },
            {
              "name": "Get Staging Credentials",
              "command": "gcloud container clusters get-credentials staging-cluster --zone=us-central1-a",
              "description": "Get staging cluster credentials",
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
            "name": "Unknown Environment",
            "command": "echo \"Unknown environment: {{ env }}\"\necho \"Valid environments: dev, staging, prod\"",
            "description": "Report unknown environment",
            "continue_on_error": false,
            "step_type": "Command"
          }
        ]
      }
    },
    {
      "name": "Deploy Application",
      "command": "kubectl set image deployment/myapp myapp=myapp:{{ version }} && kubectl rollout status deployment/myapp",
      "description": "Deploy and monitor rollout",
      "continue_on_error": false,
      "step_type": "Command"
    }
  ]
}
```

This workflow:
1. Checks if the user is authenticated with Google Cloud
2. If not authenticated, performs the authentication steps
3. Configures the environment based on the `env` variable
4. Deploys the application with the specified version

### Example 3: Data Processing with Loops

```json
{
  "name": "process-data",
  "description": "Process data files with loop",
  "steps": [
    {
      "name": "Check Directory",
      "command": "[ -d data ]",
      "description": "Check if data directory exists",
      "continue_on_error": false,
      "step_type": "Conditional",
      "conditional": {
        "then_steps": [],
        "else_steps": [
          {
            "name": "Create Directory",
            "command": "mkdir -p data",
            "description": "Create data directory",
            "continue_on_error": false,
            "step_type": "Command"
          }
        ]
      }
    },
    {
      "name": "Create Output Directory",
      "command": "mkdir -p output",
      "description": "Create output directory",
      "continue_on_error": false,
      "step_type": "Command"
    },
    {
      "name": "Process Files",
      "command": "",
      "description": "Process all CSV files",
      "continue_on_error": false,
      "step_type": "Loop",
      "loop_data": {
        "variable": "file",
        "values": "$(ls data/*.csv)",
        "steps": [
          {
            "name": "Extract Filename",
            "command": "basename \"{{ file }}\" .csv",
            "description": "Get filename without extension",
            "continue_on_error": false,
            "step_type": "Conditional",
            "conditional": {
              "then_steps": [],
              "variable": "basename"
            }
          },
          {
            "name": "Process File",
            "command": "cat \"{{ file }}\" | sort | uniq > \"output/{{ basename }}_processed.csv\"",
            "description": "Process file and save result",
            "continue_on_error": true,
            "step_type": "Command"
          },
          {
            "name": "Report",
            "command": "echo \"Processed {{ file }} -> output/{{ basename }}_processed.csv\"",
            "description": "Report progress",
            "continue_on_error": false,
            "step_type": "Command"
          }
        ]
      }
    },
    {
      "name": "Summarize",
      "command": "ls -la output/",
      "description": "List processed files",
      "continue_on_error": false,
      "step_type": "Command"
    }
  ]
}
```

This workflow:
1. Checks if the data directory exists, creating it if needed
2. Creates an output directory
3. Processes all CSV files in the data directory:
   - Extracts the basename for use in the output filename
   - Processes each file (sort and remove duplicates)
   - Reports progress for each file
4. Lists all processed files at the end

## Advanced Techniques

### Variable Capture

You can capture command output into variables:

```json
{
  "name": "Capture Version",
  "command": "kubectl version --short",
  "description": "Get Kubernetes version",
  "continue_on_error": false,
  "step_type": "Conditional",
  "conditional": {
    "then_steps": [],
    "variable": "k8s_version"
  }
}
```

The output of the command will be stored in the `k8s_version` variable for use in later steps.

### Early Return

You can return early from a workflow with a specific exit code:

```json
{
  "name": "Check Requirements",
  "command": "! command -v kubectl > /dev/null",
  "description": "Check if kubectl is installed",
  "continue_on_error": false,
  "step_type": "Conditional",
  "conditional": {
    "then_steps": [
      {
        "name": "Error Message",
        "command": "echo \"kubectl is required but not installed\"",
        "description": "Display error message",
        "continue_on_error": false,
        "step_type": "Command"
      }
    ],
    "action": "return",
    "exit_code": 1
  }
}
```

If kubectl is not installed, the workflow displays an error message and exits with code 1.

### Nested Conditionals

You can nest conditionals within other conditionals or branches:

```json
{
  "name": "Check Environment",
  "command": "[ \"$env\" = \"prod\" ]",
  "description": "Check if production environment",
  "continue_on_error": false,
  "step_type": "Conditional",
  "conditional": {
    "then_steps": [
      {
        "name": "Check Authorization",
        "command": "[ -f ~/.prod_authorized ]",
        "description": "Check if user is authorized for production",
        "continue_on_error": false,
        "step_type": "Conditional",
        "conditional": {
          "then_steps": [
            {
              "name": "Deploy to Production",
              "command": "kubectl apply -f prod-deployment.yaml",
              "description": "Deploy to production",
              "continue_on_error": false,
              "step_type": "Command",
              "require_approval": true
            }
          ],
          "else_steps": [
            {
              "name": "Authorization Error",
              "command": "echo \"You are not authorized to deploy to production\"",
              "description": "Display authorization error",
              "continue_on_error": false,
              "step_type": "Command"
            }
          ],
          "action": "return",
          "exit_code": 1
        }
      }
    ],
    "else_steps": [
      {
        "name": "Deploy to Non-Production",
        "command": "kubectl apply -f {{ env }}-deployment.yaml",
        "description": "Deploy to non-production environment",
        "continue_on_error": false,
        "step_type": "Command"
      }
    ]
  }
}
```

## Best Practices

1. **Keep conditions simple**: Use simple expressions for better readability and maintainability.

2. **Provide descriptive names**: Name your steps and conditions clearly to make the workflow self-documenting.

3. **Test thoroughly**: Test conditional workflows with different inputs and edge cases to ensure they behave as expected.

4. **Use variables**: Leverage variables for flexible workflows, making them reusable across different environments or situations.

5. **Add error handling**: Use `continue_on_error` for steps that can fail safely without aborting the entire workflow.

6. **Add approvals for sensitive operations**: Require explicit approval for potentially destructive operations, especially in production environments.

7. **Group related steps**: Keep related steps together to improve readability and maintainability.

8. **Document expected inputs**: Clearly document any required variables and their expected values in the workflow description.

9. **Consider output visibility**: Use echo commands strategically to provide feedback to users about workflow progress and decisions.

10. **Avoid deep nesting**: Overly nested conditional structures can be hard to understand. Consider breaking complex workflows into smaller, more manageable pieces.