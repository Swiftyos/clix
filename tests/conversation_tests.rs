use clix::ai::{ConversationSession, ConversationState, MessageRole};
use clix::storage::ConversationStorage;
use std::fs;
use temp_dir::TempDir;

#[test]
fn test_conversation_session_creation() {
    let session = ConversationSession::new();
    assert!(!session.id.is_empty());
    assert!(matches!(session.state, ConversationState::Active));
    assert!(session.messages.is_empty());
}

#[test]
fn test_conversation_session_messages() {
    let mut session = ConversationSession::new();
    
    let message_id = session.add_message(MessageRole::User, "Hello Claude".to_string());
    assert!(!message_id.is_empty());
    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].content, "Hello Claude");
    assert!(matches!(session.messages[0].role, MessageRole::User));
}

#[test]
fn test_conversation_storage() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory for testing
    let temp_dir = TempDir::new()?;
    let conversations_path = temp_dir.path().join("conversations.json");
    
    // Override home directory for testing
    std::env::set_var("HOME", temp_dir.path());
    
    let storage = ConversationStorage::new()?;
    let mut session = ConversationSession::new();
    session.add_message(MessageRole::User, "Test message".to_string());
    
    // Save session
    storage.save_session(&session)?;
    
    // Load session
    let loaded_session = storage.get_session(&session.id)?;
    assert!(loaded_session.is_some());
    let loaded_session = loaded_session.unwrap();
    assert_eq!(loaded_session.id, session.id);
    assert_eq!(loaded_session.messages.len(), 1);
    
    Ok(())
}

#[test]
fn test_conversation_state_management() {
    let mut session = ConversationSession::new();
    
    // Test state transitions
    session.set_state(ConversationState::WaitingForConfirmation);
    assert!(matches!(session.state, ConversationState::WaitingForConfirmation));
    
    session.set_state(ConversationState::Completed);
    assert!(matches!(session.state, ConversationState::Completed));
}

#[test]
fn test_conversation_context_retrieval() {
    let mut session = ConversationSession::new();
    
    // Add multiple messages
    session.add_message(MessageRole::User, "Message 1".to_string());
    session.add_message(MessageRole::Assistant, "Response 1".to_string());
    session.add_message(MessageRole::User, "Message 2".to_string());
    session.add_message(MessageRole::Assistant, "Response 2".to_string());
    
    // Test context retrieval
    let recent_context = session.get_recent_context(2);
    assert_eq!(recent_context.len(), 2);
    assert_eq!(recent_context[0].content, "Message 2");
    assert_eq!(recent_context[1].content, "Response 2");
}