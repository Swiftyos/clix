use crate::commands::models::{Command, CommandStore, Workflow};
use crate::error::Result;
use crate::git::GitRepositoryManager;
use crate::settings::SettingsManager;
use crate::storage::Storage;
use std::fs;
use std::path::Path;

pub struct GitIntegratedStorage {
    local_storage: Storage,
    git_manager: GitRepositoryManager,
}

impl GitIntegratedStorage {
    pub fn new() -> Result<Self> {
        let local_storage = Storage::new()?;
        let mut git_manager = GitRepositoryManager::new()?;
        git_manager.load_configs()?;

        Ok(Self {
            local_storage,
            git_manager,
        })
    }

    pub fn get_git_manager(&mut self) -> &mut GitRepositoryManager {
        &mut self.git_manager
    }

    pub fn get_local_storage(&self) -> &Storage {
        &self.local_storage
    }

    pub fn sync_with_repositories(&self) -> Result<()> {
        // Pull from all repositories first
        let pull_results = self.git_manager.pull_all_repositories()?;

        for (repo_name, result) in pull_results {
            match result {
                Ok(()) => println!("✓ Synced repository: {}", repo_name),
                Err(e) => eprintln!("✗ Failed to sync repository {}: {}", repo_name, e),
            }
        }

        // Load commands and workflows from all repositories
        self.load_from_repositories()?;

        Ok(())
    }

    pub fn load_from_repositories(&self) -> Result<()> {
        let repo_paths = self.git_manager.get_all_repo_paths();
        let mut local_store = self.local_storage.load()?;

        for repo_path in repo_paths {
            self.load_from_repository(&repo_path, &mut local_store)?;
        }

        self.local_storage.save(&local_store)?;
        Ok(())
    }

    fn load_from_repository(&self, repo_path: &Path, local_store: &mut CommandStore) -> Result<()> {
        // Look for commands.json in the repository
        let commands_file = repo_path.join("commands.json");
        if commands_file.exists() {
            let content = fs::read_to_string(&commands_file)?;
            let repo_store: CommandStore = serde_json::from_str(&content)?;

            // Merge commands and workflows with local storage
            self.merge_commands(&repo_store.commands, local_store)?;
            self.merge_workflows(&repo_store.workflows, local_store)?;
        }

        Ok(())
    }

    fn merge_commands(
        &self,
        repo_commands: &std::collections::HashMap<String, Command>,
        local_store: &mut CommandStore,
    ) -> Result<()> {
        for (name, command) in repo_commands {
            if let Some(local_command) = local_store.commands.get(name) {
                // Compare timestamps to determine if the repo command is newer
                if command.created_at > local_command.created_at {
                    local_store.commands.insert(name.clone(), command.clone());
                }
            } else {
                // Command does not exist locally, so insert it
                local_store.commands.insert(name.clone(), command.clone());
            }
        }
        Ok(())
    }

    fn merge_workflows(
        &self,
        repo_workflows: &std::collections::HashMap<String, Workflow>,
        local_store: &mut CommandStore,
    ) -> Result<()> {
        for (name, workflow) in repo_workflows {
            if let Some(local_workflow) = local_store.workflows.get(name) {
                // Compare timestamps to determine if the repo workflow is newer
                if workflow.created_at > local_workflow.created_at {
                    local_store.workflows.insert(name.clone(), workflow.clone());
                }
            } else {
                // Workflow does not exist locally, so insert it
                local_store.workflows.insert(name.clone(), workflow.clone());
            }
        }
        Ok(())
    }

    pub fn commit_changes_to_repositories(&self, message: &str) -> Result<()> {
        let settings_manager = SettingsManager::new()?;
        let settings = settings_manager.load()?;
        let prefixed_message = format!(
            "{} {}",
            settings.git_settings.commit_message_prefix, message
        );

        let repo_paths = self.git_manager.get_all_repo_paths();

        for repo_path in repo_paths {
            self.commit_to_repository(&repo_path, &prefixed_message)?;
        }

        Ok(())
    }

