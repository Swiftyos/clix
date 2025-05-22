pub mod ai;
pub mod cli;
pub mod commands;
pub mod error;
pub mod git;
pub mod security;
pub mod settings;
pub mod share;
pub mod storage;

// Re-export for convenience
pub use ai::ClaudeAssistant;
pub use commands::{Command, Workflow, WorkflowStep};
pub use error::{ClixError, Result};
pub use git::{GitRepository, GitRepositoryManager, RepoConfig};
pub use settings::{Settings, SettingsManager};
pub use share::{ExportManager, ImportManager};
pub use storage::{Storage, GitIntegratedStorage};
