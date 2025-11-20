#!/bin/bash
# Monitor rusty-hooks service

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================="
echo "  Rusty Hooks Service Monitor"
echo "========================================="
echo ""

# Check service status
echo -e "${YELLOW}Service Status:${NC}"
sudo systemctl status rusty-hooks --no-pager -l | head -15
echo ""

# Check log file sizes
echo -e "${YELLOW}Log File Sizes:${NC}"
ls -lh /var/log/rusty-hooks*.log 2>/dev/null | awk '{print $9, $5}'
echo ""

# Show recent errors from main log (last 10 minutes)
echo -e "${YELLOW}Recent Errors (last 10 min):${NC}"
ERRORS=$(sudo journalctl -u rusty-hooks --since "10 minutes ago" --no-pager | grep -i "error\|lagged\|panic" || echo "None found")
if [ "$ERRORS" == "None found" ]; then
    echo -e "${GREEN}✓ No errors in last 10 minutes${NC}"
else
    echo -e "${RED}$ERRORS${NC}"
fi
echo ""

# Show recent script executions
echo -e "${YELLOW}Recent Script Activity:${NC}"
if [ -f /var/log/rusty-hooks-script-output.log ]; then
    tail -5 /var/log/rusty-hooks-script-output.log | jq -r '.time + " | " + .level + " | " + .message' 2>/dev/null || tail -5 /var/log/rusty-hooks-script-output.log
else
    echo "No script output yet"
fi
echo ""

# Show watched paths
echo -e "${YELLOW}Watched Paths:${NC}"
grep "now watching path" /var/log/rusty-hooks.log | tail -5 | jq -r '.message' 2>/dev/null || echo "Check /var/log/rusty-hooks.log manually"
echo ""

echo "========================================="
echo "Quick Commands:"
echo "  View live logs:     sudo journalctl -u rusty-hooks -f"
echo "  Restart service:    sudo systemctl restart rusty-hooks"
echo "  Stop service:       sudo systemctl stop rusty-hooks"
echo "  View main log:      tail -f /var/log/rusty-hooks.log"
echo "  View script output: tail -f /var/log/rusty-hooks-script-output.log"
echo "========================================="
