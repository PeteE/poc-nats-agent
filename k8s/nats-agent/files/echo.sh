#!/bin/bash
# NOTE: /bin/bash, not /usr/bin/env bash. The Nix-built image lays bash out
# under /bin and /sbin and creates no /usr at all, so the kernel cannot
# resolve /usr/bin/env and execve fails ENOENT -- which the agent surfaces
# as "Failed to spawn handler command". /bin/bash is a symlink to the real
# bashInteractive in the Nix store.
# Example message handler script
# Reads a JSON event from stdin, processes it, and exits with appropriate code
#
# The event body on stdin is payload only. What KIND of event this is comes
# from the NATS subject, which the agent passes in the environment:
#   NATS_SUBJECT - subject the message was published to (e.g.
#                  events.workflow.executed)
#   NATS_MSG_ID  - the event's "id" field, or "unknown"

set -euo pipefail

# Read the JSON event from stdin
EVENT=$(cat)

# Subject is the source of truth for the event kind. Default so the script
# still runs when invoked by hand outside the agent.
SUBJECT="${NATS_SUBJECT:-<none>}"
MSG_ID="${NATS_MSG_ID:-unknown}"

echo "Processing event: subject=${SUBJECT} id=${MSG_ID}"

# Branch on the subject rather than on a field inside the body.
case "$SUBJECT" in
    *.workflow.executed) echo "  -> workflow execution finished" ;;
    *.workflow.created)  echo "  -> new workflow created" ;;
    *.workflow.deleted)  echo "  -> workflow deleted" ;;
    *.workflow.failed)   echo "  -> workflow execution failed" ;;
    *)                   echo "  -> no specific handling for this subject" ;;
esac

# Parse the event (using jq if available, otherwise just echo)
if command -v jq &> /dev/null; then
    echo "$EVENT" | jq
else
    # If jq is not available, just echo the raw event
    echo "(jq not available for parsing)"
    echo "$EVENT"
fi

echo "Successfully processed event"
exit 0
