# Conditional Workflows

Clix supports advanced workflow features including conditionals, branching, and loops. This document explains how to use these features to create more powerful and flexible workflows.

## Overview

Conditional workflows enable you to create sophisticated automation by adding logic to your workflows. You can:

1. Execute steps only if certain conditions are met
2. Choose different execution paths based on variable values
3. Create loops that repeat steps until a condition is met
4. Return early from workflows with specific exit codes
5. Build complex, function-like workflows that accept parameters

## Conditional Steps

A conditional step executes a block of steps only if a condition is true. It's similar to an "if/then/else" statement in programming.

### Example: Authentication Check

```json
{
  "name": "Check Authentication",
  "type": "conditional",
  "condition": "! gcloud auth application-default print-access-token > /dev/null 2>&1",
  "then": [
    {
      "name": "Login to GCloud",
      "type": "auth",
      "command": "gcloud auth login"
    },
    {
      "name": "Setup Application Default Credentials",
      "type": "auth",
      "command": "gcloud auth application-default login"
    }
  ]
}
```

### Adding a Conditional Step

You can add a conditional step to a workflow using the CLI:

```bash
clix flow add-condition my-workflow \
  --name "Check Authentication" \
  --description "Check if already authenticated with GCloud" \
  --condition "! gcloud auth application-default print-access-token > /dev/null 2>&1" \
  --then-file then_steps.json
```

Where `then_steps.json` contains:

```json
[
  {
    "name": "Login to GCloud",
    "type": "auth",
    "command": "gcloud auth login"
  },
  {
    "name": "Setup Application Default Credentials",
    "type": "auth",
    "command": "gcloud auth application-default login"
  }
]
```

### Adding an Else Block

You can also add an else block that executes when the condition is false:

```bash
clix flow add-condition my-workflow \
  --name "Check File Exists" \
  --description "Check if config file exists" \
  --condition "[ -f config.yaml ]" \
  --then-file config_exists.json \
  --else-file create_config.json
```

## Branch Steps

Branch steps (similar to switch/case statements) execute different blocks of steps based on a variable value.

### Example: Environment Selection

```json
{
  "name": "Set Environment",
  "type": "branch",
  "variable": "env",
  "cases": {
    "dev": [
      {
        "name": "Set Dev Project",
        "type": "command",
        "command": "gcloud config set project dev-project"
      },
      {
        "name": "Get Dev Credentials",
        "type": "command",
        "command": "gcloud container clusters get-credentials dev-gke-cluster --zone=us-central1-a"
      }
    ],
    "prod": [
      {
        "name": "Set Prod Project",
        "type": "command",
        "command": "gcloud config set project prod-project"
      },
      {
        "name": "Get Prod Credentials",
        "type": "command",
        "command": "gcloud container clusters get-credentials prod-gke-cluster --zone=us-central1-a"
      }
    ],
    "default": [
      {
        "name": "Show Usage",
        "type": "command",
        "command": "echo \"Usage: gke [dev|prod]\""
      }
    ]
  }
}
```

### Adding a Branch Step

You can add a branch step using the CLI:

```bash
clix flow add-branch my-workflow \
  --name "Set Environment" \
  --description "Configure environment based on parameter" \
  --variable "env" \
  --cases-file cases.json \
  --default-file default.json
```

Where `cases.json` contains:

```json
{
  "dev": [
    {
      "name": "Set Dev Project",
      "type": "command",
      "command": "gcloud config set project dev-project"
    }
  ],
  "prod": [
    {
      "name": "Set Prod Project",
      "type": "command",
      "command": "gcloud config set project prod-project"
    }
  ]
}
```

## Condition Expressions

Conditions in Clix support a wide range of expressions:

### Exit Code Checks

- `$? -eq 0` - Check if previous command succeeded
- `$? -ne 0` - Check if previous command failed

### File Tests

