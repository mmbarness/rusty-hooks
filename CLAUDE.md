# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rusty-Hooks is a filesystem event watcher that triggers user-provided bash scripts based on file system events. It uses the `notify` crate to monitor directories and execute scripts when specific events (Create, Modify, Delete) occur. The application is multithreaded and can efficiently watch multiple directories concurrently.

## Build and Development Commands

### Building
```bash
# Build the project
cargo build

# Build release binary
cargo build --release
```

### Testing
```bash
# Run all tests
cargo test

# Run a specific test
cargo test <test_name>

# Run tests with nightly toolchain (as used in CI)
cargo +nightly test
```

### Linting and Formatting
```bash
# Format code
cargo fmt --all

# Check formatting without making changes
cargo fmt --all -- --check

# Run clippy linter
cargo clippy -- -D warnings
```

### Compilation Check
```bash
# Check if code compiles without building
cargo check
```

### Running the Application

**For production (recommended):**
```bash
# Install and run as systemd service (see RUNNING.md)
./install-service.sh
sudo systemctl start rusty-hooks
```

**For development:**
```bash
# Run with a script configuration folder
cargo run -- --script-folder ./scripts --log-level debug

# Or with the release binary
target/release/rusty-hooks --script-folder /path/to/scripts --log-level error
```

**Important**: Don't use `nohup` without redirecting to `/dev/null` - the app already has rotating file logs in `/var/log/`. See `RUNNING.md` for details.

## Architecture

The application uses a single-runtime architecture where all async tasks run on the main Tokio runtime (configured with 12 worker threads). Three main concurrent systems communicate via broadcast channels:

### 1. Watcher System (src/watcher/)
- **Purpose**: Monitors filesystem events using the `notify` crate
- **Key Components**:
  - `Watcher::new()` creates a watcher instance (no dedicated runtime)
  - `notifier_task()` sets up the `notify::RecommendedWatcher` with a callback that broadcasts events
  - Three concurrent tasks spawned with `tokio::spawn`:
    1. **Event Loop** (`watch_events`): Receives filesystem events from notify and filters them by event type
    2. **Subscription Task** (`route_subscriptions`): Creates path subscriptions when matching events occur
    3. **Unsubscribe Task**: Cleans up processed paths from the subscription map
- **Event Flow**: FileSystem → notify → Broadcast Channel (256 buffer) → Event Filtering → Path Subscription

### 2. Runner System (src/runner/)
- **Purpose**: Executes scripts for paths that have been subscribed
- **Key Components**:
  - `Runner::new()` creates channel pairs (no dedicated runtime)
  - `init()` listens on `spawn_channel` for (PathBuf, Vec<Script>) tuples
  - Spawns tasks with `tokio::spawn` for each path to run all associated scripts concurrently
  - After scripts complete, sends path to `unsubscribe_broadcast_channel` to clean up
- **Script Execution**: Uses `async_process::Command` to run shell scripts with the target path as an argument
- **Retry Logic**: Implements recursive unsubscribe with 5 retries if channel send fails

### 3. Health Reporter System (src/health_reporter/)
- **Purpose**: Periodic health check logging
- **Behavior**: Logs "rusty hooks is healthy" every 30 seconds to a separate log target using `tokio::spawn`

### Channel Architecture
The three systems communicate via Tokio broadcast channels (buffer size: 256):
- `spawn_channel`: Watcher → Runner (signals scripts to execute)
- `unsubscribe_broadcast_channel`: Runner → Watcher (signals path processing complete)

**Note**: The buffer size was increased from 16 to 256 to prevent `Lagged(N)` errors during bursts of filesystem events.

### Main Orchestration (src/main.rs)
- Initializes all three systems and spawns their tasks
- Uses `tokio::select!` to monitor all tasks
- If any task exits or fails, the entire application terminates
- Sets a lockfile to ensure only one instance runs at a time

## Configuration System

### Scripts Configuration (src/scripts/)
- Configuration is loaded from a YAML file (`scripts.schema.json` defines the schema)
- Each script configuration specifies:
  - `watch_path`: Absolute path to monitor
  - `file_name`: Script file to execute (or inline `script`)
  - `event_triggers`: Array of event types (Create, Modify, Delete)
  - `enabled`: Boolean flag
  - `run_delay`: Seconds to wait before executing (defaults to 0)
  - `dependencies`: List of scripts that must complete first (not yet implemented)
- Scripts are validated on startup and cached in a HashMap keyed by EventKind
- The config file and script files must be in the same directory

### Event Mapping
Event triggers from YAML are mapped to notify's EventKind:
- "Access" → `EventKind::Access(AccessKind::Any)`
- "Create" → `EventKind::Create(CreateKind::Any)`
- "Modify" → `EventKind::Modify(ModifyKind::Name(RenameMode::To))`
- "Remove" → `EventKind::Remove(RemoveKind::Any)`
- "Other" → `EventKind::Other`

Note: Event matching currently only supports top-level EventKind categories, not subcategories.

## Logging

- Uses `log4rs` for logging configured via `log4rs.yml`
- Log levels: error, info, debug
- **Log locations** (with automatic rotation, 50MB limit, 5 archived files):
  - Main logs: `/var/log/rusty-hooks.log` (JSON format)
  - Health reports: `/var/log/rusty-hooks-health-report.log` (JSON format)
  - Script output: `/var/log/rusty-hooks-script-output.log` (JSON format)
  - Archived logs: `/var/log/rusty-hooks_*.gz`
- **Console output**: Disabled in production (stdout appender removed from root logger)
  - Prevents infinite growth of nohup.out when running as daemon
  - Re-enable by adding `stdout` to root appenders in log4rs.yml for development

## Important Notes

- **Nightly Toolchain Required**: The project uses `#![feature(io_error_more)]` and requires Rust nightly
- **Rust Version**: 1.83.0 (see `rust-toolchain.toml` and Cargo.toml)
- **Single Runtime Architecture**: All async operations run on the main Tokio runtime (12 worker threads). Previous architecture with multiple runtimes per component caused "Cannot drop a runtime in a context where blocking is not allowed" panics.
- **Channel Buffer Sizing**: Broadcast channels use 256-item buffers to handle bursts of filesystem events without dropping messages
- **Lockfile**: Uses `fs2` to create a lockfile preventing multiple instances
- **Test Structure**: Integration tests are in `tests/` and use `rexpect` for spawning and interacting with the binary
- **Error Handling**: Custom error types in `src/errors/` with conversions between different error domains

## Common Development Patterns

- All major components (Watcher, Runner, HealthReporter, PathSubscriber) implement the `Utilities` trait for creating channels
- All async tasks are spawned using `tokio::spawn` on the main runtime
- Async operations use Tokio with the "full" feature set and 12 worker threads
- Path validation happens at configuration load time, not runtime
- Scripts receive the canonicalized path as their first argument
- Use broadcast channels for one-to-many communication patterns
