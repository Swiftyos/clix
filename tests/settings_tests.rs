use clix::settings::SettingsManager;
use std::env;
use std::fs;
use test_context::{AsyncTestContext, test_context};
use std::path::PathBuf;

struct SettingsContext {
    temp_dir: PathBuf,
    settings_manager: SettingsManager,
}

impl AsyncTestContext for SettingsContext {
    fn setup<'a>() -> std::pin::Pin<Box<dyn std::future::Future<Output = Self> + Send + 'a>> {
        Box::pin(async {
            // Create a unique temporary directory for tests
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros();

            let temp_dir = std::env::temp_dir()
                .join("clix_test")
                .join(format!("test_{}", timestamp));
            fs::create_dir_all(&temp_dir).unwrap();

            // Ensure .clix directory exists
            let clix_dir = temp_dir.join(".clix");
            fs::create_dir_all(&clix_dir).unwrap();

            // Temporarily set HOME environment variable to our test directory
            unsafe {
                env::set_var("HOME", &temp_dir);
            }

            // Create the settings manager instance that will use our test directory
            let settings_manager = SettingsManager::new().unwrap();

            SettingsContext { temp_dir, settings_manager }
        })
    }

    fn teardown<'a>(self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            // Clean up the temporary directory
            fs::remove_dir_all(&self.temp_dir).unwrap_or_default();
        })
    }
}

#[test_context(SettingsContext)]
#[tokio::test]
async fn test_default_settings(ctx: &mut SettingsContext) {
    // Load the default settings
    let settings = ctx.settings_manager.load().unwrap();
    
    // Check default values
    assert_eq!(settings.ai_model, "claude-3-opus-20240229");
    assert_eq!(settings.ai_settings.temperature, 0.7);
    assert_eq!(settings.ai_settings.max_tokens, 4000);
}

#[test_context(SettingsContext)]
#[tokio::test]
async fn test_update_ai_model(ctx: &mut SettingsContext) {
    // Update AI model
    let new_model = "claude-3-sonnet-20240229";
    ctx.settings_manager.update_ai_model(new_model).unwrap();
    
    // Verify the update
    let settings = ctx.settings_manager.load().unwrap();
    assert_eq!(settings.ai_model, new_model);
}

#[test_context(SettingsContext)]
#[tokio::test]
async fn test_update_ai_temperature(ctx: &mut SettingsContext) {
    // Update AI temperature
    let new_temperature = 0.5;
    ctx.settings_manager.update_ai_temperature(new_temperature).unwrap();
    
    // Verify the update
    let settings = ctx.settings_manager.load().unwrap();
    assert_eq!(settings.ai_settings.temperature, new_temperature);
}

#[test_context(SettingsContext)]
#[tokio::test]
async fn test_update_ai_max_tokens(ctx: &mut SettingsContext) {
    // Update AI max tokens
    let new_max_tokens = 2000;
    ctx.settings_manager.update_ai_max_tokens(new_max_tokens).unwrap();
    
    // Verify the update
    let settings = ctx.settings_manager.load().unwrap();
    assert_eq!(settings.ai_settings.max_tokens, new_max_tokens);
}

#[test_context(SettingsContext)]
#[tokio::test]
async fn test_persistence(ctx: &mut SettingsContext) {
    // Update all settings
    let new_model = "claude-3-haiku-20240307";
    let new_temperature = 0.3;
    let new_max_tokens = 1000;
    
    ctx.settings_manager.update_ai_model(new_model).unwrap();
    ctx.settings_manager.update_ai_temperature(new_temperature).unwrap();
    ctx.settings_manager.update_ai_max_tokens(new_max_tokens).unwrap();
    
    // Create a new settings manager to verify persistence
    let new_settings_manager = SettingsManager::new().unwrap();
    let settings = new_settings_manager.load().unwrap();
    
    // Verify all settings were persisted
    assert_eq!(settings.ai_model, new_model);
    assert_eq!(settings.ai_settings.temperature, new_temperature);
    assert_eq!(settings.ai_settings.max_tokens, new_max_tokens);
}