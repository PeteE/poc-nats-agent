# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**NATS Agent POC** - A learning project for Rust and NATS.io messaging. This is
a generic NATS JetStream consumer that delegates message processing to external
handler programs via stdin/stdout.

Despite the repo name, **there is nothing AI- or LLM-related in this codebase**.
"agent" here means the `nats-agent` binary — the consumer process — and nothing
more. It has no notion of models, prompts, or tools. Do not describe this
project as an AI agent system.

A handler that calls an LLM would be a plausible future addition (handlers are
just programs that read JSON on stdin), but none exists today.

**Learning Goals:** Rust async programming (Tokio), NATS.io messaging patterns
(pub/sub, queues, JetStream), and the external-handler pattern — keeping the
consumer generic while business logic lives in separate programs.

## Development Environment

This project uses **Nix flakes** for development environment and container builds. All dependencies are managed through Nix.

```bash
# Enter development shell (provides all tools: nats CLI, kubectl, helm, tilt, etc.)
nix develop

# The shell automatically sets environment variables:
# - NATS_URL=localhost:4222
# - NATS_STREAM_NAME=events
# - NATS_SUBJECTS="events.>"
# - NATS_CONSUMER_NAME=all-events
# - OTEL_SERVICE_NAME=nats-agent
# - OTEL_EXPORTER_OTLP_ENDPOINT=localhost:4317
```

## Build Commands

```bash
# Build the main agent binary
cargo build
nix build .#nats-agent

# Build the producer binary (test event generator)
cargo build --bin producer
nix build .#producer

# Build container image (outputs to ./result as tarball stream)
nix build .#nats-agent-container

# Load container to local Docker
nix build .#nats-agent-container && ./result | docker load

# Push container to registry (via skopeo)
nix build .#nats-agent-container && ./result | gzip --fast | skopeo copy docker-archive:/dev/stdin docker://reg.wheat-dn42.net/poc-nats-agent:latest
```

## Running Locally

```bash
# Run the agent (requires NATS server running on localhost:4222)
cargo run

# Run the producer (publishes test events)
cargo run --bin producer -- --count 10 --subject events.workflow.executed

# Producer with custom payload
cargo run --bin producer -- --count 5 --payload '{"custom": "data"}'

# Producer with OpenTelemetry metrics
cargo run --bin producer -- --otel-endpoint http://localhost:4317
```

## Development Workflow with Tilt

The project uses **Tilt** for local Kubernetes development with Minikube:

```bash
# Start full development environment (NATS, OpenTelemetry, Prometheus, agent)
tilt up

# This deploys:
# - cert-manager (for OpenTelemetry operator)
# - OpenTelemetry operator + collector
# - kube-prometheus-stack (Prometheus + Grafana)
# - NATS server with JetStream enabled
# - nats-agent Helm chart

# Port forwards automatically configured:
# - 4222: NATS client port
# - 8080: nats-agent HTTP server (/health, /status)
# - 9090: Prometheus UI
# - 4317: OpenTelemetry collector (OTLP gRPC)
```

**Image Build Process:** Tilt uses `custom_build()` which:
1. Runs `nix build .#nats-agent-container`
2. Pipes result through `gzip --fast`
3. Pushes to registry via `skopeo copy docker-archive:/dev/stdin docker://reg.wheat-dn42.net/poc-nats-agent:latest`
4. Updates Helm deployment with new image

## Architecture

### High-Level Flow

```
Producer → NATS JetStream → nats-agent → External Handler → Response
                                ↓
                         OpenTelemetry Collector → Prometheus
```

### Key Components

**1. Event Model** (`src/event.rs`)
- `Event` is a type alias for `serde_json::Value` — the agent accepts any
  JSON payload without requiring a specific structure
- The body carries **payload only**. What kind of event it is comes from the
  NATS **subject** it was published to, which is the single source of truth.
  Nothing re-states the kind inside the body (an `event_type` field used to,
  and could silently drift from the subject)

**2. NATS Client** (`src/nats_client.rs`)
- Connects to NATS server
- Creates or gets JetStream stream and consumer
- Configured via environment variables (see `src/config.rs`)

**3. Message Processor** (`src/processor.rs`)
- Pulls messages in batches from NATS JetStream consumer
- Deserializes events from message payload
- Delegates to message handler
- Records OpenTelemetry metrics (histogram with success/failed status)
- Acknowledges messages after successful processing

**4. Message Handler** (`src/message_handler.rs`)
- **External Handler Pattern**: Executes arbitrary programs to process events
- Handler receives the JSON event body via **stdin** (payload only)
- Handler receives routing metadata via **environment**: `NATS_SUBJECT`
  (what kind of event this is) and `NATS_MSG_ID`
- Handler exits with code **0 = success**, **non-zero = failure**
- Handler stdout/stderr logged by agent
- Configured via `MESSAGE_HANDLER_CMD` environment variable

