# Approval Workflows

Clix supports requiring explicit user approval for critical or potentially destructive workflow steps. This document explains how to use approval requirements in your workflows.

## Overview

When executing workflows that contain potentially destructive commands (like deleting resources, changing production environments, or deploying applications), it's often important to get explicit confirmation from the user before proceeding. The approval workflow feature allows you to mark specific steps as requiring approval, ensuring that users understand and confirm the actions they're about to take.

Benefits of using approval requirements:
- Prevents accidental execution of destructive operations
- Creates a human checkpoint for critical actions
- Ensures users are aware of what they're executing
- Documents which steps are considered sensitive
- Provides an opportunity to review before execution

## Adding Approval Requirements

There are several ways to mark a step as requiring approval:

### 1. In JSON Workflow Definitions

When defining a workflow in JSON, add the `require_approval` field to any step that needs confirmation:

```json
{
  "name": "Delete Old Backups",
  "description": "Remove backup files older than 30 days",
  "step_type": "Command",
  "command": "find /backups -type f -mtime +30 -delete",
  "continue_on_error": false,
  "require_approval": true
}
```

### 2. Using the CLI

When adding a step to a workflow via the CLI, you can use the `--require-approval` flag:

```bash
# Add a step that requires approval
clix flow add-step my-workflow \
  --name "Delete Old Backups" \
  --description "Remove backup files older than 30 days" \
  --command "find /backups -type f -mtime +30 -delete" \
  --require-approval
```

### 3. Automatic Approval for Dangerous Operations

When converting shell functions to workflows, you can use the `--approve-dangerous` flag to automatically add approval requirements for potentially destructive operations:

```bash
clix flow convert-function \
  --file ~/.zshrc \
  --function cleanup \
  --name cleanup-workflow \
  --description "Clean up temporary files and resources" \
  --approve-dangerous
```

This will analyze the function and automatically add approval requirements to steps that:
- Delete files or directories (rm, rmdir)
- Delete resources (kubectl delete, aws s3 rm)
- Modify production environments (detected via "prod" in command)
- Other potentially destructive operations

### 4. Using Constructor Methods (For Developers)

When programmatically creating workflow steps, you can use the dedicated constructor method:

```rust
let step = WorkflowStep::new_command_with_approval(
    "Delete Old Backups".to_string(),
    "find /backups -type f -mtime +30 -delete".to_string(),
    "Remove backup files older than 30 days".to_string(),
    false,
);
```

### 5. Using the Builder Pattern (For Developers)

Alternatively, you can use the `with_approval` method to add approval requirements:

```rust
let step = WorkflowStep::new_command(
    "Delete Old Backups".to_string(),
    "find /backups -type f -mtime +30 -delete".to_string(),
    "Remove backup files older than 30 days".to_string(),
    false,
).with_approval();
```

## How Approval Works

When Clix encounters a step that requires approval:

1. It displays detailed information about the step:
   - The step name
   - The step description
   - The command to be executed (if applicable)

2. It prompts the user with a yes/no question:
   ```
   ⚠️  This step requires approval before execution:
   Name: Delete Old Backups
   Description: Remove backup files older than 30 days
   Command: find /backups -type f -mtime +30 -delete
   Do you want to proceed? [y/N]:
   ```

3. The user must type `y` or `yes` (case-insensitive) to proceed. Any other input (or just pressing Enter) will cancel the step execution.

4. If the step is approved, execution continues normally.

5. If the step is rejected, the workflow stops with an error message: "Step execution canceled by user".

## Working with Approval in Different Step Types

Approval requirements can be added to any step type:

### Command Steps

```json
{
  "name": "Delete Resources",
  "description": "Delete all resources in the test namespace",
  "step_type": "Command",
  "command": "kubectl delete namespace test",
  "continue_on_error": false,
  "require_approval": true
}
```

### Auth Steps

```json
{
  "name": "Login to Production",
  "description": "Authenticate to production environment",
  "step_type": "Auth",
  "command": "aws sso login --profile production",
  "continue_on_error": false,
  "require_approval": true
}
```

### Conditional Steps

Approval can be added to the conditional step itself:

