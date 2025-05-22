# Clix Examples

This directory contains example workflows for the Clix command-line tool.

## Authentication Workflow Examples

### Google Cloud Resources Workflow (`gcloud-resources.json`)

This workflow demonstrates:
1. Authentication handling with `step_type: "Auth"`
2. Parameter substitution using `${PARAMETER_NAME}` syntax
3. Reading various resources from a Google Cloud project

The workflow:
1. Authenticates with Google Cloud
2. Sets the project using a parameter
3. Lists various GCP resources (compute instances, storage buckets, SQL instances, etc.)

#### Using the workflow

First, import the workflow:

```bash
clix import --input examples/gcloud-resources.json
```

Then run it with a specific project ID:

```bash
# This will prompt you to replace ${PROJECT_ID} with your actual project ID
clix run-workflow gcloud-resources
```

Or set environment variables before running:

```bash
export PROJECT_ID=my-actual-project-id
clix run-workflow gcloud-resources
```

### Google Cloud Deploy Workflow (`auth-workflow.json`)

This workflow demonstrates:
1. Authentication handling with Google Cloud
2. Deploying an application to Google App Engine

The workflow:
1. Authenticates with Google Cloud
2. Sets a fixed project ID
3. Deploys an application to Google App Engine

#### Using the workflow

```bash
clix import --input examples/auth-workflow.json
clix run-workflow gcloud-deploy
```

## Other Examples

### Shared Commands (`shared-commands.json`)

A collection of commonly used commands that can be shared with a team.

### General Workflow Example (`workflow.json`)

A basic example showing how to structure a workflow with multiple steps.