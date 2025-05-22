pub mod cli;
pub mod commands;
pub mod error;
pub mod share;
pub mod storage;

// Re-export for convenience
pub use commands::{Command, Workflow, WorkflowStep};
pub use error::{ClixError, Result};
pub use share::{ExportManager, ImportManager};
pub use storage::Storage;
