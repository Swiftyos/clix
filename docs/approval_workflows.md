# Approval Workflows

Clix supports requiring explicit user approval for critical or potentially destructive workflow steps. This document explains how to use approval requirements in your workflows.

## Overview

When executing workflows that contain potentially destructive commands (like deleting resources, changing production environments, or deploying applications), it's often important to get explicit confirmation from the user before proceeding. The approval workflow feature allows you to mark specific steps as requiring approval, ensuring that users understand and confirm the actions they're about to take.

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

### 2. Using Constructor Methods

When programmatically creating workflow steps, you can use the dedicated constructor method:

```rust
let step = WorkflowStep::new_command_with_approval(
    "Delete Old Backups".to_string(),
    "find /backups -type f -mtime +30 -delete".to_string(),
    "Remove backup files older than 30 days".to_string(),
    false,
);
```

### 3. Using the Builder Pattern

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

## When to Use Approval

Consider requiring approval for steps that:

1. **Delete data**: Commands that remove files, databases, or other data
2. **Modify production environments**: Changes to production systems or configurations
3. **Deploy applications**: Releasing new versions to production
4. **Create expensive resources**: Creating cloud resources that may incur significant costs
5. **Modify security settings**: Changes to security groups, firewall rules, etc.

## Examples

### Production Deployment Workflow

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

### Database Cleanup Workflow

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

## Best Practices

1. **Be selective**: Only require approval for truly destructive or critical operations to avoid approval fatigue.

2. **Provide clear descriptions**: Make sure your step descriptions clearly explain the purpose and impact of the step.

3. **Use variables**: Use workflow variables to make approval steps more informative: `"Delete database {{ database_name }}"`.

4. **Group related changes**: If multiple steps are related and all need approval, consider grouping them logically.

5. **Add safeguards**: Even with approval, include safeguards like checking if files exist before deleting them.

6. **Consider environments**: You might want to require approval only in production environments but not in development.