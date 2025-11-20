#!/bin/bash
# Example script triggered when files are modified
# Receives the path to the modified directory as first argument

TARGET_PATH="$1"

echo "=========================="
echo "File modification detected!"
echo "Path: $TARGET_PATH"
echo "Time: $(date)"
echo "=========================="

# List contents of the directory
ls -lh "$TARGET_PATH"

# Example: Count files
FILE_COUNT=$(find "$TARGET_PATH" -type f | wc -l)
echo "Total files: $FILE_COUNT"

# Add your custom processing here
# Examples:
# - Run validation/tests
# - Trigger a build
# - Send a notification
# - Archive files
# - Process media files
# - Update a database
