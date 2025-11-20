#!/bin/bash
# Install rusty-hooks as a systemd service

set -e

echo "Building release binary..."
cargo build --release

echo "Creating systemd service from template..."
if [ ! -f rusty-hooks.service ]; then
    # Create service file from example if it doesn't exist
    sed -e "s|YOUR_USERNAME|$USER|g" \
        -e "s|/path/to/rusty-hooks|$PWD|g" \
        rusty-hooks.service.example > rusty-hooks.service
    echo "✓ Created rusty-hooks.service from template"
fi

echo "Installing systemd service..."
sudo cp rusty-hooks.service /etc/systemd/system/

echo "Reloading systemd daemon..."
sudo systemctl daemon-reload

echo "Enabling rusty-hooks service..."
sudo systemctl enable rusty-hooks

echo ""
echo "Installation complete!"
echo ""
echo "To start the service:"
echo "  sudo systemctl start rusty-hooks"
echo ""
echo "To check status:"
echo "  sudo systemctl status rusty-hooks"
echo ""
echo "To view logs:"
echo "  sudo journalctl -u rusty-hooks -f"
echo "  or check /var/log/rusty-hooks*.log"
echo ""
echo "To stop the service:"
echo "  sudo systemctl stop rusty-hooks"
