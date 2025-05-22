use crate::commands::{Command, Workflow, WorkflowStep};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSession {
    pub id: String,
    pub created_at: u64,
    pub last_activity: u64,
    pub messages: Vec<ConversationMessage>,
    pub context: ConversationContext,
    pub state: ConversationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: u64,
    pub metadata: MessageMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub action_suggested: Option<String>,
    pub action_executed: bool,
    pub tokens_used: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub available_commands: Vec<String>,
    pub available_workflows: Vec<String>,
    pub current_working_dir: Option<String>,
    pub last_command_result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversationState {
    Active,
    WaitingForConfirmation,
    CreatingWorkflow(WorkflowCreationState),
    RefiningWorkflow(String), // workflow name being refined
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCreationState {
    pub name: Option<String>,
    pub description: Option<String>,
    pub steps: Vec<WorkflowStep>,
    pub pending_refinements: Vec<String>,
}

impl ConversationSession {
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            last_activity: now,
            messages: Vec::new(),
            context: ConversationContext {
                available_commands: Vec::new(),
                available_workflows: Vec::new(),
                current_working_dir: None,
                last_command_result: None,
            },
            state: ConversationState::Active,
        }
    }

    pub fn with_context(commands: Vec<&Command>, workflows: Vec<&Workflow>) -> Self {
        let mut session = Self::new();
        session.context.available_commands = commands.iter().map(|c| c.name.clone()).collect();
        session.context.available_workflows = workflows.iter().map(|w| w.name.clone()).collect();
        session
    }

    pub fn add_message(&mut self, role: MessageRole, content: String) -> String {
        let message_id = Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let message = ConversationMessage {
            id: message_id.clone(),
            role,
            content,
            timestamp: now,
            metadata: MessageMetadata {
                action_suggested: None,
                action_executed: false,
                tokens_used: None,
            },
        };

        self.messages.push(message);
        self.last_activity = now;
        message_id
    }

    pub fn update_message_metadata(&mut self, message_id: &str, metadata: MessageMetadata) {
        if let Some(message) = self.messages.iter_mut().find(|m| m.id == message_id) {
            message.metadata = metadata;
        }
    }

    pub fn set_state(&mut self, state: ConversationState) {
        self.state = state;
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    pub fn is_expired(&self, max_age_hours: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        (now - self.last_activity) > (max_age_hours * 3600)
    }

    pub fn get_recent_context(&self, max_messages: usize) -> Vec<&ConversationMessage> {
        self.messages
            .iter()
            .rev()
            .take(max_messages)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
}

impl Default for ConversationSession {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationStore {
    pub sessions: HashMap<String, ConversationSession>,
}

impl ConversationStore {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn add_session(&mut self, session: ConversationSession) {
        self.sessions.insert(session.id.clone(), session);
    }

    pub fn get_session(&self, id: &str) -> Option<&ConversationSession> {
        self.sessions.get(id)
    }

    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut ConversationSession> {
        self.sessions.get_mut(id)
    }

    pub fn list_active_sessions(&self) -> Vec<&ConversationSession> {
        self.sessions
            .values()
            .filter(|s| {
                matches!(
                    s.state,
                    ConversationState::Active
                        | ConversationState::WaitingForConfirmation
                        | ConversationState::CreatingWorkflow(_)
                        | ConversationState::RefiningWorkflow(_)
                )
            })
            .collect()
    }

    pub fn cleanup_expired_sessions(&mut self, max_age_hours: u64) {
        self.sessions
            .retain(|_, session| !session.is_expired(max_age_hours));
    }

    pub fn remove_session(&mut self, id: &str) -> Option<ConversationSession> {
        self.sessions.remove(id)
    }
}

impl Default for ConversationStore {
    fn default() -> Self {
        Self::new()
    }
}
