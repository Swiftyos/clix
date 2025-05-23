use crate::error::{ClixError, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    #[serde(default = "default_ai_model")]
    pub ai_model: String,

    #[serde(default)]
    pub ai_settings: AiSettings,

    #[serde(default)]
    pub git_settings: GitSettings,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiSettings {
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitSettings {
    #[serde(default = "default_auto_sync")]
    pub auto_sync: bool,

    #[serde(default = "default_auto_commit")]
    pub auto_commit: bool,

    #[serde(default = "default_commit_message_prefix")]
    pub commit_message_prefix: String,
}

fn default_ai_model() -> String {
    "claude-3-opus-20240229".to_string()
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> usize {
    4000
}

fn default_auto_sync() -> bool {
    true
}

fn default_auto_commit() -> bool {
    true
}

fn default_commit_message_prefix() -> String {
    "clix:".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            ai_model: default_ai_model(),
            ai_settings: AiSettings::default(),
            git_settings: GitSettings::default(),
        }
    }
}

impl Default for AiSettings {
    fn default() -> Self {
        AiSettings {
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

impl Default for GitSettings {
    fn default() -> Self {
        GitSettings {
            auto_sync: default_auto_sync(),
            auto_commit: default_auto_commit(),
            commit_message_prefix: default_commit_message_prefix(),
        }
    }
}

pub struct SettingsManager {
    settings_path: PathBuf,
}

impl SettingsManager {
    pub fn new() -> Result<Self> {
        let settings_dir = home_dir()
            .ok_or_else(|| {
                ClixError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not determine home directory",
                ))
            })?
            .join(".clix");

        fs::create_dir_all(&settings_dir)?;

        let settings_path = settings_dir.join("settings.json");

        Ok(SettingsManager { settings_path })
    }

    /// Create settings manager with custom directory for testing
    pub fn new_with_dir(settings_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&settings_dir)?;
        let settings_path = settings_dir.join("settings.json");

        Ok(SettingsManager { settings_path })
    }

    pub fn load(&self) -> Result<Settings> {
        if !self.settings_path.exists() {
            return Ok(Settings::default());
        }

        let content = fs::read_to_string(&self.settings_path)?;
        let settings: Settings = serde_json::from_str(&content)?;
        Ok(settings)
    }

    pub fn save(&self, settings: &Settings) -> Result<()> {
        let content = serde_json::to_string_pretty(settings)?;
        fs::write(&self.settings_path, content)?;
        Ok(())
    }

    pub fn update_ai_model(&self, model: &str) -> Result<()> {
        let mut settings = self.load()?;
        settings.ai_model = model.to_string();
        self.save(&settings)
    }

    pub fn update_ai_temperature(&self, temperature: f32) -> Result<()> {
        // Validate temperature range (0.0 to 1.0)
        if !(0.0..=1.0).contains(&temperature) {
            return Err(ClixError::InvalidInput(format!(
                "Temperature must be between 0.0 and 1.0, got: {}",
                temperature
            )));
        }

        let mut settings = self.load()?;
        settings.ai_settings.temperature = temperature;
        self.save(&settings)
    }

    pub fn update_ai_max_tokens(&self, max_tokens: usize) -> Result<()> {
        let mut settings = self.load()?;
        settings.ai_settings.max_tokens = max_tokens;
        self.save(&settings)
    }
}
