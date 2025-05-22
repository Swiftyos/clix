use crate::ai::claude::ClaudeAction;
use crate::commands::WorkflowStep;

// Mock implementation for tests to avoid needing real API calls
pub struct MockClaudeAssistant;

impl MockClaudeAssistant {
    pub fn mock_response(question: &str) -> (String, ClaudeAction) {
        match question {
            q if q.contains("create command") => {
                (
                    "[CREATE COMMAND]\nName: test-echo\nDescription: Echo a test message\nCommand: echo \"This is a test\"\n\nThis command will simply echo a test message to the console.".to_string(),
                    ClaudeAction::CreateCommand {
                        name: "test-echo".to_string(),
                        description: "Echo a test message".to_string(),
                        command: "echo \"This is a test\"".to_string(),
                    }
                )
            },
            q if q.contains("create workflow") => {
                (
                    "[CREATE WORKFLOW]\nName: test-workflow\nDescription: A test workflow\nSteps:\n- Step 1: name=\"Echo Step\", command=\"echo Step 1\", description=\"Echo the first step\", continue_on_error=false, step_type=\"Command\"\n- Step 2: name=\"Echo Step 2\", command=\"echo Step 2\", description=\"Echo the second step\", continue_on_error=true, step_type=\"Command\"\n\nThis workflow will run two echo commands in sequence.".to_string(),
                    ClaudeAction::CreateWorkflow {
                        name: "test-workflow".to_string(),
                        description: "A test workflow".to_string(),
                        steps: vec![
                            WorkflowStep::new_command(
                                "Echo Step".to_string(),
                                "echo Step 1".to_string(),
                                "Echo the first step".to_string(),
                                false,
                            ),
                            WorkflowStep::new_command(
                                "Echo Step 2".to_string(),
                                "echo Step 2".to_string(),
                                "Echo the second step".to_string(),
                                true,
                            ),
                        ],
                    }
                )
            },
            q if q.contains("run command") => {
                (
                    "[RUN COMMAND: list-files]\n\nThis command will list all files in the current directory.".to_string(),
                    ClaudeAction::RunCommand("list-files".to_string())
                )
            },
            q if q.contains("run workflow") => {
                (
                    "[RUN WORKFLOW: deploy-app]\n\nThis workflow will deploy your application to the production environment.".to_string(),
                    ClaudeAction::RunWorkflow("deploy-app".to_string())
                )
            },
            _ => (
                "[INFO]\nI don't have a specific action to suggest for that question. Here's some information about Clix instead...".to_string(),
                ClaudeAction::NoAction
            ),
        }
    }
}