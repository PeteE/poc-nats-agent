# NATS Proof-of-Concept
This project is a POC for how an event stream could be built using `nats.io` + Rust-based producers and consumers.  Additionally, I wanted to experiment with using `tilt` (`Tiltfile`) for developer experience, and Clickhouse for event storage.

Overall my goal was to show my colleagues how an event stream could be implemented and how it could provide value to our company as well as customers.

# commands
```
# start the consumer with the echo handler
NATS_URL=nats://192.168.128.13:4222 \
    NATS_SUBJECTS="workflows.>" \
    NATS_STREAM_NAME=events \
    BATCH_SIZE=1 \
    OTEL_EXPORTER_OTLP_ENDPOINT=http://10.97.25.182:4317 \
    MESSAGE_HANDLER_CMD=/home/petee/dev/poc-nats-agent/k8s/nats-agent/files/echo.sh \
    cargo run --bin nats-agent
```

# NOTES
Creating a stream
```
nats stream add workflows --subjects 'workflows.*' --description 'Stream containg all workflow information' --storage file --ack --retention limits --discard old 
```
Creating a durable consumer for workflow created messages
```
nats consumer create workflows wf-created --filter=workflows.created --description='Workflow Created Events' --ack=explicit --pull --deliver=all --max-deliver=-1 --sample=-1 --replay=instant  --no-headers-only 
```

## What This Is

A generic NATS JetStream consumer, written in Rust, that delegates the actual
work to an external handler program. The consumer pulls messages in batches,
hands each event body to the handler on stdin, and acknowledges based on the
handler's exit code. The handler can be any executable -- a shell script, a
Python script, a compiled binary.

**Learning Goals:**
- Rust async programming (Tokio)
- NATS.io messaging patterns (pub/sub, queues, JetStream)
- The external-handler pattern: keeping the consumer generic and pushing
  business logic out into separate programs

## Architecture

```
producer → NATS JetStream → nats-agent → external handler program
                                ↓
                     OpenTelemetry Collector → Prometheus
```

**Possible future direction:** because the handler is just a program that
reads JSON on stdin, one could be written that calls an LLM and takes some
action based on the response. Nothing in this repo does that today -- the
consumer has no notion of models, prompts, or tools.

## Tech Stack

- **Rust** - Consumer implementation
- **NATS.io** - Message broker
- **Tilt.dev** - Kubernetes development workflow
- **Minikube** - Local k8s cluster

## Quick Start

```bash
# Start NATS and development environment
tilt up

# Build and run the consumer locally
cargo build
cargo run

# Run tests
cargo test
```

## Project Structure

```
.
├── src/              # Rust source
├── k8s/              # Kubernetes manifests
├── Tiltfile          # Tilt configuration
├── Cargo.toml        # Rust dependencies
└── README.md
```

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [NATS Docs](https://docs.nats.io/)
- [async-nats client](https://github.com/nats-io/nats.rs)
- [Tilt.dev](https://tilt.dev/)

## Author

Pete Erickson (pete@opaque.co)
