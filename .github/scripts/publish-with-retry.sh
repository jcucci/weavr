#!/usr/bin/env bash
set -euo pipefail

# Publish a crate with exponential backoff retries for crates.io index propagation.
# Usage: publish-with-retry.sh <crate-name>

CRATE="$1"
MAX_RETRIES=10
DELAY=5

for attempt in $(seq 1 "$MAX_RETRIES"); do
  if cargo publish -p "$CRATE"; then
    echo "Successfully published $CRATE"
    exit 0
  fi

  if [ "$attempt" -eq "$MAX_RETRIES" ]; then
    echo "Failed to publish $CRATE after $MAX_RETRIES attempts"
    exit 1
  fi

  echo "Publish of $CRATE failed, retrying in ${DELAY}s (attempt $attempt/$MAX_RETRIES)..."
  sleep "$DELAY"
  DELAY=$((DELAY * 2))
done
