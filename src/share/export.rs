use crate::commands::models::{Command, Workflow, CommandStore};
use crate::error::{ClixError, Result};
use crate::storage::Storage;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportData {
    pub version: String,
    pub metadata: ExportMetadata,
    pub commands: Option<HashMap<String, Command>>,
    pub workflows: Option<HashMap<String, Workflow>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportMetadata {
    pub exported_at: u64,
    pub exported_by: String,
    pub description: String,
}

pub struct ExportManager {
    storage: Storage,
}

impl ExportManager {
    pub fn new(storage: Storage) -> Self {
        ExportManager { storage }
    }

    pub fn export_all(&self, output_path: &str) -> Result<()> {
        let store = self.storage.load()?;
        self.write_export_file(output_path, store, None, false, false)
    }

    pub fn export_with_filter(
        &self, 
        output_path: &str,
        tag_filter: Option<String>,
        commands_only: bool,
        workflows_only: bool,
    ) -> Result<()> {
        let store = self.storage.load()?;
        self.write_export_file(output_path, store, tag_filter, commands_only, workflows_only)
    }

    fn write_export_file(
        &self,
        output_path: &str,
        store: CommandStore,
        tag_filter: Option<String>,
        commands_only: bool,
        workflows_only: bool,
    ) -> Result<()> {
        // Filter commands if needed
        let commands = if !workflows_only {
            let mut filtered_commands = store.commands;
            
            if let Some(tag) = &tag_filter {
                filtered_commands = filtered_commands
                    .into_iter()
                    .filter(|(_, cmd)| cmd.tags.contains(tag))
                    .collect();
            }
            
            Some(filtered_commands)
        } else {
            None
        };

        // Filter workflows if needed
        let workflows = if !commands_only {
            let mut filtered_workflows = store.workflows;
            
            if let Some(tag) = &tag_filter {
                filtered_workflows = filtered_workflows
                    .into_iter()
                    .filter(|(_, wf)| wf.tags.contains(tag))
                    .collect();
            }
            
            Some(filtered_workflows)
        } else {
            None
        };

        // Create metadata
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let username = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        
        let metadata = ExportMetadata {
            exported_at: now,
            exported_by: username,
            description: format!(
                "Exported {} {}{}",
                if tag_filter.is_some() { "with tag filter" } else { "all" },
                if commands_only { "commands" } else if workflows_only { "workflows" } else { "commands and workflows" },
                if let Some(tag) = &tag_filter { format!(": {}", tag) } else { "".to_string() }
            ),
        };

        // Create export data
        let export_data = ExportData {
            version: env!("CARGO_PKG_VERSION").to_string(),
            metadata,
            commands,
            workflows,
        };

        // Serialize to JSON and write to file
        let json = serde_json::to_string_pretty(&export_data)
            .map_err(|e| ClixError::Serialization(e))?;

        fs::write(output_path, json)
            .map_err(|e| ClixError::Io(e))?;

        Ok(())
    }
}