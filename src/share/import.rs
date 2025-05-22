use crate::error::{ClixError, Result};
use crate::share::export::ExportData;
use crate::storage::Storage;
use std::fs;

pub struct ImportManager {
    storage: Storage,
}

impl ImportManager {
    pub fn new(storage: Storage) -> Self {
        ImportManager { storage }
    }

    pub fn import_from_file(&self, input_path: &str, overwrite: bool) -> Result<ImportSummary> {
        // Read the file
        let file_content = fs::read_to_string(input_path).map_err(ClixError::Io)?;

        // Parse the JSON
        let export_data: ExportData =
            serde_json::from_str(&file_content).map_err(ClixError::Serialization)?;

        // Load the current store
        let mut store = self.storage.load()?;

        // Initialize counters
        let mut summary = ImportSummary {
            commands_added: 0,
            commands_updated: 0,
            commands_skipped: 0,
            workflows_added: 0,
            workflows_updated: 0,
            workflows_skipped: 0,
            metadata: export_data.metadata,
        };

        // Import commands
        if let Some(commands) = export_data.commands {
            for (name, command) in commands {
                if store.commands.contains_key(&name) {
                    if overwrite {
                        store.commands.insert(name.clone(), command);
                        summary.commands_updated += 1;
                    } else {
                        summary.commands_skipped += 1;
                    }
                } else {
                    store.commands.insert(name, command);
                    summary.commands_added += 1;
                }
            }
        }

        // Import workflows
        if let Some(workflows) = export_data.workflows {
            for (name, workflow) in workflows {
                if store.workflows.contains_key(&name) {
                    if overwrite {
                        store.workflows.insert(name.clone(), workflow);
                        summary.workflows_updated += 1;
                    } else {
                        summary.workflows_skipped += 1;
                    }
                } else {
                    store.workflows.insert(name, workflow);
                    summary.workflows_added += 1;
                }
            }
        }

        // Save the updated store
        self.storage.save(&store)?;

        Ok(summary)
    }
}

pub struct ImportSummary {
    pub commands_added: usize,
    pub commands_updated: usize,
    pub commands_skipped: usize,
    pub workflows_added: usize,
    pub workflows_updated: usize,
    pub workflows_skipped: usize,
    pub metadata: crate::share::export::ExportMetadata,
}
