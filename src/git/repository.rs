use crate::error::{ClixError, Result};
use dirs::home_dir;
use git2::{BranchType, Repository};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub name: String,
    pub url: String,
    pub enabled: bool,
}

pub struct GitRepository {
    repo_path: PathBuf,
    config: RepoConfig,
}

impl GitRepository {
    pub fn new(config: RepoConfig, base_path: &Path) -> Self {
        let repo_path = base_path.join(&config.name);
        Self { repo_path, config }
    }

    pub fn clone(&self) -> Result<()> {
        if self.repo_path.exists() {
            return Err(ClixError::GitError(format!(
                "Repository directory '{}' already exists",
                self.repo_path.display()
            )));
        }

        fs::create_dir_all(self.repo_path.parent().unwrap_or(&self.repo_path))?;

        match Repository::clone(&self.config.url, &self.repo_path) {
            Ok(_) => Ok(()),
            Err(e) => Err(ClixError::GitError(format!(
                "Failed to clone repository '{}': {}",
                self.config.url, e
            ))),
        }
    }

    pub fn pull(&self) -> Result<()> {
        let repo = Repository::open(&self.repo_path).map_err(|e| {
            ClixError::GitError(format!(
                "Failed to open repository at '{}': {}",
                self.repo_path.display(),
                e
            ))
        })?;

        // Get the current branch
        let head = repo.head().map_err(|e| {
            ClixError::GitError(format!("Failed to get HEAD reference: {}", e))
        })?;

        let branch_name = head
            .shorthand()
            .ok_or_else(|| ClixError::GitError("Failed to get branch name".to_string()))?;

        // Fetch from origin
        let mut remote = repo.find_remote("origin").map_err(|e| {
            ClixError::GitError(format!("Failed to find remote 'origin': {}", e))
        })?;

        remote
            .fetch(&[branch_name], None, None)
            .map_err(|e| ClixError::GitError(format!("Failed to fetch from origin: {}", e)))?;

        // Get the updated reference
        let fetch_head = repo.find_reference("FETCH_HEAD").map_err(|e| {
            ClixError::GitError(format!("Failed to find FETCH_HEAD: {}", e))
        })?;

        let fetch_commit = repo.reference_to_annotated_commit(&fetch_head).map_err(|e| {
            ClixError::GitError(format!("Failed to get fetch commit: {}", e))
        })?;

        // Perform merge analysis
        let analysis = repo.merge_analysis(&[&fetch_commit]).map_err(|e| {
            ClixError::GitError(format!("Failed to analyze merge: {}", e))
        })?;

        if analysis.0.is_fast_forward() {
            // Fast-forward merge
            let refname = format!("refs/heads/{}", branch_name);
            let mut reference = repo.find_reference(&refname).map_err(|e| {
                ClixError::GitError(format!("Failed to find reference '{}': {}", refname, e))
            })?;

            reference
                .set_target(fetch_commit.id(), "Fast-forward")
                .map_err(|e| ClixError::GitError(format!("Failed to fast-forward: {}", e)))?;

            // Update working directory
            repo.set_head(&refname).map_err(|e| {
                ClixError::GitError(format!("Failed to set HEAD: {}", e))
            })?;

            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .map_err(|e| ClixError::GitError(format!("Failed to checkout HEAD: {}", e)))?;
        } else if analysis.0.is_up_to_date() {
            // Already up to date
        } else {
            return Err(ClixError::GitError(
                "Repository has diverged. Manual merge required".to_string(),
            ));
        }

        Ok(())
    }

