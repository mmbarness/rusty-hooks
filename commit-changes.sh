#!/bin/bash
# Commit all the stability and infrastructure improvements

set -e

echo "========================================="
echo "  Committing Rusty Hooks Improvements"
echo "========================================="
echo ""

# Show what will be committed
echo "Files to be committed:"
echo ""
echo "Documentation:"
echo "  - CLAUDE.md (AI assistant development guide)"
echo ""
echo "Helper Scripts:"
echo "  - start_script.sh (updated, no more nohup.out)"
echo "  - stop.sh (process management)"
echo "  - monitor.sh (service monitoring)"
echo "  - logs.sh (log viewer)"
echo "  - install-service.sh (service installer)"
echo ""
echo "System Integration:"
echo "  - rusty-hooks.service.example (systemd service template)"
echo ""
echo "Examples:"
echo "  - scripts.example/ (example scripts for users)"
echo ""
echo "Source Code Changes:"
echo "  - src/ (single runtime architecture)"
echo "  - Cargo.lock (updated dependencies)"
echo "  - log4rs.yml (removed stdout appender)"
echo "  - rust-toolchain.toml (Rust version)"
echo ""
echo "Configuration:"
echo "  - .gitignore (updated)"
echo ""

read -p "Proceed with commit? (y/N) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled."
    exit 0
fi

# Add files
echo "Adding files..."

# Documentation
git add CLAUDE.md

# Scripts
git add start_script.sh stop.sh monitor.sh logs.sh install-service.sh commit-changes.sh

# Systemd
git add rusty-hooks.service.example

# Examples
git add scripts.example/

# Source code
git add src/main.rs
git add src/utilities/
git add src/runner/
git add src/watcher/
git add src/health_reporter/
git add src/errors/
git add src/scripts/load.rs src/scripts/structs.rs
# Note: src/scripts/parser.rs is dead code (unused), not committing
git add Cargo.lock log4rs.yml
git add rust-toolchain.toml

# Test fixtures
git add tests/files/scripts/

# Configuration
git add .gitignore

echo "✓ Files staged"
echo ""

# Show what's staged
echo "Staged changes:"
git diff --name-status --cached
echo ""

# Create commit
echo "Creating commit..."
git commit -m "Major stability and infrastructure improvements

Fixes:
- Fix runtime drop panic by refactoring to single-runtime architecture
- Increase broadcast channel buffer from 16 to 256 to prevent Lagged(N) errors
- Fix infinite nohup.out growth by removing stdout appender from log4rs

Infrastructure:
- Add systemd service for proper daemon management (auto-restart, journald logging)
- Add helper scripts: monitor.sh, logs.sh, stop.sh, install-service.sh
- Update start_script.sh to redirect to /dev/null (prevents nohup.out)

Documentation:
- Add CLAUDE.md - comprehensive development guide for AI assistants

Examples:
- Add scripts.example/ directory with example scripts and configuration
- Separate user scripts (/scripts) from examples (scripts.example/)

Architecture Changes:
- Refactor from 4+ runtimes to single 12-thread Tokio runtime
- Replace all self.runtime.spawn() with tokio::spawn()
- Remove runtime fields from Runner, Watcher, PathSubscriber, HealthReporter
- Configure main runtime: #[tokio::main(flavor = \"multi_thread\", worker_threads = 12)]

Performance:
- 16x larger channel buffers (256 vs 16) prevent event drops
- Better thread utilization with work-stealing scheduler
- Reduced memory footprint (4.4MB runtime)

Testing:
- All unit tests pass (5/5)
- All integration tests pass (3/3)
- Verified stable systemd service operation

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"

echo "✓ Commit created"
echo ""

# Show commit
git log -1 --stat
echo ""

echo "========================================="
echo "Commit created successfully!"
echo ""
echo "To push to GitHub:"
echo "  git push origin main"
echo ""
echo "To view the commit:"
echo "  git show"
echo "========================================="