**5. HTTP Server** (`src/http_server.rs`)
- Axum-based server with two endpoints:
  - `GET /health` - Liveness check (returns 200 OK)
  - `GET /status` - Runtime stats (messages processed, errors, uptime, system metrics)
- Shares `AppState` with processor to track counters

**6. OpenTelemetry Metrics** (`src/telemetry.rs`)
- Exports to OTLP endpoint (default: localhost:4317)
- **Processor metrics:**
  - `messages.duration` (histogram) - Processing latency with labels: `status` (success/failed), `subject`, `error`
- **Producer metrics:**
  - `producer.events.published` (counter)
  - `producer.events.failed` (counter)
  - `producer.publish.duration` (histogram)

### Concurrent Execution

The main binary (`src/main.rs`) uses `tokio::select!` to run two tasks concurrently:
- HTTP server (health/status endpoints)
- NATS message processor (infinite loop)

Both share `AppState` for coordinating runtime statistics.

## Kubernetes Deployment

### Helm Chart Structure

Located in `k8s/nats-agent/`:

```
k8s/nats-agent/
├── Chart.yaml
├── values.yaml
├── templates/
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── serviceaccount.yaml
│   ├── configmap.yaml      # Dynamically loads handler scripts
│   └── _helpers.tpl
└── files/
    └── echo.sh              # Example handler (random sleep 20ms-1s)
```

**ConfigMap Pattern:** Handler scripts are mounted from ConfigMap based on `handlerScript.filename` value:
```yaml
# ConfigMap loads: k8s/nats-agent/files/{{ .Values.handlerScript.filename }}
# Mounted at: /app/handler.sh
# MESSAGE_HANDLER_CMD=/app/handler.sh
```

### Helm Values Configuration

Key configuration sections in `values.yaml`:

```yaml
nats:
  url: "nats://nats:4222"
  streamName: "events"
  subjects: "events.>"      # Wildcard subject filter
  consumerName: "nats-agent"
  batchSize: 10

messageHandler:
  command: "/app/handler.sh"

handlerScript:
  enabled: true
  filename: "echo.sh"       # Change to use different handler from files/

opentelemetry:
  enabled: false            # Set true to enable OTLP metrics export
  serviceName: "nats-agent"
  endpoint: ""              # e.g., http://otel-collector:4317
```

### Deploying with Helm

```bash
# Deploy via Tilt (recommended for development)
tilt up

# Manual Helm deployment
helm install poc-nats-agent ./k8s/nats-agent \
  --set nats.url=nats://nats.nats-io.svc.cluster.local:4222 \
  --set opentelemetry.enabled=true \
  --set opentelemetry.endpoint=http://otel-collector-collector.otel-system.svc.cluster.local:4317

# Use custom handler script
helm upgrade poc-nats-agent ./k8s/nats-agent \
  --set handlerScript.filename=my-custom-handler.sh
```

## Writing Custom Handler Scripts

Handler scripts must follow this contract:

1. **Read JSON event from stdin** (single line or multi-line)
2. **Process the event** (call APIs, update DB, etc.)
3. **Exit with code 0** on success, **non-zero** on failure
4. **Optional:** Write to stdout (logged as info) or stderr (logged as error)

**Environment provided to the handler:** stdin carries the payload only, so
the agent passes the routing metadata via environment variables:

| Variable | Description |
|----------|-------------|
| `NATS_SUBJECT` | Subject the message arrived on, e.g. `events.workflow.executed`. **This is how a handler knows what kind of event it got.** |
| `NATS_MSG_ID` | The event's `id` field, or `unknown` |

**Example handler** (`k8s/nats-agent/files/echo.sh`):
```bash
#!/bin/bash
# /bin/bash, NOT /usr/bin/env bash — the Nix image has no /usr directory,
# so a /usr/bin/env shebang fails to exec.
set -euo pipefail

# Read JSON event from stdin (payload only)
EVENT=$(cat)

# Kind of event comes from the subject, not from the body. Defaults let the
# script still run when invoked by hand.
SUBJECT="${NATS_SUBJECT:-<none>}"
MSG_ID="${NATS_MSG_ID:-unknown}"

echo "Processing event: subject=${SUBJECT} id=${MSG_ID}"

case "$SUBJECT" in
    *.workflow.executed) echo "  -> workflow execution finished" ;;
    *.workflow.created)  echo "  -> new workflow created" ;;
    *)                   echo "  -> no specific handling for this subject" ;;
esac

# Your business logic here
if command -v jq &> /dev/null; then
    echo "$EVENT" | jq
else
    echo "$EVENT"
fi

echo "Successfully processed event"
exit 0
```

**To add a new handler:**
1. Create script in `k8s/nats-agent/files/your-handler.sh`
2. Make it executable: `chmod +x k8s/nats-agent/files/your-handler.sh`
3. Update Helm value: `handlerScript.filename: "your-handler.sh"`