```json
{
  "name": "Check Production Deploy",
  "description": "Confirm deployment to production",
  "step_type": "Conditional",
  "command": "[ \"$env\" = \"prod\" ]",
  "continue_on_error": false,
  "require_approval": true,
  "conditional": {
    "then_steps": [
      {
        "name": "Deploy to Production",
        "description": "Deploy app to production servers",
        "step_type": "Command",
        "command": "kubectl apply -f deployment.yaml",
        "continue_on_error": false
      }
    ]
  }
}
```

Or to specific steps within the then/else blocks:

```json
{
  "name": "Check Environment",
  "description": "Select deployment environment",
  "step_type": "Conditional",
  "command": "[ \"$env\" = \"prod\" ]",
  "continue_on_error": false,
  "conditional": {
    "then_steps": [
      {
        "name": "Deploy to Production",
        "description": "Deploy app to production servers",
        "step_type": "Command",
        "command": "kubectl apply -f deployment.yaml",
        "continue_on_error": false,
        "require_approval": true
      }
    ],
    "else_steps": [
      {
        "name": "Deploy to Development",
        "description": "Deploy app to development servers",
        "step_type": "Command",
        "command": "kubectl apply -f dev-deployment.yaml",
        "continue_on_error": false
      }
    ]
  }
}
```

### Branch Steps

Approval can be added to individual cases within a branch step:

```json
{
  "name": "Select Environment",
  "description": "Configure the deployment environment",
  "step_type": "Branch",
  "branch": {
    "variable": "env",
    "cases": {
      "dev": [
        {
          "name": "Deploy to Dev",
          "description": "Deploy to development environment",
          "step_type": "Command",
          "command": "kubectl apply -f dev-deployment.yaml",
          "continue_on_error": false
        }
      ],
      "prod": [
        {
          "name": "Deploy to Production",
          "description": "Deploy to production environment",
          "step_type": "Command",
          "command": "kubectl apply -f prod-deployment.yaml",
          "continue_on_error": false,
          "require_approval": true
        }
      ]
    }
  }
}
```

### Loop Steps

Approval can be added to specific steps within a loop:

```json
{
  "name": "Clean Up Services",
  "description": "Clean up multiple services",
  "step_type": "Loop",
  "loop_data": {
    "variable": "service",
    "values": "auth api frontend database",
    "steps": [
      {
        "name": "Delete Service",
        "description": "Delete {{ service }} service resources",
        "step_type": "Command",
        "command": "kubectl delete -f {{ service }}.yaml",
        "continue_on_error": false,
        "require_approval": true
      }
    ]
  }
}
```

## When to Use Approval

Consider requiring approval for steps that:

1. **Delete data**: Commands that remove files, databases, or other data
   - `rm -rf /path/to/data`
   - `DROP TABLE users`
   - `kubectl delete namespace`

2. **Modify production environments**: Changes to production systems or configurations
   - `kubectl apply -f prod-deployment.yaml`
   - `aws cloudformation update-stack --stack-name prod-stack`
   - `terraform apply -var environment=prod`

3. **Deploy applications**: Releasing new versions to production
   - `aws s3 sync ./build s3://production-website`
   - `kubectl set image deployment/app app=app:v2`
   - `gcloud app deploy --project=prod-app`

4. **Create expensive resources**: Creating cloud resources that may incur significant costs
   - `aws ec2 run-instances --instance-type m5.4xlarge`
   - `gcloud compute instances create --machine-type=n1-standard-16`
   - `terraform apply -var "instance_count=20"`

5. **Modify security settings**: Changes to security groups, firewall rules, etc.
   - `aws ec2 authorize-security-group-ingress`
   - `gcloud compute firewall-rules update`
   - `kubectl apply -f network-policy.yaml`

6. **Scale resources**: Significantly scaling up or down resources
   - `kubectl scale deployment/app --replicas=50`
   - `aws autoscaling update-auto-scaling-group --max-size 100`

## Examples

### Example 1: Production Deployment Workflow

