#!/bin/bash
set -euo pipefail

# Clean build to get accurate size
cargo build --release 2>/dev/null

# Binary size in bytes
SIZE=$(stat -f%z target/release/splitwise)
SIZE_KB=$((SIZE / 1024))

echo "METRIC binary_size_kb=$SIZE_KB"

# Count total crate dependencies
DEP_COUNT=$(cargo tree 2>/dev/null | wc -l | tr -d ' ')
echo "METRIC dep_count=$DEP_COUNT"

# Verify the binary still works
if ./target/release/splitwise --help >/dev/null 2>&1; then
  echo "METRIC help_works=1"
else
  echo "METRIC help_works=0"
  exit 1
fi

# Verify TUI flag exists
if ./target/release/splitwise --help 2>&1 | grep -q -- '--tui'; then
  echo "METRIC tui_flag=1"
else
  echo "METRIC tui_flag=0"
  exit 1
fi
