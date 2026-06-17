# Thalamus

Event-driven cognitive coordination protocol and runtime substrate for AI-native execution environments.

---

## Overview

Thalamus is a distributed cognitive runtime architecture for coordinating sandboxed runtime workers through event-driven communication.

It is **not**:

* an agent framework
* a workflow engine
* an orchestration framework
* a prompt chaining system

Instead, Thalamus focuses on:

* distributed cognition
* runtime coordination
* event topology
* capability virtualization
* disposable runtime workers
* AI-native execution environments

Thalamus treats AI systems as a distributed nervous system rather than a collection of directly connected services.

---

## Core Concept

Modern AI systems often tightly couple:

* agents
* tools
* memory
* execution
* orchestration
* provider APIs

This creates rigid systems that are difficult to:

* scale
* isolate
* distribute
* replay
* observe
* evolve

Thalamus separates these concerns into independent runtime layers.

---

## Architecture

```text
Human
 ↓
AI Agent
 ↓
Cingulater
(Shared Cognition Plane)
 ↓
Thalamus Runtime
(Event-driven Cognitive Coordination Layer)
 ↓
Rust Runtime / Python Runtime
 ↓
Sandboxed Runtime Workers
 ↓
mcp-routing-gateway
(Capability Plane)
 ↓
MCP Servers
```

---

## Design Philosophy

### Event-driven First

Thalamus is built around events, not synchronous orchestration.

Workers communicate through cognitive signals propagated over an event fabric.

The system prioritizes:

* loose coupling
* asynchronous execution
* distributed coordination
* emergent behavior
* disposable workers

---

### Runtime Workers Are Disposable

Runtime workers are temporary execution units.

They are expected to:

* spawn dynamically
* process cognitive tasks
* emit events
* terminate cleanly

Workers are intentionally lightweight and runtime-managed.

No persistent ownership assumptions are made.

---

### Runtime Owns Capabilities

Workers do not own tools.

Capabilities belong to the runtime.

Workers borrow capabilities through the runtime layer.

```python
runtime.call_tool(...)
```

Internally this may route through:

* MCP
* capability brokers
* policy layers
* permission scopes
* sandbox restrictions

---

### Shared Cognition Plane

Workers do not directly connect to LLM providers.

All cognition flows through a shared cognition layer.

```text
Runtime Worker
 ↓
Thalamus Runtime
 ↓
Cingulater
 ↓
LLM Provider
```

This enables:

* centralized policy control
* model routing
* context compression
* unified inference management
* cost optimization

---

### Cognitive Event Fabric

Thalamus uses an event fabric (currently planned around NATS) as its nervous system.

Events represent:

* state transitions
* lifecycle updates
* cognitive signals
* capability requests
* task coordination

The runtime is centered around signal propagation rather than direct endpoint invocation.

---

## Communication Model

Thalamus separates runtime concerns into independent planes.

| Plane            | Responsibility                  |
| ---------------- | ------------------------------- |
| Cognition Plane  | Shared LLM inference            |
| Capability Plane | Tool access and routing         |
| Event Plane      | Cognitive coordination          |
| Data Plane       | Contexts, artifacts, workspaces |
| Execution Plane  | Sandboxed runtime execution     |

---

## Why Event-driven Instead of REST?

REST models systems as direct endpoint-to-endpoint communication.

Thalamus models systems as cognitive signal propagation.

REST:

```text
A → B
```

Thalamus:

```text
signal → propagation → resonance → reaction
```

The runtime is designed more like a distributed nervous system than a traditional service mesh.

---

## Runtime Workers

Runtime workers are not autonomous AI personalities.

They are disposable cognitive execution units operating inside sandboxed environments.

Workers:

* receive events
* perform inference
* borrow capabilities
* emit new events

The intelligence of the system emerges from runtime coordination and event propagation rather than persistent agent identities.

---

## Planned Runtime Components

```text
thalamus/
├─ protocol/
├─ schemas/
├─ runtime/        # Python reference implementation
├─ rust/           # Rust Runtime MVP workspace
├─ sdk/
├─ supervisor/
├─ docs/
└─ examples/
```

---

## Directory Overview

### protocol/

Core runtime contracts and specifications.

Includes:

* event model
* lifecycle model
* capability model
* subject naming
* runtime contracts

---

### schemas/

JSON schemas for runtime events and protocol structures.

---

### runtime/

Core runtime coordination layer.

Includes:

* NATS integration
* event dispatch
* session coordination
* runtime APIs

---

### sdk/

Runtime worker SDKs.

Initial target:

* Python SDK

Example API:

```python
runtime.publish()
runtime.subscribe()
runtime.ask_llm()
runtime.call_tool()
```

---

### supervisor/

Sandbox lifecycle management.

Responsible for:

* spawning containers
* mounting workspaces
* monitoring execution
* terminating sandboxes

---

### docs/

Architecture notes, RFCs, and protocol discussions.

---

### examples/

Reference implementations and minimal runtime examples.

---

## Initial Goals

The first milestone is intentionally minimal.

Goal:

```text
task publish
 ↓
runtime worker receives task
 ↓
shared LLM inference
 ↓
result publish
```

Before building advanced cognitive systems, Thalamus focuses on defining:

* runtime contracts
* event topology
* lifecycle semantics
* capability boundaries

---

## Design Principles

* Protocol before implementation
* Event-driven first
* Distributed by default
* Runtime workers are disposable
* Runtime owns capabilities
* Shared cognition plane
* Capability virtualization
* Loose coupling
* Sandbox isolation

---

## Long-term Vision

Thalamus aims to become an AI-native runtime substrate for distributed cognition systems.

The project explores:

* cognitive coordination
* distributed AI execution
* runtime-managed capabilities
* recursive worker spawning
* event-native AI systems
* AI operating substrate architectures

---
## Status
- **Rust Runtime MVP**: ✅ Completed
  - Implemented core runtime semantics (Task assignment, Result handling, Tool/LLM request/response).
  - Added `scope` and `refs` to `EventEnvelope`.
  - Worker registry and state management implemented.
  - Default handlers and publish semantics verified.
  - CLI `RunDemo` functional.
  - Tests: 46 pass, Clippy pass, Fmt pass.

Prototype implementation phase.

Implemented today:

* event validation and publication for `runtime.task.assign`, `runtime.task.result`, `runtime.agent.ready`, `runtime.agent.exit`
* NATS-based event bus adapter
* disposable worker spawn flow through supervisor
* direct task addressing via `runtime.task.assign.<agent_id>`
* minimal sandbox shell worker capability (`tool.shell`) with timeout, stdout/stderr, and exit-code reporting
* minimal reference implemented runtime subjects for LLM and tool mediation (`runtime.llm.request` / `runtime.llm.response`, `runtime.tool.request` / `runtime.tool.result`)
* Rust Runtime MVP is implemented in [`rust/`](rust/): protocol envelope and payload contracts, in-memory bus behavior, runtime lifecycle, worker/task state tracking, bus-mediated deterministic LLM/tool result handlers, CLI parsing, and deterministic local demo command coverage.
* Rust protocol exposes [`EventEnvelope`](rust/protocol/src/message.rs:5) with canonical fields plus optional `scope` and `refs`, constructs envelopes through [`EventEnvelopeFields`](rust/protocol/src/message.rs:26), and defines MVP runtime payloads such as [`RuntimeLLMRequestPayload`](rust/protocol/src/payload.rs:75) and [`RuntimeToolRequestPayload`](rust/protocol/src/payload.rs:50) with payload-level `correlation_id` fields.
* Rust bus exposes [`BasicBus`](rust/bus/src/lib.rs:84) with publish delivery, closed-bus errors, no-subscriber success, and an observation snapshot via [`published_events()`](rust/bus/src/lib.rs:101).
* Rust runtime exposes [`TaskState`](rust/runtime/src/lib.rs:66), [`WorkerRegistry`](rust/runtime/src/lib.rs:113), [`MockLlmProvider`](rust/runtime/src/lib.rs:145), [`EchoTool`](rust/runtime/src/lib.rs:178), default MVP subject registration, LLM request input from `prompt` or the last `messages` content, payload-level `correlation_id` preservation, local agent/task state updates, and bus-mediated publication of [`runtime.llm.response`](rust/protocol/src/subject.rs:12) and [`runtime.tool.result`](rust/protocol/src/subject.rs:14).
* [`ThalamusRuntime::publish()`](rust/runtime/src/lib.rs:530) first records accepted events through the bus, then applies default runtime behavior for agent ready/exit/error, task assign/result, LLM request, and tool request subjects; accepted events remain observable even when there are no external subscribers.
* Rust CLI includes [`RunDemo`](rust/cli/src/lib.rs:37), which starts a local runtime, publishes LLM/tool request payloads through the [`BasicBus`](rust/bus/src/lib.rs:84) path, and prints `Runtime Event Flow`, `Mock response: summarize runtime MVP`, and the echo tool JSON outcome.
* Rust workspace verification commands: `cd rust && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`.
* Rust SDK FFI subscription rejects a NULL callback with `-1`; callers must pass a valid callback function pointer to `thalamus_subscribe()`. The callback payload pointer is NUL-terminated, non-null, and valid only for the duration of the callback invocation.

Not implemented yet (still design-level in docs):

* full shared cognition plane integration beyond the minimal reference runtime path
* full capability-plane mediation through external gateway integrations such as MCP
* complete multi-plane architecture semantics beyond the current minimal executable subset