## NATS JetStream Setup

**Creating a stream:**
```bash
nats stream add events \
  --subjects 'events.*' \
  --description 'Stream containing all events' \
  --storage file \
  --ack \
  --retention limits \
  --discard old
```

**Creating a consumer:**
```bash
nats consumer create events nats-agent \
  --filter=events.> \
  --description='NATS Agent Consumer' \
  --ack=explicit \
  --pull \
  --deliver=all \
  --max-deliver=-1 \
  --replay=instant \
  --no-headers-only
```

**Publishing test events:**
```bash
# Using the producer binary
cargo run --bin producer -- --subject events.workflow.executed --count 10

# Using nats CLI
nats pub events.workflow.executed '{"id":"123","workflow_id":"wf-1","status":"completed"}'
```

## Monitoring & Observability

**Metrics Flow:**
```
nats-agent → OTLP (gRPC:4317) → OpenTelemetry Collector → Prometheus (OTLP HTTP receiver)
producer   → OTLP (gRPC:4317) ↗
```

**Key Metrics to Monitor:**
- `messages_duration_count{status="success"}` - Total successful messages
- `messages_duration_count{status="failed"}` - Total failed messages
- `messages_duration_sum / messages_duration_count` - Average processing time
- `messages_duration_bucket` - Latency distribution (histogram buckets)

**Accessing Monitoring:**
```bash
# Prometheus UI (when Tilt is running)
open http://localhost:9090

# Example PromQL queries:
# Success rate: rate(messages_duration_count{status="success"}[5m])
# Error rate: rate(messages_duration_count{status="failed"}[5m])
# P95 latency: histogram_quantile(0.95, rate(messages_duration_bucket[5m]))
```

## Configuration via Environment Variables

All configuration is via environment variables (see `src/config.rs`):

| Variable | Default | Description |
|----------|---------|-------------|
| `NATS_URL` | `nats://localhost:4222` | NATS server URL |
| `NATS_STREAM_NAME` | `events` | JetStream stream name |
| `NATS_SUBJECTS` | `events.>` | Subject filter (supports wildcards) |
| `NATS_CONSUMER_NAME` | `nats-agent` | Durable consumer name |
| `BATCH_SIZE` | `10` | Messages to fetch per batch |
| `HTTP_PORT` | `8080` | HTTP server port |
| `MESSAGE_HANDLER_CMD` | (required) | Command to execute for each message |
| `OTEL_SERVICE_NAME` | `nats-agent` | OpenTelemetry service name |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | (optional) | OTLP endpoint (e.g., `http://localhost:4317`) |

## Binaries

This project produces two binaries:

1. **`nats-agent`** (`src/main.rs`) - The main consumer agent
   - Runs HTTP server + NATS message processor
   - Delegates message processing to external handler
   - Exports OpenTelemetry metrics

2. **`producer`** (`src/bin/producer.rs`) - Test event generator
   - Publishes events to NATS subjects
   - Supports custom payloads and event types
   - Exports OpenTelemetry metrics for published events

## Common Issues

**Container image reference in Kubernetes:**
- Tilt uses `image_keys=[('image.registry', 'image.repository', 'image.tag')]`
- Helm template helper in `_helpers.tpl` constructs full image from separate fields
- Image format: `reg.wheat-dn42.net/poc-nats-agent:latest`

**Handler script not executing:**
- Ensure `MESSAGE_HANDLER_CMD` environment variable is set
- Verify script is executable and has proper shebang (`#!/bin/bash`)
- Check container includes necessary tools (bash, jq, awk) in `flake.nix`

**Metrics not appearing in Prometheus:**
- Verify OpenTelemetry collector is running: `kubectl get pods -n otel-system`
- Check OTLP endpoint configuration matches collector service
- Ensure Prometheus has OTLP receiver enabled: `prometheus.prometheusSpec.enableOTLPReceiver=true`
- Verify collector exports to Prometheus: check `otlphttp` exporter in `k8s/otel-collector.yaml`

**`minikube start` fails on NixOS (kernel 6.18+):**

Reports `no such container "minikube"`, but the real error is `dockerd` failing inside
kicbase: `iptables (legacy): can't initialize iptables table 'nat'`. Kernel 6.18 dropped
legacy xtables; the kicbase entrypoint picks `legacy` on a 0-vs-0 rule-count tie.

Root-level `Containerfile` patches kicbase to use nft. It shadows this repo's default
build file — use an explicit `-f` for other images.

```bash
sudo podman build -f Containerfile -t localhost/kicbase-nftfix:v0.0.50 .
minikube start --driver=podman --base-image=localhost/kicbase-nftfix:v0.0.50
```

`--base-image` must be passed every start (`minikube config set base-image` is unsupported).
If a start left things wedged, `minikube delete` **and** `sudo podman volume rm minikube`.