- `[ -f file.txt ]` - Check if file exists
- `[ -d directory ]` - Check if directory exists
- `[ -r file.txt ]` - Check if file is readable
- `[ -w file.txt ]` - Check if file is writable
- `[ -x file.txt ]` - Check if file is executable

### String Tests

- `[ -z "$var" ]` - Check if variable is empty
- `[ -n "$var" ]` - Check if variable is not empty
- `[ "$var" = "value" ]` - Check if variable equals a value

### Logical Operators

- `condition1 && condition2` - Logical AND
- `condition1 || condition2` - Logical OR
- `! condition` - Logical NOT

## Converting Shell Functions

You can convert existing shell functions to Clix workflows with conditionals:

```bash
clix flow convert-function gke \
  --file ~/.zshrc \
  --function gke \
  --description "Switch to a GKE cluster environment" \
  --tags gcloud,kubernetes
```

## Examples

### Example Workflows in Clix Repository

The Clix repository includes several example workflows that demonstrate the conditional features:

1. **GKE Environment Switch** - `examples/conditional_workflows/gke.json`
   - Demonstrates authentication checking and branching based on environment
   - Similar to the `gke()` shell function shown earlier

2. **Port Process Killer** - `examples/conditional_workflows/killport.json`
   - Checks if processes exist on a port before trying to kill them
   - Demonstrates conditional steps with error handling

3. **GKE Service Connection** - `examples/conditional_workflows/gkeconnect.json`
   - More complex example with multiple branches and conditionals
   - Demonstrates service-specific configuration and port forwarding

4. **Shell Function Conversion** - `examples/shell_function_conversion/`
   - Contains original shell functions and their Clix workflow equivalents
   - Shows how to convert a complex function like `kseal` into a workflow

### Authentication with Conditional

```json
{
  "name": "deploy",
  "description": "Deploy application to GKE",
  "steps": [
    {
      "name": "Check Authentication",
      "type": "conditional",
      "condition": "! gcloud auth application-default print-access-token > /dev/null 2>&1",
      "then": [
        {
          "name": "Login to GCloud",
          "type": "auth",
          "command": "gcloud auth login"
        }
      ]
    },
    {
      "name": "Deploy Application",
      "type": "command",
      "command": "kubectl apply -f deployment.yaml"
    }
  ]
}
```

### Port Killing Workflow

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
      "type": "command",
      "command": "lsof -ti tcp:{{ port }}"
    },
    {
      "name": "Check Process Exists",
      "type": "conditional",
      "condition": "$? -eq 0",
      "then": [
        {
          "name": "Kill Process",
          "type": "command",
          "command": "kill -9 $(lsof -ti tcp:{{ port }})"
        },
        {
          "name": "Confirm",
          "type": "command",
          "command": "echo \"Process on port {{ port }} killed successfully\""
        }
      ],
      "else": [
        {
          "name": "No Process",
          "type": "command",
          "command": "echo \"No process found running on port {{ port }}\""
        }
      ]
    }
  ]
}
```

## Advanced Techniques

### Variable Capture

You can capture command output into variables:

```json
{
  "name": "Capture Output",
  "type": "conditional",
  "condition": "true",
  "variable": "output",
  "then": [
    {
      "name": "Use Output",
      "type": "command",
      "command": "echo \"Captured: $output\""
    }
  ]
}
```

### Early Return

You can return early from a workflow with a specific exit code:

```json
{
  "name": "Check Requirements",
  "type": "conditional",
  "condition": "! command -v kubectl > /dev/null",
  "action": "return",
  "exit_code": 1,
  "then": [
    {
      "name": "Error Message",
      "type": "command",
      "command": "echo \"kubectl is required but not installed\""
    }
  ]
}
```

## Best Practices

1. **Keep conditions simple**: Use simple expressions for better readability
2. **Provide descriptive names**: Name your steps and conditions clearly
3. **Test thoroughly**: Test conditional workflows with different inputs
4. **Use variables**: Leverage variables for flexible workflows
5. **Add error handling**: Use continue_on_error for steps that can fail safely