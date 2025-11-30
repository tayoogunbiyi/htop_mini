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

## Architecture

### Platform Abstraction Pattern

The codebase uses Rust's conditional compilation to provide platform-specific implementations:

- **src/sampler.rs**: Defines the `Sampler` trait and uses `#[cfg(target_os = "...")]` to export the appropriate platform implementation as `PlatformSampler`
- **src/sampler/macos.rs**: macOS implementation using Mach kernel FFI calls
- **src/sampler/linux.rs**: Linux implementation (currently a stub)

### Main Sampling Loop

The main loop (src/main.rs) creates a platform-specific sampler and calls `sample()` every 2 seconds (configurable via `SAMPLE_INTERVAL_SECS`).

### macOS Kernel Integration

The macOS sampler uses FFI to call Mach kernel APIs directly:
- `host_processor_info()` retrieves per-CPU load statistics
- Returns arrays of CPU state information (user, system, idle, nice)
- Memory must be manually deallocated using `vm_deallocate()` to prevent leaks

Current implementation prints CPU percentages but doesn't populate the `RawSample` struct yet.

### Planned Components

The following modules are placeholders for future development:
- **collector.rs**: Will handle data collection orchestration
- **store.rs**: Will store historical samples
- **ui.rs**: Will handle terminal UI display

## Important Notes

- This project uses Cargo edition 2024

## Coding style
- Avoid OBVIOUS comments.


