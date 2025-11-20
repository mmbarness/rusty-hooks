#!/bin/bash
# Stop rusty-hooks processes

echo "Searching for rusty-hooks processes..."
PIDS=$(pgrep -f "rusty-hooks.*--script-folder")

if [ -z "$PIDS" ]; then
    echo "No rusty-hooks processes found."
    exit 0
fi

echo "Found rusty-hooks processes:"
ps -fp $PIDS

echo ""
read -p "Kill these processes? (y/N) " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Killing processes..."
    kill $PIDS
    sleep 1

    # Check if any are still running
    REMAINING=$(pgrep -f "rusty-hooks.*--script-folder")
    if [ -n "$REMAINING" ]; then
        echo "Some processes didn't stop, force killing..."
        kill -9 $REMAINING
    fi

    echo "All rusty-hooks processes stopped."
else
    echo "Cancelled."
fi
