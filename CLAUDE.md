# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

htop_mini is a lightweight system monitoring tool written in Rust that collects and displays system metrics (CPU usage, memory, process information) on macOS and Linux. It's in early development with basic CPU sampling implemented for macOS.

## Build and Run Commands

```bash
# Build the project
cargo build

# Run the application (samples every 2 seconds)
cargo run

# Run tests
cargo test

# Run a specific test
cargo test test_name

# Build for release
cargo build --release
```

## Important Notes
- This project uses Cargo edition 2024
- Avoid OBVIOUS comments when writing code
- When writing a plan for a feature, write it to the /tmp directory in a file named <feature_name>.plan


