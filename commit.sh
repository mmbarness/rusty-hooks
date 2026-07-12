#!/bin/bash
# Auto-commit using Claude CLI to generate a short commit message

set -e

# Check for claude CLI
if ! command -v claude &> /dev/null; then
    echo "Error: 'claude' command not found"
    echo "Install Claude Code from: https://claude.ai/code"
    exit 1
fi

echo "========================================="
echo "  Claude-Powered Git Commit"
echo "========================================="
echo ""

# Stage files
echo "Staging files..."
git add .gitignore
git add CLAUDE.md
git add start_script.sh stop.sh monitor.sh logs.sh install-service.sh
git add rusty-hooks.service.example
git add scripts.example/
git add src/main.rs
git add src/utilities/
git add src/runner/
git add src/watcher/
git add src/health_reporter/
git add src/errors/
git add src/scripts/load.rs src/scripts/structs.rs
git add Cargo.lock log4rs.yml
git add rust-toolchain.toml
git add tests/files/scripts/

echo "✓ Files staged"
echo ""

# Get the diff
echo "Analyzing changes..."
DIFF=$(git diff --cached --stat)
DIFF_DETAILS=$(git diff --cached | head -150)

echo "Changes to commit:"
echo "$DIFF"
echo ""

# Generate commit message using Claude CLI
echo "Generating commit message with Claude CLI..."

PROMPT="Write a git commit message based on these staged changes.

CRITICAL REQUIREMENTS:
- Maximum 10 lines TOTAL
- First line: One-line summary (max 72 chars)
- Then blank line
- Then 3-5 brief bullet points only
- NO explanations, NO prose, NO extra text
- Be concise and specific

Staged changes:
$DIFF

File changes (first 150 lines):
$DIFF_DETAILS

Output ONLY the commit message, nothing else."

# Use claude CLI in headless mode
COMMIT_MSG=$(echo "$PROMPT" | claude -p --output-format text 2>/dev/null)

if [ -z "$COMMIT_MSG" ]; then
    echo "Error: Failed to generate commit message"
    exit 1
fi

echo "========================================="
echo "Generated commit message:"
echo "========================================="
echo "$COMMIT_MSG"
echo "========================================="
echo ""

read -p "Use this commit message? (y/N) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled. Files are still staged."
    echo "To unstage: git reset"
    exit 0
fi

# Create commit
git commit -m "$COMMIT_MSG"

echo ""
echo "✓ Commit created!"
echo ""
echo "To push: git push origin main"
