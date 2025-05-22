# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Clix is a command-line tool for developers to store and execute commands and workflows. It allows users to:
- Save frequently used commands with descriptions for easy recall
- Create and run complex workflows (e.g., troubleshooting production issues)
- Leverage AI to enhance command management and execution
- List saved commands with explanations using `clix list`

Use cases include:
- Retrieving logs from cloud services
- Restarting services during incidents
- Running complex development or deployment sequences
- Managing common commands similar to shell aliases but with better organization

## Commands

### Building and Running

```bash
# Build the project
cargo build

# Run the project
cargo run

# Build with optimizations for release
cargo build --release

# Install locally for testing
cargo install --path .
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### Development Tools

```bash
# Check code for errors without building
cargo check

# Format code
cargo fmt

# Lint the codebase
cargo clippy

# Update dependencies
cargo update
```

## Architecture

The project will likely consist of these main components:

- **Command Storage**: Persistence layer for storing commands and workflows
- **Command Parser**: Processing user input and mapping to stored commands
- **Workflow Engine**: Handling multi-step command sequences
- **AI Integration**: AI capabilities for command enhancement and suggestions
- **CLI Interface**: User interface for adding, listing, and executing commands

Current project structure:
- `Cargo.toml`: Project configuration and dependencies
- `src/main.rs`: Entry point for the application

Suggested module organization:
- `src/commands/`: Command parsing and execution
- `src/storage/`: Persistence of commands and workflows
- `src/workflows/`: Complex workflow definitions and execution
- `src/ai/`: AI integration for enhancing commands
- `src/cli/`: Command-line interface components