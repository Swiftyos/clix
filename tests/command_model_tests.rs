use clix::commands::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn test_command_creation() {
    let name = "test-command".to_string();
    let description = "Test command description".to_string();
    let command_str = "echo 'Hello, World!'".to_string();
    let tags = vec!["test".to_string(), "example".to_string()];

    let command = Command::new(
        name.clone(),
        description.clone(),
        command_str.clone(),
        tags.clone(),
    );

    assert_eq!(command.name, name);
    assert_eq!(command.description, description);
    assert_eq!(command.command, command_str);
    assert_eq!(command.tags, tags);
    assert_eq!(command.use_count, 0);
    assert!(command.last_used.is_none());

    // Ensure created_at is reasonably close to now
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    assert!(command.created_at <= now);
    assert!(command.created_at >= now - 10); // Allow 10 seconds leeway
}

#[test]
fn test_command_usage_tracking() {
    let command = Command::new(
        "usage-test".to_string(),
        "Test usage tracking".to_string(),
        "echo 'Testing'".to_string(),
        vec!["test".to_string()],
    );

    assert_eq!(command.use_count, 0);
    assert!(command.last_used.is_none());

    let mut command = command;
    command.mark_used();

    assert_eq!(command.use_count, 1);
    assert!(command.last_used.is_some());

    let first_usage = command.last_used.unwrap();
    command.mark_used();

    assert_eq!(command.use_count, 2);
    assert!(command.last_used.unwrap() >= first_usage);
}
