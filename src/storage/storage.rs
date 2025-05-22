use crate::commands::models::{Command, CommandStore, Workflow};
use crate::error::{ClixError, Result};
use dirs::home_dir;
use std::fs;
use std::path::PathBuf;

pub struct Storage {
    store_path: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let store_dir = home_dir()
            .ok_or_else(|| ClixError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine home directory",
            )))?
            .join(".clix");

        fs::create_dir_all(&store_dir)?;

        let store_path = store_dir.join("commands.json");

        Ok(Storage { store_path })
    }

    pub fn load(&self) -> Result<CommandStore> {
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
        Ok(())
    }

    pub fn add_command(&self, command: Command) -> Result<()> {
        let mut store = self.load()?;
        store.commands.insert(command.name.clone(), command);
        self.save(&store)
    }

    pub fn get_command(&self, name: &str) -> Result<Command> {
        let store = self.load()?;
        store
            .commands
            .get(name)
            .cloned()
            .ok_or_else(|| ClixError::CommandNotFound(name.to_string()))
    }

    pub fn list_commands(&self) -> Result<Vec<Command>> {
        let store = self.load()?;
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
        let store = self.load()?;
        store
            .workflows
            .get(name)
            .cloned()
            .ok_or_else(|| ClixError::CommandNotFound(name.to_string()))
    }

    pub fn list_workflows(&self) -> Result<Vec<Workflow>> {
        let store = self.load()?;
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
}