```json
{
  "name": "deploy-to-production",
  "description": "Deploy application to production environment",
  "steps": [
    {
      "name": "Run Tests",
      "description": "Run all tests before deployment",
      "step_type": "Command",
      "command": "npm test",
      "continue_on_error": false
    },
    {
      "name": "Build Application",
      "description": "Build the production version",
      "step_type": "Command",
      "command": "npm run build",
      "continue_on_error": false
    },
    {
      "name": "Deploy to Production",
      "description": "Deploy to production servers",
      "step_type": "Command",
      "command": "aws s3 sync ./build s3://production-website",
      "continue_on_error": false,
      "require_approval": true
    },
    {
      "name": "Invalidate Cache",
      "description": "Invalidate CDN cache",
      "step_type": "Command",
      "command": "aws cloudfront create-invalidation --distribution-id ABCDEF12345 --paths '/*'",
      "continue_on_error": false,
      "require_approval": true
    }
  ]
}
```

### Example 2: Database Cleanup Workflow

```json
{
  "name": "database-cleanup",
  "description": "Clean up old records from the database",
  "steps": [
    {
      "name": "Create Backup",
      "description": "Backup the database before cleaning",
      "step_type": "Command",
      "command": "pg_dump -Fc mydb > /backups/mydb-$(date +%Y%m%d).dump",
      "continue_on_error": false
    },
    {
      "name": "Verify Backup",
      "description": "Verify backup was created successfully",
      "step_type": "Conditional",
      "command": "[ -f /backups/mydb-$(date +%Y%m%d).dump ]",
      "continue_on_error": false,
      "conditional": {
        "then_steps": [],
        "else_steps": [
          {
            "name": "Backup Failed",
            "description": "Display error and exit",
            "step_type": "Command",
            "command": "echo \"Backup failed, aborting cleanup\"",
            "continue_on_error": false
          }
        ],
        "action": "return",
        "exit_code": 1
      }
    },
    {
      "name": "Delete Old Records",
      "description": "Remove records older than 1 year",
      "step_type": "Command",
      "command": "psql -c \"DELETE FROM logs WHERE created_at < NOW() - INTERVAL '1 year'\"",
      "continue_on_error": false,
      "require_approval": true
    },
    {
      "name": "Vacuum Database",
      "description": "Reclaim storage space",
      "step_type": "Command",
      "command": "psql -c \"VACUUM FULL\"",
      "continue_on_error": false,
      "require_approval": true
    }
  ]
}
```

### Example 3: Multi-Environment Deployment with Approval for Production Only

```json
{
  "name": "deploy-app",
  "description": "Deploy application to specified environment",
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
      "name": "Select Environment",
      "description": "Configure deployment based on environment",
      "step_type": "Branch",
      "branch": {
        "variable": "env",
        "cases": {
          "dev": [
            {
              "name": "Deploy to Dev",
              "description": "Deploy to development environment",
              "step_type": "Command",
              "command": "kubectl set image deployment/app-dev app=app:{{ version }} -n dev",
              "continue_on_error": false
            }
          ],
          "staging": [
            {
              "name": "Deploy to Staging",
              "description": "Deploy to staging environment",
              "step_type": "Command",
              "command": "kubectl set image deployment/app-staging app=app:{{ version }} -n staging",
              "continue_on_error": false,
              "require_approval": false
            }
          ],
          "prod": [
            {
              "name": "Deploy to Production",
              "description": "Deploy to production environment",
              "step_type": "Command",
              "command": "kubectl set image deployment/app-prod app=app:{{ version }} -n prod",
              "continue_on_error": false,
              "require_approval": true
            }
          ]
        },
        "default_case": [
          {
            "name": "Unknown Environment",
            "description": "Show error for unknown environment",
            "step_type": "Command",
            "command": "echo \"Unknown environment: {{ env }}\"\necho \"Valid environments: dev, staging, prod\"",
            "continue_on_error": false
          }
        ]
      }
    },
    {
      "name": "Verify Deployment",
      "description": "Verify deployment was successful",
      "step_type": "Command",
      "command": "kubectl rollout status deployment/app-{{ env }} -n {{ env }}",
      "continue_on_error": false
    }
  ]
}
```

## Advanced Techniques

### Conditional Approval

You can combine conditional steps with approval requirements to create workflows that require approval only in certain situations:

