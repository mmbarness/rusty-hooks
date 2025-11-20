#!/bin/bash
# Example backup script
# Receives the path to the changed directory as first argument

SOURCE_PATH="$1"
BACKUP_DIR="/path/to/backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

echo "Backing up: $SOURCE_PATH"

# Create backup directory if it doesn't exist
mkdir -p "$BACKUP_DIR"

# Create a tarball backup
BASENAME=$(basename "$SOURCE_PATH")
BACKUP_FILE="$BACKUP_DIR/${BASENAME}_${TIMESTAMP}.tar.gz"

tar -czf "$BACKUP_FILE" -C "$(dirname "$SOURCE_PATH")" "$(basename "$SOURCE_PATH")"

if [ $? -eq 0 ]; then
    echo "✓ Backup created: $BACKUP_FILE"

    # Optional: Remove old backups (keep last 7 days)
    find "$BACKUP_DIR" -name "*.tar.gz" -mtime +7 -delete
else
    echo "✗ Backup failed!"
    exit 1
fi
