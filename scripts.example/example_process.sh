#!/bin/bash
# Example processing script for downloaded files
# Receives the path to the new download as first argument

DOWNLOAD_PATH="$1"
PROCESSED_DIR="/path/to/processed"

echo "Processing download: $DOWNLOAD_PATH"

# Check if path exists
if [ ! -e "$DOWNLOAD_PATH" ]; then
    echo "✗ Path doesn't exist (file may have been moved/deleted)"
    exit 0
fi

# Create processed directory
mkdir -p "$PROCESSED_DIR"

# Example processing steps:
if [ -d "$DOWNLOAD_PATH" ]; then
    echo "Processing directory: $DOWNLOAD_PATH"

    # Example: Process video files
    # find "$DOWNLOAD_PATH" -type f \( -name "*.mp4" -o -name "*.mkv" \) -exec ffmpeg -i {} -c:v libx264 {} \;

    # Example: Extract archives
    # find "$DOWNLOAD_PATH" -type f -name "*.zip" -exec unzip -d "$PROCESSED_DIR" {} \;

    echo "✓ Processing complete"
elif [ -f "$DOWNLOAD_PATH" ]; then
    echo "Processing file: $DOWNLOAD_PATH"

    # Example: Move to processed directory
    # mv "$DOWNLOAD_PATH" "$PROCESSED_DIR/"

    echo "✓ File processed"
fi
