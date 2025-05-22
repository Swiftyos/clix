use crate::commands::models::{Command, CommandStore, Workflow};
use crate::error::{ClixError, Result};
use dirs::home_dir;
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Clone)]
pub struct Storage {
    store_path: PathBuf,
    cache: RefCell<Option<CachedStore>>,
}

#[derive(Clone)]
struct CachedStore {
    store: CommandStore,
    last_modified: SystemTime,
    dirty: bool,
}

impl Storage {
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

        let store_path = store_dir.join("commands.json");

        Ok(Storage { 
            store_path,
            cache: RefCell::new(None),
        })
    }

    /// Load store with caching for improved performance
    pub fn load(&self) -> Result<CommandStore> {
        self.load_with_cache()
    }

    /// Load store from cache if valid, otherwise from disk
    fn load_with_cache(&self) -> Result<CommandStore> {
        // Check if file exists
        if !self.store_path.exists() {
            return Ok(CommandStore::new());
        }

        // Get file modification time
        let file_modified = fs::metadata(&self.store_path)?.modified()?;

        // Check cache validity
        let mut cache = self.cache.borrow_mut();
        if let Some(ref cached) = *cache {
            if cached.last_modified >= file_modified && !cached.dirty {
                return Ok(cached.store.clone());
            }
        }

        // Load from disk and update cache
        let content = fs::read_to_string(&self.store_path)?;
        let store: CommandStore = serde_json::from_str(&content)?;

        *cache = Some(CachedStore {
            store: store.clone(),
            last_modified: file_modified,
            dirty: false,
        });

        Ok(store)
    }

    /// Load store without caching (for when we need fresh data)
    fn load_direct(&self) -> Result<CommandStore> {
        if !self.store_path.exists() {
            return Ok(CommandStore::new());
        }

        let content = fs::read_to_string(&self.store_path)?;
        let store: CommandStore = serde_json::from_str(&content)?;
        Ok(store)
    }

    pub fn save(&self, store: &CommandStore) -> Result<()> {
        let content = serde_json::to_string_pretty(store)?;
        fs::write(&self.store_path, content)?;
        
        // Update cache with new data
        let file_modified = fs::metadata(&self.store_path)?.modified()?;
        let mut cache = self.cache.borrow_mut();
        *cache = Some(CachedStore {
            store: store.clone(),
            last_modified: file_modified,
            dirty: false,
        });
        
        Ok(())
    }

    /// Mark cache as dirty without saving (for bulk operations)
    fn mark_cache_dirty(&self) {
        let mut cache = self.cache.borrow_mut();
        if let Some(ref mut cached) = *cache {
            cached.dirty = true;
        }
    }

    /// Save and update cache atomically
    fn save_and_update_cache(&self, store: &CommandStore) -> Result<()> {
        self.save(store)
    }

    pub fn add_command(&self, command: Command) -> Result<()> {
        let mut store = self.load()?;
        store.commands.insert(command.name.clone(), command);
        self.save(&store)
    }

    pub fn get_command(&self, name: &str) -> Result<Command> {
        let store = self.load_with_cache()?;
        store
            .commands
            .get(name)
            .cloned()
            .ok_or_else(|| ClixError::CommandNotFound(name.to_string()))
    }

    /// Get command reference without cloning (more efficient for read-only operations)
    pub fn get_command_ref<F, R>(&self, name: &str, f: F) -> Result<R>
    where
        F: FnOnce(&Command) -> R,
    {
        let store = self.load_with_cache()?;
        store
            .commands
            .get(name)
            .map(f)
            .ok_or_else(|| ClixError::CommandNotFound(name.to_string()))
    }

    pub fn list_commands(&self) -> Result<Vec<Command>> {
        let store = self.load_with_cache()?;
        Ok(store.commands.values().cloned().collect())
    }

    pub fn remove_command(&self, name: &str) -> Result<()> {
        let mut store = self.load()?;
        if store.commands.remove(name).is_none() {
            return Err(ClixError::CommandNotFound(name.to_string()));
        }
        self.save(&store)
    }

    pub fn update_command_usage(&self, name: &str) -> Result<()> {
        let mut store = self.load()?;

        if let Some(cmd) = store.commands.get_mut(name) {
            cmd.mark_used();
            self.save(&store)?;
            Ok(())
        } else {
            Err(ClixError::CommandNotFound(name.to_string()))
        }
    }

    pub fn add_workflow(&self, workflow: Workflow) -> Result<()> {
        let mut store = self.load()?;
        store.workflows.insert(workflow.name.clone(), workflow);
        self.save(&store)
    }

    pub fn get_workflow(&self, name: &str) -> Result<Workflow> {
        let store = self.load_with_cache()?;
        store
            .workflows
            .get(name)
            .cloned()
            .ok_or_else(|| ClixError::CommandNotFound(name.to_string()))
    }

    /// Get workflow reference without cloning (more efficient for read-only operations)
    pub fn get_workflow_ref<F, R>(&self, name: &str, f: F) -> Result<R>
    where
        F: FnOnce(&Workflow) -> R,
    {
        let store = self.load_with_cache()?;
        store
            .workflows
            .get(name)
            .map(f)
            .ok_or_else(|| ClixError::CommandNotFound(name.to_string()))
    }

    pub fn list_workflows(&self) -> Result<Vec<Workflow>> {
        let store = self.load_with_cache()?;
        Ok(store.workflows.values().cloned().collect())
    }

    pub fn remove_workflow(&self, name: &str) -> Result<()> {
        let mut store = self.load()?;
        if store.workflows.remove(name).is_none() {
            return Err(ClixError::CommandNotFound(name.to_string()));
        }
        self.save(&store)
    }

    pub fn update_workflow_usage(&self, name: &str) -> Result<()> {
        let mut store = self.load()?;

        if let Some(wf) = store.workflows.get_mut(name) {
            wf.mark_used();
            self.save(&store)?;
            Ok(())
        } else {
            Err(ClixError::CommandNotFound(name.to_string()))
        }
    }

    pub fn update_workflow(&self, workflow: &Workflow) -> Result<()> {
        let mut store = self.load()?;

        if store.workflows.contains_key(&workflow.name) {
            store
                .workflows
                .insert(workflow.name.clone(), workflow.clone());
            self.save(&store)?;
            Ok(())
        } else {
            Err(ClixError::CommandNotFound(workflow.name.clone()))
        }
    }
}
