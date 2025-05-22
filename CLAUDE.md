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
# Install cargo-nextest if not already installed
cargo install cargo-nextest

# Run all tests using nextest
cargo nextest run

# Run specific test
cargo nextest run test_name

# Run tests with higher verbosity
cargo nextest run -v

# Run tests in a specific file
cargo nextest run --package clix --lib commands::tests

# Run tests matching a pattern
cargo nextest run 'command_*'

# Run tests with a specific profile
cargo nextest run -P ci

# Legacy test command (fallback)
cargo test
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

## CI/CD

This project uses GitHub Actions for continuous integration. The workflow includes:

- Running tests with nextest
- Linting with clippy
- Checking formatting with rustfmt
- Building on multiple platforms (Linux, macOS, Windows)

The CI configuration is in `.github/workflows/ci.yml`.

## Architecture

The project consists of these main components:

- **Command Storage**: Persistence layer for storing commands and workflows
- **Command Parser**: Processing user input and mapping to stored commands
- **Workflow Engine**: Handling multi-step command sequences
- **AI Integration**: AI capabilities for command enhancement and suggestions
- **CLI Interface**: User interface for adding, listing, and executing commands

Current project structure:
- `Cargo.toml`: Project configuration and dependencies
- `src/lib.rs`: Core functionality as a library for testability
- `src/main.rs`: CLI entry point for the application
- `src/commands/`: Command models and execution
- `src/storage/`: Persistence of commands and workflows
- `src/share/`: Import/export functionality for sharing commands
- `src/cli/`: Command-line interface and argument parsing
- `src/error.rs`: Error types and handling
- `tests/`: Integration tests
- `.github/workflows/`: CI/CD configuration

Future additions:
- `src/ai/`: AI integration for enhancing commands