    fn commit_to_repository(&self, repo_path: &Path, message: &str) -> Result<()> {
        // Export current commands to the repository
        let commands_file = repo_path.join("commands.json");
        let store = self.local_storage.load()?;
        let content = serde_json::to_string_pretty(&store)?;
        fs::write(&commands_file, content)?;

        // Find the repository config and commit
        if let Some(repo_name) = repo_path.file_name().and_then(|n| n.to_str()) {
            if let Some(repo) = self.git_manager.get_repository(repo_name) {
                repo.commit_and_push(message, &["commands.json"])?;
            }
        }

        Ok(())
    }

    // Delegate methods to local storage
    pub fn add_command(&self, command: Command) -> Result<()> {
        let result = self.local_storage.add_command(command);

        // If successful, try to commit to repositories
        if result.is_ok() {
            if let Err(e) = self.commit_changes_to_repositories("Add new command via clix") {
                eprintln!("Warning: Failed to sync to git repositories: {}", e);
            }
        }

        result
    }

    pub fn get_command(&self, name: &str) -> Result<Command> {
        self.local_storage.get_command(name)
    }

    pub fn list_commands(&self) -> Result<Vec<Command>> {
        self.local_storage.list_commands()
    }

    pub fn remove_command(&self, name: &str) -> Result<()> {
        let result = self.local_storage.remove_command(name);

        // If successful, try to commit to repositories
        if result.is_ok() {
            if let Err(e) =
                self.commit_changes_to_repositories(&format!("Remove command: {}", name))
            {
                eprintln!("Warning: Failed to sync to git repositories: {}", e);
            }
        }

        result
    }

    pub fn update_command_usage(&self, name: &str) -> Result<()> {
        self.local_storage.update_command_usage(name)
    }

    pub fn update_command(&self, command: &Command) -> Result<()> {
        let result = self.local_storage.update_command(command);

        // If successful, try to commit to repositories
        if result.is_ok() {
            if let Err(e) =
                self.commit_changes_to_repositories(&format!("Update command: {}", command.name))
            {
                eprintln!("Warning: Failed to sync to git repositories: {}", e);
            }
        }

        result
    }

    pub fn add_workflow(&self, workflow: Workflow) -> Result<()> {
        let result = self.local_storage.add_workflow(workflow);

        // If successful, try to commit to repositories
        if result.is_ok() {
            if let Err(e) = self.commit_changes_to_repositories("Add new workflow via clix") {
                eprintln!("Warning: Failed to sync to git repositories: {}", e);
            }
        }

        result
    }

    pub fn get_workflow(&self, name: &str) -> Result<Workflow> {
        self.local_storage.get_workflow(name)
    }

    pub fn list_workflows(&self) -> Result<Vec<Workflow>> {
        self.local_storage.list_workflows()
    }

    pub fn remove_workflow(&self, name: &str) -> Result<()> {
        let result = self.local_storage.remove_workflow(name);

        // If successful, try to commit to repositories
        if result.is_ok() {
            if let Err(e) =
                self.commit_changes_to_repositories(&format!("Remove workflow: {}", name))
            {
                eprintln!("Warning: Failed to sync to git repositories: {}", e);
            }
        }

        result
    }

    pub fn update_workflow_usage(&self, name: &str) -> Result<()> {
        self.local_storage.update_workflow_usage(name)
    }

    pub fn update_workflow(&self, workflow: &Workflow) -> Result<()> {
        let result = self.local_storage.update_workflow(workflow);

        // If successful, try to commit to repositories
        if result.is_ok() {
            if let Err(e) =
                self.commit_changes_to_repositories(&format!("Update workflow: {}", workflow.name))
            {
                eprintln!("Warning: Failed to sync to git repositories: {}", e);
            }
        }

        result
    }
}
