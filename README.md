# htop_mini

A lightweight, terminal-based system monitoring tool written in Rust, inspired by [htop](https://htop.dev/).

![Rust](https://img.shields.io/badge/rust-2024-orange.svg)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey.svg)

## Overview

htop_mini is a minimal system resource monitor that provides real-time insights into your system's performance. Built with Rust for speed and reliability, it offers a clean terminal interface powered by [ratatui](https://ratatui.rs/) and [crossterm](https://github.com/crossterm-rs/crossterm).

## Features

### Currently Implemented

- **Per-Core CPU Monitoring**: Visual bars showing user/system CPU usage for each core
- **Memory Statistics**: Real-time tracking of:
  - Active, wired, and compressed memory
  - Total memory usage with visual bars
  - Swap memory usage
- **Task & Thread Statistics**: Monitor running tasks and threads
- **System Load Average**: 1, 5, and 15-minute load averages
- **System Uptime**: Formatted uptime display
- **Interactive TUI**: Clean, responsive terminal interface
- **Keyboard Controls**: Simple navigation (press `q` or `Esc` to quit)

### Platform Support

| Platform | Status |
|----------|--------|
| macOS    | ✅ Fully supported |
| Linux    | 🚧 In progress |

## Installation

### Prerequisites

- Rust toolchain
- Cargo

### Build from Source

```bash
# Clone the repository
git clone <repository-url>
cd htop_mini

# Build the project
cargo build --release

# Run the application
cargo run --release
```

## Usage

Simply run the executable to start monitoring:

```bash
cargo run
```

Or if you've built the release binary:

```bash
./target/release/htop_mini
```

### Keyboard Shortcuts

- `q` or `Esc` - Quit the application
- `Ctrl+C` - Force exit

### Configuration

- **Sample Interval**: Currently fixed at 1 second (configurable in source)

## Architecture

htop_mini follows a layered architecture designed for extensibility and testing:

### Core Components

- **model.rs**: Defines the data model
  - `RawSample`: Raw system metrics from kernel APIs
  - `Snapshot`: Computed metrics with CPU usage percentages

- **sampler/**: Platform-specific metric collection
  - Trait-based design for cross-platform support
  - `MacOsSampler`: Uses Mach kernel APIs and sysctl
  - `LinuxSampler`: In development

- **ui.rs**: Terminal UI rendering with ratatui
  - CPU usage bars with color-coded user/system time
  - Memory bars and statistics
  - System information display

### Data Flow

1. Platform-specific sampler collects raw metrics from kernel
2. Main loop samples every 2 seconds
3. Snapshots computed from consecutive samples (handles CPU tick deltas)
4. UI renders metrics in real-time TUI

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run a specific test
cargo test test_name

# Run with verbose output
cargo test -- --nocapture
```

The project includes:
- Unit tests for CPU usage calculations
- Property-based tests using proptest
- Edge case coverage (u32 wraparound, zero deltas, etc.)


## Up Next

### Short-term Roadmap

- [ ] **Process List**: Display running processes with CPU/memory usage
- [ ] **Interactive Sorting**: Sort processes by CPU, memory, PID, name
- [ ] **Process Filtering**: Search and filter processes
- [ ] **Process Management**: Kill/nice processes from the UI
- [ ] **Linux Support**: Complete Linux implementation using /proc filesystem



