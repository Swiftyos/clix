use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClixError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Command execution failed: {0}")]
    CommandExecutionFailed(String),

    #[error("Invalid command format: {0}")]
    InvalidCommandFormat(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Header value error: {0}")]
    HeaderValueError(#[from] reqwest::header::InvalidHeaderValue),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Security error: {0}")]
    SecurityError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),

    #[error("Git error: {0}")]
    GitError(String),
}

impl ClixError {
    /// Convert error to user-friendly message with helpful suggestions
    pub fn to_user_friendly_message(&self) -> String {
        match self {
            ClixError::CommandNotFound(name) => {
                format!("Command '{}' not found.\n💡 Use 'clix list' to see available commands or 'clix add' to create a new one.", name)
            }
            ClixError::CommandExecutionFailed(msg) => {
                format!("Execution failed: {}\n💡 Check your command syntax and try again. Use 'clix list' to verify the command.", msg)
            }
            ClixError::InvalidCommandFormat(msg) => {
                format!("Invalid format: {}\n💡 Check the documentation or use 'clix --help' for correct syntax.", msg)
            }
            ClixError::Io(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        "File not found.\n💡 Check if the file path is correct and the file exists.".to_string()
                    }
                    std::io::ErrorKind::PermissionDenied => {
                        "Permission denied.\n💡 Check file permissions or run with appropriate privileges.".to_string()
                    }
                    std::io::ErrorKind::AlreadyExists => {
                        "File already exists.\n💡 Use --overwrite flag or choose a different name.".to_string()
                    }
                    _ => format!("File operation failed: {}\n💡 Check file permissions and disk space.", e)
                }
            }
            ClixError::Serialization(e) => {
                format!("Data format error: {}\n💡 Check if your JSON files are properly formatted. Use a JSON validator if needed.", e)
            }
            ClixError::ApiError(msg) => {
                format!("API error: {}\n💡 Check your internet connection and API key configuration.", msg)
            }
            ClixError::ValidationError(msg) => {
                format!("Validation failed: {}\n💡 Review your input and ensure all required fields are provided.", msg)
            }
            ClixError::SecurityError(msg) => {
                format!("Security check failed: {}\n⚠️  This command was blocked for security reasons.", msg)
            }
            ClixError::ConfigurationError(msg) => {
                format!("Configuration error: {}\n💡 Check your settings with 'clix settings list' or reset with default values.", msg)
            }
            ClixError::NetworkError(msg) => {
                format!("Network error: {}\n💡 Check your internet connection and try again.", msg)
            }
            ClixError::RateLimitError(msg) => {
                format!("Rate limit exceeded: {}\n💡 Wait a moment before trying again.", msg)
            }
            ClixError::HeaderValueError(e) => {
                format!("Header format error: {}\n💡 Check your API configuration.", e)
            }
            ClixError::GitError(msg) => {
                format!("Git operation failed: {}\n💡 Check repository access and git configuration.", msg)
            }
        }
    }

    /// Get suggested actions for the error
    pub fn get_suggestions(&self) -> Vec<String> {
        match self {
            ClixError::CommandNotFound(_) => vec![
                "Run 'clix list' to see all available commands".to_string(),
                "Use 'clix add' to create a new command".to_string(),
                "Check for typos in the command name".to_string(),
            ],
            ClixError::CommandExecutionFailed(_) => vec![
                "Verify command syntax is correct".to_string(),
                "Check if required tools are installed".to_string(),
                "Try running the command manually first".to_string(),
            ],
            ClixError::InvalidCommandFormat(_) => vec![
                "Check the command format with 'clix --help'".to_string(),
                "Refer to documentation for examples".to_string(),
                "Validate JSON files with a JSON validator".to_string(),
            ],
            ClixError::Io(_) => vec![
                "Check file path is correct".to_string(),
                "Verify file permissions".to_string(),
                "Ensure sufficient disk space".to_string(),
            ],
            ClixError::ApiError(_) => vec![
                "Check your internet connection".to_string(),
                "Verify API key is set correctly".to_string(),
                "Try again in a few moments".to_string(),
            ],
            ClixError::GitError(_) => vec![
                "Check if git is installed and configured".to_string(),
                "Verify repository URL and access permissions".to_string(),
                "Ensure SSH keys are set up correctly for private repos".to_string(),
            ],
            _ => vec!["Consult the documentation for more help".to_string()],
        }
    }

    /// Check if this error suggests retrying the operation
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ClixError::NetworkError(_) | ClixError::ApiError(_) | ClixError::RateLimitError(_)
        )
    }
}

impl From<ClixError> for String {
    fn from(error: ClixError) -> String {
        error.to_user_friendly_message()
    }
}

pub type Result<T> = std::result::Result<T, ClixError>;
