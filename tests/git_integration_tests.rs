use clix::git::{GitRepositoryManager, RepoConfig};
use clix::storage::GitIntegratedStorage;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_git_repository_manager_creation() {
    let mut manager = GitRepositoryManager::new().expect("Should create git manager");
    
    // Test that we can load configs without error
    manager.load_configs().expect("Should load configs successfully");
    
    // Test that initially no repositories are configured
    assert_eq!(manager.list_repositories().len(), 0);
}


#[test]
fn test_repo_config_serialization() {
    let config = RepoConfig {
        name: "test-repo".to_string(),
        url: "https://github.com/example/repo.git".to_string(),
        enabled: true,
    };
    
    let json = serde_json::to_string(&config).expect("Should serialize config");
    let deserialized: RepoConfig = serde_json::from_str(&json).expect("Should deserialize config");
    
    assert_eq!(config.name, deserialized.name);
    assert_eq!(config.url, deserialized.url);
    assert_eq!(config.enabled, deserialized.enabled);
}

#[test]
fn test_config_file_operations() {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let config_path = temp_dir.path().join("config.json");
    
    let configs = vec![
        RepoConfig {
            name: "repo1".to_string(),
            url: "https://github.com/example/repo1.git".to_string(),
            enabled: true,
        },
        RepoConfig {
            name: "repo2".to_string(),
            url: "https://github.com/example/repo2.git".to_string(),
            enabled: false,
        },
    ];
    
    // Test writing configs
    let content = serde_json::to_string_pretty(&configs).expect("Should serialize configs");
    fs::write(&config_path, content).expect("Should write config file");
    
    // Test reading configs
    let read_content = fs::read_to_string(&config_path).expect("Should read config file");
    let read_configs: Vec<RepoConfig> = serde_json::from_str(&read_content).expect("Should deserialize configs");
    
    assert_eq!(configs.len(), read_configs.len());
    assert_eq!(configs[0].name, read_configs[0].name);
    assert_eq!(configs[1].enabled, read_configs[1].enabled);
}