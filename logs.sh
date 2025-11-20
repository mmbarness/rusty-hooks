#!/bin/bash
# Tail all rusty-hooks logs with pretty formatting

# Check if multitail is available
if command -v multitail &> /dev/null; then
    # Use multitail for split-screen view
    multitail \
        -l "tail -f /var/log/rusty-hooks.log | jq -r '.time + \" | \" + .level + \" | \" + .message'" \
        -l "tail -f /var/log/rusty-hooks-script-output.log | jq -r '.time + \" | \" + .level + \" | \" + .message'" \
        -l "tail -f /var/log/rusty-hooks-health-report.log | jq -r '.time + \" | \" + .level + \" | \" + .message'"
else
    # Fallback: use tail with labels
    echo "Tailing all rusty-hooks logs (Ctrl+C to stop)..."
    echo "Tip: Install 'multitail' for split-screen view: sudo apt install multitail"
    echo ""

    tail -f \
        /var/log/rusty-hooks.log \
        /var/log/rusty-hooks-script-output.log \
        /var/log/rusty-hooks-health-report.log
fi
