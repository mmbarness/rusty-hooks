#!/bin/bash
cargo build --release
# Redirect stdout/stderr to /dev/null since log4rs already writes to /var/log/
nohup ./target/release/rusty-hooks --script-folder ./scripts --log-level debug > /dev/null 2>&1 &
echo "Started rusty-hooks with PID: $!"
echo "Logs are in /var/log/rusty-hooks*.log"
