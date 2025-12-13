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

The codebase follows a layered architecture:

### Core Modules
- **model.rs**: Defines the data model with two key types:
  - `RawSample`: Raw system metrics collected from the kernel (CPU ticks, boot time, load average, memory stats)
  - `Snapshot`: Computed metrics derived from two `RawSample`s (CPU usage percentages, uptime). The `Snapshot::compute()` function calculates deltas between samples to determine usage percentages, handling u32 wraparound correctly.

- **sampler/**: Platform-specific system metric collection using a trait-based design:
  - `Sampler` trait: Defines the interface for collecting a `RawSample`
  - `PlatformSampler`: Type alias that resolves to `MacOsSampler` or `LinuxSampler` based on target OS
  - `KernelInterface` trait: Abstracts kernel system calls for testing
  - **macos.rs**: Implements macOS metric collection via Mach kernel APIs (host_processor_info, host_statistics64) and sysctl. CRITICAL: Uses `mach_vm_deallocate` to prevent memory leaks from Mach API allocations.
  - **linux.rs**: Stub implementation (not yet functional)

- **main.rs**: Main monitoring loop that samples every 2 seconds, computes snapshots from consecutive samples, and renders output

### Data Flow
1. `PlatformSampler::sample()` calls platform-specific kernel interfaces to collect `RawSample`
2. Main loop stores current sample and compares with previous sample
3. `Snapshot::compute()` calculates CPU usage percentages from tick deltas
4. `Snapshot::render()` displays formatted metrics to stdout

### Testing
- Extensive unit tests in model.rs verify CPU usage calculations
- Property-based tests using proptest ensure percentage calculations are always valid (0-100%, sum to 100%)
- Tests cover edge cases: u32 wraparound, zero deltas, multiple CPUs

## Important Notes
- This project uses Cargo edition 2024
- Avoid OBVIOUS comments when writing code
- When writing a plan for a feature, write it to the current directory's .claude directory in a file named <feature_name>.plan.md


