# Event Model

## Overview

Thalamus uses an event-native runtime architecture.

All runtime coordination occurs through structured cognitive events propagated over the event fabric.

Events represent:

* state transitions
* cognitive signals
* coordination requests
* lifecycle updates
* capability operations
* inference operations

The event model defines the canonical structure of runtime communication.

---

# Relationship to Subject Naming

Each event is published to a subject defined by the subject naming specification.

Example:

```text id="3r7n8c"
runtime.task.assign
```

The subject describes:

```text id="o6a7ps"
"What kind of signal is propagating"
```

The event payload describes:

```text id="2y66rd"
"The runtime state associated with that signal"
```

---

# Event Design Principles

## 1. Events Represent State Transitions

Events describe runtime state changes.

They should communicate:

* what happened
* what changed
* what requires reaction

Events should not model direct procedural invocation.

---

## 2. Events Are Immutable

Once emitted, events must not be modified.

Corrections or updates must be represented as new events.

---

## 3. Events Are Transport-agnostic

The event model is independent of:

* NATS
* Kafka
* Redis Streams
* internal brokers

The transport layer is an implementation detail.

---

## 4. Events Are Lightweight

Events should contain:

* identifiers
* metadata
* references
* coordination signals

Large runtime artifacts should not be embedded directly.

Examples of externalized data:

* repository contents
* prompts
* embeddings
* binary artifacts
* long inference transcripts

---

## 5. Events Prefer References Over Payload Size

The runtime favors reference-based coordination.

Correct:

```json id="4m1r5d"
{
  "refs": {
    "workspace": "workspace://task-123",
    "context": "context://session-9"
  }
}
```

Avoid:

```json id="az15lh"
{
  "prompt": "very large prompt ..."
}
```

---

# Canonical Event Envelope

All runtime events MUST follow the canonical event envelope.

---

## Event Structure

```json id="0g9o3q"
{
  "id": "evt_01HX8K...",
  "type": "runtime.task.assign",
  "timestamp": "2026-05-10T12:00:00Z",
  "source": "runtime.supervisor",
  "scope": {
    "sandbox": "sb-123",
    "worker": "wk-9",
    "task": "task-456",
    "session": "sess-abc"
  },
  "refs": {
    "workspace": "workspace://task-456",
    "context": "context://sess-abc",
    "artifact": "artifact://result-1"
  },
  "payload": {}
}
```

---

# Event Fields

| Field     | Required | Description                              |
| --------- | -------- | ---------------------------------------- |
| id        | yes      | Globally unique event identifier         |
| type      | yes      | Subject-compatible event type            |
| timestamp | yes      | Event creation timestamp (UTC ISO8601)   |
| source    | yes      | Runtime component that emitted the event |
| scope     | optional | Runtime execution scope                  |
| refs      | optional | External runtime references              |
| payload   | optional | Event-specific structured payload        |

---

# Field Definitions

## id

Globally unique event identifier.

Requirements:

* unique across the runtime
* immutable
* stable for replay systems

Example:

```text id="u7lh6q"
evt_01HX8K2R4P7A...
```

---

## type

The canonical event type.

Must match the published runtime subject.

Example:

```text id="nh2c7k"
runtime.task.assign
```

---

## timestamp

UTC ISO8601 event creation time.

Example:

```text id="8e6f8j"
2026-05-10T12:00:00Z
```

---

## source

Identifies the runtime component that emitted the event.

Examples:

```text id="j8jzz8"
runtime.supervisor
runtime.worker.wk-9
runtime.gateway
runtime.scheduler
```

---

## scope

Defines runtime locality and execution boundaries.

Example:

```json id="lv6ew0"
{
  "sandbox": "sb-123",
  "worker": "wk-9",
  "task": "task-456"
}
```

Possible scope fields:

| Field    | Purpose                      |
| -------- | ---------------------------- |
| sandbox  | Sandbox execution boundary   |
| worker   | Runtime worker identifier    |
| task     | Task coordination identifier |
| session  | Shared cognition session     |
| workflow | Optional workflow grouping   |

---

## refs

References to external runtime data.

Refs separate:

* coordination
* storage
* cognition
* artifacts

from the event transport itself.

Example:

```json id="qduhaj"
{
  "workspace": "workspace://task-456",
  "context": "context://sess-abc",
  "artifact": "artifact://result-1"
}
```

---

### Common Reference Types

| Ref Type  | Purpose                     |
| --------- | --------------------------- |
| workspace | Mounted sandbox workspace   |
| context   | Shared cognition state      |
| artifact  | Generated outputs           |
| memory    | Runtime memory store        |
| stream    | Streaming inference channel |

---

## payload

Structured event-specific data.

Payloads should remain:

* compact
* semantic
* structured
* transport-friendly

Example:

```json id="9y93kk"
{
  "objective": "Fix authentication bug",
  "priority": "high"
}
```

---

# Event Categories

---

## Runtime Events

Examples:

```text id="5sm0k6"
runtime.agent.spawn
runtime.agent.exit
runtime.agent.error
```

Purpose:

* lifecycle coordination
* worker supervision
* runtime monitoring

---

## Task Events

Examples:

```text id="2grv1s"
runtime.task.assign
runtime.task.result
```

Purpose:

* distributed execution coordination
* cognitive workload propagation

---

## Tool Events

Examples:

```text id="16s10z"
runtime.tool.request
runtime.tool.result
```

Purpose:

* capability access
* runtime-mediated tool execution

---

## LLM Events

Examples:

```text id="d2wljm"
runtime.llm.request
runtime.llm.response
```

Purpose:

* shared cognition inference
* centralized LLM coordination

---

# Streaming Events

Streaming operations should emit lifecycle events rather than raw token floods.

Correct:

```text id="w8rzrz"
runtime.llm.stream.started
runtime.llm.stream.completed
```

Avoid:

```text id="d63xod"
one event per token
```

Large streaming payloads should use external stream references.

Example:

```json id="2phxiy"
{
  "refs": {
    "stream": "stream://llm/session-123"
  }
}
```

---

# Event Propagation Model

Events propagate through the runtime fabric.

Runtime workers may:

* react
* ignore
* transform
* amplify
* emit derived events

This creates emergent cognitive coordination behavior.

The runtime prioritizes:

```text id="4yw9ib"
signal → propagation → resonance → reaction
```

rather than direct procedural control.

---

# Observability

Because all coordination occurs through events, the runtime naturally supports:

* replay
* tracing
* debugging
* distributed monitoring
* execution reconstruction
* runtime analytics

The event fabric becomes the observable nervous system of the runtime.

---

# Philosophy

Thalamus does not model cognition as direct API invocation.

It models cognition as structured signal propagation through a distributed runtime substrate.

Events are not merely messages.

They are cognitive state transitions flowing through the runtime nervous system.
