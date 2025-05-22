# Conditional Workflow Design

## Overview

The goal is to extend the current workflow system to support conditionals, allowing users to create more complex workflows that can make decisions based on the output of previous steps or user input.

## Requirements

Based on the example shell functions, we need to support:

1. **Conditional Execution** - Execute steps only if certain conditions are met
2. **Branching Logic** - Choose different execution paths based on conditions
3. **Exit Conditions** - Exit a workflow or continue based on step results
4. **Parameter/Argument Handling** - Support workflow parameters like functions
5. **Return Values** - Allow workflows to return values or exit codes
6. **Input Validation** - Validate input parameters and provide usage messages
7. **Variable Scoping** - Support local variables within conditional blocks

## Design Proposal

### 1. Extended WorkflowStep Model

Add a new `ConditionalStep` type to `StepType` enum and create supporting structures:

```rust
pub enum StepType {
    Command,
    Auth,
    Conditional,
    Branch,
    Loop,
}

pub struct Condition {
    pub expression: String,    // Expression to evaluate
    pub variable: Option<String>,  // Optional variable to capture from output
}

pub enum ConditionalAction {
    RunThen,     // Run then block
    RunElse,     // Run else block
    Continue,    // Skip conditional and continue
    Break,       // Exit workflow
    Return(i32), // Return with exit code
}

pub struct ConditionalBlock {
    pub steps: Vec<WorkflowStep>,  // Steps to execute in this block
}
```

### 2. Expression Evaluation

Create an expression evaluator that can:

1. Parse and evaluate expressions like `$? -eq 0` (exit code equals 0)
2. Support comparison operators: `-eq`, `-ne`, `-gt`, `-lt`, `-ge`, `-le`
3. Support file tests: `-f`, `-d`, `-e`, etc.
4. Support string operations: `-z` (empty), `-n` (not empty), `==`, `!=`
5. Support logical operators: `&&`, `||`, `!`
6. Support capturing command output with `$(command)`
7. Support accessing variables with `$variable_name`

### 3. Conditional Step Structure

```rust
pub struct ConditionalStep {
    pub condition: Condition,
    pub then_block: ConditionalBlock,
    pub else_block: Option<ConditionalBlock>,
}
```

### 4. JSON Schema for Workflow Conditionals

```json
{
  "name": "gke",
  "description": "Switch to a GKE cluster environment",
  "parameters": [
    {
      "name": "env",
      "description": "Environment to switch to (dev or prod)",
      "required": true
    }
  ],
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
        },
        {
          "name": "Setup Application Default Credentials",
          "type": "auth",
          "command": "gcloud auth application-default login"
        }
      ]
    },
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
          },
          {
            "name": "Set Dev Namespace",
            "type": "command",
            "command": "kubectl config set-context --current --namespace=dev-namespace"
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
          },
          {
            "name": "Set Prod Namespace",
            "type": "command",
            "command": "kubectl config set-context --current --namespace=prod-namespace"
          }
        ],
        "default": [
          {
            "name": "Show Usage",
            "type": "command",
            "command": "echo \"Usage: gke [dev|prod]\""
          },
          {
            "name": "Exit",
            "type": "conditional",
            "condition": "true",
            "action": "return",
            "exit_code": 1
          }
        ]
      }
    }
  ]
}
```

### 5. Workflow Executor Changes

The workflow executor needs to be extended to:

1. Evaluate conditions
2. Execute the appropriate conditional block
3. Handle branching based on variables
4. Support exit codes and return values
5. Maintain a stack for nested conditionals

### 6. CLI Command Updates

Update CLI commands to:

1. Add support for defining conditional workflows
2. Provide validation for conditional workflow syntax
3. Support importing shell functions with automatic conversion
4. Include completion information as workflow metadata

## Implementation Strategy

1. **Phase 1**: Add basic conditional support (if/then/else)
2. **Phase 2**: Add branching logic (case statements)
3. **Phase 3**: Add loop support (for, while)
4. **Phase 4**: Add function parameter support
5. **Phase 5**: Add completion information

## Compatibility

The design should maintain backward compatibility with existing workflows while adding the new capabilities. Existing workflows will continue to work without modifications, while new workflows can take advantage of the conditional features.