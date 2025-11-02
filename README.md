# NATS Agent POC

Learning project for Rust, NATS.io messaging, and AI agents.

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
