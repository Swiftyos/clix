use crate::ai::conversation::{ConversationSession, ConversationStore};
use crate::error::{ClixError, Result};
use dirs::home_dir;
use std::fs;
use std::path::PathBuf;

pub struct ConversationStorage {
    store_path: PathBuf,
}

impl ConversationStorage {
    pub fn new() -> Result<Self> {
        let store_dir = home_dir()
            .ok_or_else(|| {
                ClixError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not determine home directory",
                ))
            })?
            .join(".clix");

        fs::create_dir_all(&store_dir)?;

        let store_path = store_dir.join("conversations.json");

        Ok(ConversationStorage { store_path })
    }

    pub fn load(&self) -> Result<ConversationStore> {
        if !self.store_path.exists() {
            return Ok(ConversationStore::new());
        }

        let content = fs::read_to_string(&self.store_path)?;
        let store: ConversationStore = serde_json::from_str(&content)?;
        Ok(store)
    }

    pub fn save(&self, store: &ConversationStore) -> Result<()> {
        let content = serde_json::to_string_pretty(store)?;
        fs::write(&self.store_path, content)?;
        Ok(())
    }

    pub fn save_session(&self, session: &ConversationSession) -> Result<()> {
        let mut store = self.load()?;
        store.add_session(session.clone());
        self.save(&store)
    }

    pub fn get_session(&self, id: &str) -> Result<Option<ConversationSession>> {
        let store = self.load()?;
        Ok(store.get_session(id).cloned())
    }

    pub fn update_session(&self, session: &ConversationSession) -> Result<()> {
        let mut store = self.load()?;
        if store.sessions.contains_key(&session.id) {
            store.sessions.insert(session.id.clone(), session.clone());
            self.save(&store)?;
        } else {
            return Err(ClixError::NotFound(format!(
                "Conversation session '{}' not found",
                session.id
            )));
        }
        Ok(())
    }

    pub fn list_active_sessions(&self) -> Result<Vec<ConversationSession>> {
        let store = self.load()?;
        Ok(store.list_active_sessions().into_iter().cloned().collect())
    }

    pub fn cleanup_expired_sessions(&self, max_age_hours: u64) -> Result<usize> {
        let mut store = self.load()?;
        let initial_count = store.sessions.len();
        store.cleanup_expired_sessions(max_age_hours);
        let final_count = store.sessions.len();
        self.save(&store)?;
        Ok(initial_count - final_count)
    }

    pub fn remove_session(&self, id: &str) -> Result<bool> {
        let mut store = self.load()?;
        let removed = store.remove_session(id).is_some();
        if removed {
            self.save(&store)?;
        }
        Ok(removed)
    }
}
