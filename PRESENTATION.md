---
title: An In-Product Event Stream
sub_title: A NATS JetStream proof-of-concept
author: Pete Erickson <pete@opaque.co>
---

# The pitch

**Every meaningful action in the product emits an event.**

<!-- pause -->

Not a log line. Not a metric. A durable, replayable, ordered record
that something happened.

<!-- pause -->

Once that stream exists, the interesting part is what you *hang off* it:

- Someone wants a Slack alert when a workflow fails
- Someone wants every event archived for compliance
- Someone wants it in Snowflake by morning
- Someone wants a real-time dashboard

<!-- pause -->

Today each of those is a **bespoke integration** into the app.
With a stream, each is **a consumer** — and the app never changes again.

<!-- end_slide -->

# What I actually built

```
producer  ──▶  NATS JetStream  ──▶  nats-agent  ──▶  handler program
                                         │
                                         └──▶  OTel Collector ──▶ Prometheus
```

<!-- pause -->

`nats-agent` is deliberately **boring**. It:

1. Pulls a batch of messages off a durable consumer
2. Writes the event body to a child process on **stdin**
3. Acks on exit code `0`, records a failure otherwise

<!-- pause -->

That's it. It has no idea what your events mean.

<!-- end_slide -->

# The handler contract

The entire integration surface:

| Channel | Carries |
|---|---|
| **stdin** | the event body, as JSON |
| `NATS_SUBJECT` | *what kind* of event this is |
| `NATS_MSG_ID` | the event id |
| **exit code** | `0` = ack, non-zero = failure |

<!-- pause -->

That is a contract that **any** language satisfies. Bash. Go. Python.
Rust. A binary someone compiled in 2009.

<!-- end_slide -->

# A consumer, in its entirety

```bash +line_numbers
#!/bin/bash
set -euo pipefail

EVENT=$(cat)
SUBJECT="${NATS_SUBJECT:-<none>}"

case "$SUBJECT" in
    *.workflow.failed)
        curl -XPOST "$SLACK_WEBHOOK" \
             -d "{\"text\":\"workflow failed: $EVENT\"}"
        ;;
    *) exit 0 ;;
esac
```

<!-- pause -->

**This is a production consumer.** No SDK, no framework, no redeploy of
the app that emits the event.

<!-- end_slide -->

# Subject as the contract

Don't duplicate routing information into the payload — it just gives
the two a way to drift apart.

<!-- pause -->

The subject is already the routing key, already indexed, already
wildcard-matchable:

```
events.workflow.executed
events.workflow.failed
events.>                    ← the agent's filter
```

<!-- pause -->

One source of truth. The body carries **payload only**.

<!-- end_slide -->

# The consumers I did *not* build

This POC has one consumer. A real deployment has a **fleet**, and most
of them are infrastructure, not product logic:

<!-- pause -->

**Archival** — every event to blob storage. Cheap, immutable,
replayable. The thing you want when someone asks "what happened on
March 3rd?"

<!-- pause -->

**Warehouse loading** — batch into Parquet, land in Databricks or
Snowflake. Analysts get product events without querying prod.

<!-- pause -->

**Real-time query** — ClickHouse for sub-second aggregate queries.
Dashboards that don't hammer the application database.

<!-- end_slide -->

# Why this matters

Each of those consumers is **independent**:

- Different teams own them
- Different reliability requirements
- One can be down without the others noticing
- Add a new one without touching the producer

<!-- pause -->

JetStream keeps its own cursor per consumer. A consumer that was offline
for six hours catches up. **That's the whole value proposition.**

<!-- end_slide -->

# Observability is a different signal

Both the producer and consumer emit OpenTelemetry metrics:

```
events_processed_duration_seconds{status,subject}
events_published_duration_seconds{status,subject}
```

<!-- pause -->

Grafana dashboard, provisioned from JSON in the repo — totals,
throughput by subject, latency percentiles.

<!-- pause -->

**But I want to be precise about what these are.**

<!-- end_slide -->

# Metrics are not events

| | Event stream | OTel metrics |
|---|---|---|
| Shape | your domain | counters, histograms |
| Delivery | at-least-once, acked | best effort, lossy |
| Replay | yes, from any point | no |
| If it drops | incident | a gap in a graph |

<!-- pause -->

Metrics answer *"is the system healthy?"*

The stream answers *"what happened, exactly, and can you prove it?"*

<!-- pause -->

**Do not build business logic on your metrics pipeline.**
It has no delivery guarantee and it is pre-aggregated by design.

<!-- end_slide -->

<!-- jump_to_middle -->

# The Tilt experiment

<!-- end_slide -->

# The question

> Is `tilt` a good way to manage a local dev environment?

<!-- pause -->

It promises a lot: watch source, rebuild images, redeploy to a local
cluster, port-forward everything, one dashboard.

<!-- pause -->

## My honest answer: I'm not a fan.

<!-- end_slide -->

# It is magical, and that's the problem

Tilt's abstractions hide Kubernetes and Helm behind a Starlark DSL.
When they work, it's great. When they don't, **the error surfaces
somewhere unrelated to the cause.**

<!-- pause -->

What that cost me, concretely:

- CRDs hit the 256KB annotation limit — because Tilt applied them
  itself instead of letting Helm do it
- An operator crash-looped: its CRDs were **silently skipped**
- `401`s everywhere — Tilt recreated ServiceAccounts, invalidating
  running pods' tokens. Looks nothing like the actual cause
- A port forward attached to the wrong pod. Port open, every client
  got EOF

<!-- pause -->

None of these say *"Tilt did this."* That's the tax.

<!-- end_slide -->

# The fix was to stop using the magic

Every chart moved to `helm_resource` — which just runs
`helm upgrade --install`.

<!-- pause -->

```python
helm_resource(
    name='cert-manager',
    chart='jetstack/cert-manager',
    flags=['--set', 'crds.enabled=true'],
)
```

<!-- pause -->

Helm owns its own CRDs. Objects update in place. UIDs stay stable.

<!-- pause -->

**The tool got useful once I stopped letting it be clever.**

<!-- end_slide -->

# Tilt: the balanced view

**Genuinely good:**

- One command brings up the whole stack
- File-watch → rebuild → redeploy is a real loop
- Port-forward management is convenient
- Dependency ordering between services

<!-- pause -->

**The cost:**

- Failures are indirect and hard to attribute
- Its Helm/K8s abstractions leak badly
- Extensions vary wildly in quality
- Debugging requires knowing what it does underneath — at which point,
  why the abstraction?

<!-- pause -->

For a POC: fine. For a team's daily driver: **I'd want something more
boring.**

<!-- end_slide -->

# Takeaways

**The event stream idea holds up.** A generic consumer plus the
stdin/exit-code contract means new integrations are shell scripts, not
projects.

<!-- pause -->

**Let the subject be the contract.** Don't duplicate routing
information into the payload.

<!-- pause -->

**Keep events and metrics separate.** Different shapes, different
guarantees. Don't build logic on the lossy one.

<!-- pause -->

**Tilt is a sharp tool held by the blade.** Useful, but I would not
standardize a team on it.

<!-- end_slide -->

<!-- jump_to_middle -->

# Questions?
