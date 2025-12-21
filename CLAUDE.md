# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

htop_mini is a lightweight, terminal-based system monitoring tool written in Rust. It displays real-time CPU usage (per-core), memory statistics, task/thread counts, load averages, and uptime using a TUI powered by ratatui and crossterm. Currently fully functional on macOS; Linux support is in progress.

## Build and Run Commands

```bash
# Build the project
cargo build

# Run the application
cargo run

# Run tests
cargo test

# Run a specific test
cargo test test_name

# Run tests with output visible
cargo test -- --nocapture

# Build for release
cargo build --release
```

## Architecture

The codebase follows a layered architecture with clear separation between data collection, computation, and presentation:

### Core Modules
- **model.rs**: Defines the data model with two key types:
  - `RawSample`: Raw system metrics collected from the kernel (CPU ticks, boot time, load average, memory stats, task stats)
  - `Snapshot`: Computed metrics derived from two `RawSample`s (CPU usage percentages, uptime). The `Snapshot::compute()` function calculates deltas between samples to determine usage percentages, handling u32 wraparound correctly.

- **sampler/**: Platform-specific system metric collection using a trait-based design:
  - `Sampler` trait: Defines the interface for collecting a `RawSample`
  - `PlatformSampler`: Type alias that resolves to `MacOsSampler` or `LinuxSampler` based on target OS
  - `KernelInterface` trait: Abstracts kernel system calls for testing
  - **macos.rs**: Implements macOS metric collection via Mach kernel APIs (host_processor_info, host_statistics64), sysctl, and proc_pidinfo.
  - **linux.rs**: Stub implementation (not yet functional)

- **ui.rs**: Terminal UI rendering with ratatui. Renders CPU bars (color-coded user/system), memory bars, swap, task stats, load average, and uptime.

- **main.rs**: Event loop that samples every 1 second, handles keyboard input (q/Esc to quit), computes snapshots from consecutive samples, and triggers UI redraws.

### Data Flow
1. `PlatformSampler::sample()` calls platform-specific kernel interfaces to collect `RawSample`
2. Main loop stores current sample and compares with previous sample
3. `Snapshot::compute()` calculates CPU usage percentages from tick deltas
4. `ui::render()` displays formatted metrics in the terminal

### Testing
- Extensive unit tests in model.rs verify CPU usage calculations
- Property-based tests using proptest ensure percentage calculations are always valid (0-100%, sum to 100%)
- Tests cover edge cases: u32 wraparound, zero deltas, multiple CPUs

## Important Notes
- This project uses Cargo edition 2024
- Avoid OBVIOUS comments, docstrings, and explanations when writing code
- When asked to build out a feature, always first write a detailed plan of how you plan to implement it to a file named <feature_name>.plan.md in the .claude directory