```json
{
  "name": "Conditional Cleanup",
  "description": "Conditionally approve cleanup",
  "steps": [
    {
      "name": "Check File Size",
      "description": "Check if log directory is large",
      "step_type": "Conditional",
      "command": "[ $(du -sm /var/log | cut -f1) -gt 1000 ]",
      "continue_on_error": false,
      "conditional": {
        "then_steps": [
          {
            "name": "Large Cleanup",
            "description": "Clean up logs (large, requires approval)",
            "step_type": "Command",
            "command": "find /var/log -type f -name \"*.log\" -mtime +30 -delete",
            "continue_on_error": false,
            "require_approval": true
          }
        ],
        "else_steps": [
          {
            "name": "Small Cleanup",
            "description": "Clean up logs (small, no approval needed)",
            "step_type": "Command",
            "command": "find /var/log -type f -name \"*.log\" -mtime +90 -delete",
            "continue_on_error": false
          }
        ]
      }
    }
  ]
}
```

### Environment-Based Approval

You can use branch steps to require approval only in certain environments:

```json
{
  "name": "Environment Deployment",
  "description": "Deploy with environment-specific approval",
  "variables": [
    {
      "name": "env",
      "description": "Deployment environment",
      "required": true
    }
  ],
  "steps": [
    {
      "name": "Deploy Application",
      "description": "Deploy application with environment-specific approval",
      "step_type": "Branch",
      "branch": {
        "variable": "env",
        "cases": {
          "dev": [
            {
              "name": "Deploy to Dev",
              "description": "Deploy to development (no approval)",
              "step_type": "Command",
              "command": "kubectl apply -f deployment.yaml -n dev",
              "continue_on_error": false
            }
          ],
          "staging": [
            {
              "name": "Deploy to Staging",
              "description": "Deploy to staging (requires approval)",
              "step_type": "Command",
              "command": "kubectl apply -f deployment.yaml -n staging",
              "continue_on_error": false,
              "require_approval": true
            }
          ],
          "prod": [
            {
              "name": "Deploy to Production",
              "description": "Deploy to production (requires approval)",
              "step_type": "Command",
              "command": "kubectl apply -f deployment.yaml -n prod",
              "continue_on_error": false,
              "require_approval": true
            }
          ]
        }
      }
    }
  ]
}
```

### Approval with Variables

Use variables to make approval steps more informative:

```json
{
  "name": "Delete Database",
  "description": "Delete the specified database",
  "variables": [
    {
      "name": "db_name",
      "description": "Database name to delete",
      "required": true
    }
  ],
  "steps": [
    {
      "name": "Delete Database",
      "description": "Permanently delete database {{ db_name }}",
      "step_type": "Command",
      "command": "psql -c \"DROP DATABASE {{ db_name }}\"",
      "continue_on_error": false,
      "require_approval": true
    }
  ]
}
```

## Best Practices

1. **Be selective**: Only require approval for truly destructive or critical operations to avoid approval fatigue.
   - Good: Requiring approval for `DROP DATABASE production`
   - Bad: Requiring approval for `ls -la`

2. **Provide clear descriptions**: Make sure your step descriptions clearly explain the purpose and impact of the step.
   - Good: "Delete all user data older than 1 year (cannot be undone)"
   - Bad: "Run cleanup script"

3. **Use variables**: Use workflow variables to make approval steps more informative.
   - Good: "Delete database {{ database_name }}"
   - Bad: "Delete database"

4. **Group related changes**: If multiple steps are related and all need approval, consider grouping them logically.
   - Consider requiring approval once at the beginning of a sequence rather than for each individual step

5. **Add safeguards**: Even with approval, include safeguards like checking if files exist before deleting them.
   - Add conditional checks before destructive operations
   - Create backups before dangerous operations

6. **Consider environments**: You might want to require approval only in production environments but not in development.
   - Use branch steps to apply approval selectively based on environment

7. **Be descriptive in prompts**: Ensure the command and description provide enough context for the user to make an informed decision.
   - Include what will be affected, what will happen, and whether the action can be undone

8. **Document approvals**: Include in your workflow documentation which steps require approval and why.
   - This helps users understand the workflow's safety measures

9. **Use with conditional checks**: Combine approval with conditional checks for maximum safety.
   - Check preconditions before asking for approval
   - Verify resources exist before attempting destructive actions

10. **Test your workflows**: Make sure approval steps work as expected by testing them in a safe environment first.
    - Verify that rejection properly cancels the workflow
    - Ensure all destructive steps have appropriate approval requirements