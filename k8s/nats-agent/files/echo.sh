#!/usr/bin/env bash
# Example message handler script
# Reads a JSON event from stdin, processes it, and exits with appropriate code

set -euo pipefail

# Read the JSON event from stdin
EVENT=$(cat)

# Parse the event (using jq if available, otherwise just echo)
if command -v jq &> /dev/null; then
    echo "Processing event" 
    echo "$EVENT" | jq
    echo "Successfully processed event"
    exit 0
else
    # If jq is not available, just echo the raw event
    echo "Received event (jq not available for parsing):"
    echo "$EVENT"
    exit 0
fi
