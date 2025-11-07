# NATS Agent POC

Learning project for Rust, NATS.io messaging, and AI agents.

# NOTES
Creating a stream
```
nats stream add workflows --subjects 'workflows.*' --description 'Stream containg all workflow information' --storage file --ack --retention limits --discard old 
```
Creating a durable consumer for workflow created messages
```
nats consumer create workflows wf-created --filter=workflows.created --description='Workflow Created Events' --ack=explicit --pull --deliver=all --max-deliver=-1 --sample=-1 --replay=instant  --no-headers-only 
```
Creating a durable consumer for workflow created messages
```
nats consumer create workflows wf-created --filter=workflows.created --description='Workflow Created Events' --ack=explicit --pull --deliver=all --max-deliver=-1 --sample=-1 --replay=instant  --no-headers-only 
`


## What This Is

Rust-based AI agents that consume messages from NATS queues, process them (with LLM calls), and produce results.

**Learning Goals:**
- Rust async programming (Tokio)
- NATS.io messaging patterns (pub/sub, queues, JetStream)
- AI agent architecture

## Architecture

```
Minikube → NATS Server → Rust Agent(s) → LLM APIs
```

## Tech Stack

- **Rust** - Agent implementation
- **NATS.io** - Message broker
- **Tilt.dev** - Kubernetes development workflow
- **Minikube** - Local k8s cluster

## Quick Start

```bash
# Start NATS and development environment
tilt up

# Build and run agent locally
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
