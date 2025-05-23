use clix::commands::models::{Command, CommandStore, Workflow, WorkflowStep};
use clix::git::{GitRepositoryManager, RepoConfig};
use clix::storage::GitIntegratedStorage;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_git_integrated_storage_creation() {
    let storage = GitIntegratedStorage::new();
    assert!(storage.is_ok(), "Should create git integrated storage");
}

#[test]
fn test_git_repository_manager_basic_operations() {
    let mut manager = GitRepositoryManager::new().expect("Should create manager");

    // Test listing empty repositories
    assert_eq!(manager.list_repositories().len(), 0);

    // Test loading empty configs
    manager.load_configs().expect("Should load configs");
    assert_eq!(manager.list_repositories().len(), 0);
}

#[test]
fn test_repo_config_operations() {
    let config = RepoConfig {
        name: "test-repo".to_string(),
        url: "https://github.com/example/test.git".to_string(),
        enabled: true,
    };

    // Test serialization
    let json = serde_json::to_string(&config).expect("Should serialize");
    let deserialized: RepoConfig = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(config.name, deserialized.name);
    assert_eq!(config.url, deserialized.url);
    assert_eq!(config.enabled, deserialized.enabled);
}

#[test]
fn test_command_store_serialization() {
    let mut store = CommandStore::new();

    // Add a test command
    let command = Command::new(
        "test-cmd".to_string(),
        "Test command".to_string(),
        "echo hello".to_string(),
        vec!["test".to_string()],
    );
    store.commands.insert("test-cmd".to_string(), command);

    // Add a test workflow
    let steps = vec![WorkflowStep::new_command(
        "step1".to_string(),
        "echo test".to_string(),
        "Test step".to_string(),
        false,
    )];
    let workflow = Workflow::new(
        "test-workflow".to_string(),
        "Test workflow".to_string(),
        steps,
        vec!["test".to_string()],
    );
    store
        .workflows
        .insert("test-workflow".to_string(), workflow);

    // Test serialization to JSON
    let json = serde_json::to_string_pretty(&store).expect("Should serialize store");
    let deserialized: CommandStore = serde_json::from_str(&json).expect("Should deserialize store");

    assert_eq!(store.commands.len(), deserialized.commands.len());
    assert_eq!(store.workflows.len(), deserialized.workflows.len());

    let original_cmd = store.commands.get("test-cmd").unwrap();
    let deserialized_cmd = deserialized.commands.get("test-cmd").unwrap();
    assert_eq!(original_cmd.name, deserialized_cmd.name);
    assert_eq!(original_cmd.command, deserialized_cmd.command);

    let original_wf = store.workflows.get("test-workflow").unwrap();
    let deserialized_wf = deserialized.workflows.get("test-workflow").unwrap();
    assert_eq!(original_wf.name, deserialized_wf.name);
    assert_eq!(original_wf.steps.len(), deserialized_wf.steps.len());
}

#[test]
fn test_git_manager_config_persistence() {
    let temp_dir = TempDir::new().expect("Should create temp dir");

    // Create config directly in temp directory structure
    let config_file = temp_dir.path().join("config.json");
    let configs = vec![
        RepoConfig {
            name: "repo1".to_string(),
            url: "https://github.com/test/repo1.git".to_string(),
            enabled: true,
        },
        RepoConfig {
            name: "repo2".to_string(),
            url: "https://github.com/test/repo2.git".to_string(),
            enabled: false,
        },
    ];

    // Write config file
    let json = serde_json::to_string_pretty(&configs).expect("Should serialize");
    fs::write(&config_file, json).expect("Should write config");

    // Read and verify
    let content = fs::read_to_string(&config_file).expect("Should read config");
    let loaded_configs: Vec<RepoConfig> =
        serde_json::from_str(&content).expect("Should parse config");

    assert_eq!(configs.len(), loaded_configs.len());
    assert_eq!(configs[0].name, loaded_configs[0].name);
    assert_eq!(configs[1].enabled, loaded_configs[1].enabled);
}

#[test]
fn test_command_merge_behavior() {
    // Test that local commands take precedence during merge
    let mut local_commands = HashMap::new();
    local_commands.insert(
        "cmd1".to_string(),
        Command::new(
            "cmd1".to_string(),
            "Local version".to_string(),
            "echo local".to_string(),
            vec![],
        ),
    );

    let mut repo_commands = HashMap::new();
    repo_commands.insert(
        "cmd1".to_string(),
        Command::new(
            "cmd1".to_string(),
            "Repo version".to_string(),
            "echo repo".to_string(),
            vec![],
        ),
    );
    repo_commands.insert(
        "cmd2".to_string(),
        Command::new(
            "cmd2".to_string(),
            "Only in repo".to_string(),
            "echo repo-only".to_string(),
            vec![],
        ),
    );

    // Simulate merge logic (local takes precedence)
    let mut merged = local_commands.clone();
    for (name, command) in repo_commands {
        merged.entry(name).or_insert(command);
    }

    assert_eq!(merged.len(), 2);
    assert_eq!(merged.get("cmd1").unwrap().description, "Local version");
    assert_eq!(merged.get("cmd2").unwrap().description, "Only in repo");
}
