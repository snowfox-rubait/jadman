#!/usr/bin/env bash

# start_jadman.sh: Auto-bootstrap script for JADMan downloads manager.
# Creates a tmux session named "jadman" and launches the daemon, TUI, and aria2c RPC dependency.

SESSION_NAME="jadman"
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Locate binaries
DAEMON_BIN="$REPO_DIR/target/release/jadm-daemon"
TUI_BIN="$REPO_DIR/target/release/jadm-tui"

# Fallback to system-wide installed path if target/release hasn't been built
if [ ! -f "$DAEMON_BIN" ]; then
    DAEMON_BIN=$(which jadm-daemon 2>/dev/null || echo "jadm-daemon")
fi

if [ ! -f "$TUI_BIN" ]; then
    TUI_BIN=$(which jadm-tui 2>/dev/null || echo "jadm-tui")
fi

# 1. Check if tmux session already exists
if tmux has-session -t "$SESSION_NAME" 2>/dev/null; then
    echo "Tmux session '$SESSION_NAME' already exists. Attaching..."
    tmux attach-session -t "$SESSION_NAME"
    exit 0
fi

echo "Starting JADMan tmux environment..."

# 2. Start detached tmux session with the 'daemon' window
tmux new-session -d -s "$SESSION_NAME" -n "daemon" -c "$REPO_DIR"
sleep 0.3
tmux send-keys -t "$SESSION_NAME:daemon" "RUST_BACKTRACE=1 $DAEMON_BIN" ENTER

# 3. Create window for aria2c RPC server
tmux new-window -t "$SESSION_NAME" -n "aria2c" -c "$REPO_DIR"
sleep 0.3
tmux send-keys -t "$SESSION_NAME:aria2c" "aria2c --enable-rpc --rpc-listen-all --rpc-allow-origin-all" ENTER

# 4. Create window for the interactive TUI
tmux new-window -t "$SESSION_NAME" -n "tui" -c "$REPO_DIR"
sleep 0.3
tmux send-keys -t "$SESSION_NAME:tui" "$TUI_BIN" ENTER

# 5. Create a spare helper shell window
tmux new-window -t "$SESSION_NAME" -n "shell" -c "$REPO_DIR"

# 6. Select the TUI window as active by default
tmux select-window -t "$SESSION_NAME:tui"

# 7. Attach to the session
tmux attach-session -t "$SESSION_NAME"