    pub fn commit_and_push(&self, message: &str, files: &[&str]) -> Result<()> {
        let repo = Repository::open(&self.repo_path).map_err(|e| {
            ClixError::GitError(format!(
                "Failed to open repository at '{}': {}",
                self.repo_path.display(),
                e
            ))
        })?;

        // Create a new branch for this commit
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let branch_name = format!("clix-update-{}", timestamp);

        // Get the current HEAD commit
        let head = repo.head().map_err(|e| {
            ClixError::GitError(format!("Failed to get HEAD reference: {}", e))
        })?;
        let head_commit = head.peel_to_commit().map_err(|e| {
            ClixError::GitError(format!("Failed to get HEAD commit: {}", e))
        })?;

        // Create new branch
        repo.branch(&branch_name, &head_commit, false)
            .map_err(|e| ClixError::GitError(format!("Failed to create branch: {}", e)))?;

        // Switch to the new branch
        let branch_ref = format!("refs/heads/{}", branch_name);
        repo.set_head(&branch_ref)
            .map_err(|e| ClixError::GitError(format!("Failed to switch to branch: {}", e)))?;

        // Add files to index
        let mut index = repo.index().map_err(|e| {
            ClixError::GitError(format!("Failed to get repository index: {}", e))
        })?;

        for file in files {
            let file_path = Path::new(file);
            index.add_path(file_path).map_err(|e| {
                ClixError::GitError(format!("Failed to add file '{}' to index: {}", file, e))
            })?;
        }

        index.write().map_err(|e| {
            ClixError::GitError(format!("Failed to write index: {}", e))
        })?;

        // Create commit
        let tree_id = index.write_tree().map_err(|e| {
            ClixError::GitError(format!("Failed to write tree: {}", e))
        })?;
        let tree = repo.find_tree(tree_id).map_err(|e| {
            ClixError::GitError(format!("Failed to find tree: {}", e))
        })?;

        let signature = git2::Signature::now("Clix", "clix@example.com").map_err(|e| {
            ClixError::GitError(format!("Failed to create signature: {}", e))
        })?;

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&head_commit],
        )
        .map_err(|e| ClixError::GitError(format!("Failed to create commit: {}", e)))?;

        // Push the branch
        let mut remote = repo.find_remote("origin").map_err(|e| {
            ClixError::GitError(format!("Failed to find remote 'origin': {}", e))
        })?;

        let push_spec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
        remote
            .push(&[&push_spec], None)
            .map_err(|e| ClixError::GitError(format!("Failed to push branch: {}", e)))?;

        Ok(())
    }

    pub fn is_cloned(&self) -> bool {
        self.repo_path.exists() && Repository::open(&self.repo_path).is_ok()
    }

    pub fn get_repo_path(&self) -> &Path {
        &self.repo_path
    }

    pub fn get_config(&self) -> &RepoConfig {
        &self.config
    }
}

pub struct GitRepositoryManager {
    repos_dir: PathBuf,
    configs: Vec<RepoConfig>,
}

impl GitRepositoryManager {
    pub fn new() -> Result<Self> {
        let repos_dir = home_dir()
            .ok_or_else(|| {
                ClixError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not determine home directory",
                ))
            })?
            .join(".clix")
            .join("repos");

        fs::create_dir_all(&repos_dir)?;

        Ok(Self {
            repos_dir,
            configs: Vec::new(),
        })
    }

    pub fn add_repository(&mut self, name: String, url: String) -> Result<()> {
        if self.configs.iter().any(|c| c.name == name) {
            return Err(ClixError::InvalidCommandFormat(format!(
                "Repository '{}' already exists",
                name
            )));
        }

        let config = RepoConfig {
            name,
            url,
            enabled: true,
        };

        let repo = GitRepository::new(config.clone(), &self.repos_dir);
        repo.clone()?;

        self.configs.push(config);
        self.save_configs()?;

        Ok(())
    }

    pub fn remove_repository(&mut self, name: &str) -> Result<()> {
        let index = self
            .configs
            .iter()
            .position(|c| c.name == name)
            .ok_or_else(|| ClixError::CommandNotFound(format!("Repository '{}'", name)))?;

        let config = &self.configs[index];
        let repo_path = self.repos_dir.join(&config.name);

        if repo_path.exists() {
            fs::remove_dir_all(&repo_path)?;
        }

        self.configs.remove(index);
        self.save_configs()?;

        Ok(())
    }

    pub fn list_repositories(&self) -> &[RepoConfig] {
        &self.configs
    }

    pub fn get_repository(&self, name: &str) -> Option<GitRepository> {
        self.configs
            .iter()
            .find(|c| c.name == name)
            .map(|config| GitRepository::new(config.clone(), &self.repos_dir))
    }

    pub fn pull_all_repositories(&self) -> Result<Vec<(String, Result<()>)>> {
        let mut results = Vec::new();

        for config in &self.configs {
            if !config.enabled {
                continue;
            }

            let repo = GitRepository::new(config.clone(), &self.repos_dir);
            if repo.is_cloned() {
                let result = repo.pull();
                results.push((config.name.clone(), result));
            } else {
                results.push((
                    config.name.clone(),
                    Err(ClixError::GitError(format!(
                        "Repository '{}' is not cloned",
                        config.name
                    ))),
                ));
            }
        }

        Ok(results)
    }

    pub fn get_all_repo_paths(&self) -> Vec<PathBuf> {
        self.configs
            .iter()
            .filter(|c| c.enabled)
            .map(|config| {
                let repo = GitRepository::new(config.clone(), &self.repos_dir);
                if repo.is_cloned() {
                    Some(repo.get_repo_path().to_path_buf())
                } else {
                    None
                }
            })
            .filter_map(|path| path)
            .collect()
    }

    fn save_configs(&self) -> Result<()> {
        let config_path = self.repos_dir.join("config.json");
        let content = serde_json::to_string_pretty(&self.configs)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    pub fn load_configs(&mut self) -> Result<()> {
        let config_path = self.repos_dir.join("config.json");
        if !config_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(config_path)?;
        self.configs = serde_json::from_str(&content)?;
        Ok(())
    }
}