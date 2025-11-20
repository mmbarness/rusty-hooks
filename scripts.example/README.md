# Example Scripts

This directory contains example scripts to help you get started with rusty-hooks.

## Quick Start

1. **Copy this directory:**
   ```bash
   cp -r scripts.example scripts
   ```

2. **Edit the configuration:**
   ```bash
   nano scripts/scripts.yml
   ```
   - Update `watch_path` to directories you want to monitor
   - Set `enabled: true` for scripts you want to use
   - Adjust `run_delay` if needed (time to wait before executing)

3. **Customize the scripts:**
   ```bash
   nano scripts/example_on_modify.sh
   ```
   - Modify the scripts to do what you need
   - Scripts receive the modified directory path as `$1`

4. **Test your setup:**
   ```bash
   cargo run -- --script-folder ./scripts --log-level debug
   ```

5. **Deploy to production:**
   ```bash
   ./install-service.sh
   sudo systemctl start rusty-hooks
   ```

## Available Examples

### example_on_modify.sh
Basic example showing how to react to file modifications. Demonstrates:
- Receiving the path argument
- Logging information
- Listing directory contents
- Counting files

### example_backup.sh
Automated backup script. Features:
- Creates timestamped tar.gz backups
- Automatically cleans up old backups (7+ days)
- Error handling

### example_process.sh
File processing template. Useful for:
- Processing downloads
- Converting media files
- Extracting archives
- Moving files to destinations

## Script Requirements

All scripts must:
1. Be executable (`chmod +x script.sh`)
2. Accept directory path as first argument: `$1`
3. Be in the same directory as `scripts.yml`

## Event Triggers

Available event types in `scripts.yml`:
- `"Modify"` - File/directory modified, renamed, or created
- `"Create"` - New file/directory created
- `"Remove"` - File/directory deleted
- `"Access"` - File/directory accessed

**Note:** Currently only `"Modify"` triggers are fully supported. The notify crate provides the events.

## Configuration Options

### run_delay
Time in seconds to wait after detecting a change before running the script.
- Useful for ensuring file operations complete
- Downloads: Use 10+ seconds
- Quick edits: Use 0-5 seconds

### dependencies
Future feature for script ordering (not yet implemented).

## Tips

1. **Test scripts manually first:**
   ```bash
   ./scripts/example_on_modify.sh /some/test/path
   ```

2. **Check logs:**
   ```bash
   tail -f /var/log/rusty-hooks-script-output.log
   ```

3. **Handle errors gracefully:**
   - Check if paths exist before processing
   - Use proper exit codes (0 = success, 1 = error)
   - Log important information

4. **Security:**
   - Validate input paths
   - Don't run with unnecessary privileges
   - Be careful with file operations

## Common Use Cases

### Media Automation
Monitor download folders and automatically:
- Rename files
- Convert formats
- Move to media library
- Update Plex/Jellyfin

### Development Workflow
Watch source directories and:
- Trigger builds
- Run tests
- Deploy changes
- Update documentation

### Backup Automation
Monitor important directories and:
- Create incremental backups
- Sync to remote storage
- Archive old files
- Send notifications

### File Organization
Watch incoming directories and:
- Sort by file type
- Rename based on metadata
- Extract archives
- Remove duplicates
