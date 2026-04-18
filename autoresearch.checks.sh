#!/bin/bash
set -euo pipefail

# Must compile without errors
cargo build --release 2>&1 | tail -5

# Binary must exist and be executable
test -x target/release/splitwise

# Help must show all major commands
HELP=$(./target/release/splitwise --help 2>&1)
for cmd in auth me friends groups expenses balances currencies; do
  if ! echo "$HELP" | grep -q "$cmd"; then
    echo "FAIL: missing command '$cmd' in help output"
    exit 1
  fi
done

echo "All checks passed"
