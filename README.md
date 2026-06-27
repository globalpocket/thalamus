# Thalamus

Event-driven cognitive coordination protocol and runtime substrate for AI-native execution environments.

[![Rust CI](https://github.com/globalpocket/thalamus/actions/workflows/rust.yml/badge.svg)](https://github.com/globalpocket/thalamus/actions/workflows/rust.yml)

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

Prototype implementation phase.

### Rust Runtime MVP

- **Rust Runtime MVP**: ✅ Completed
  - Event-driven internal handler architecture
  - `Runtime.publish()` as pure event entrance
  - LlmProvider injection with `set_llm_provider()`
  - ToolRegistry with `tool.echo` canonical capability
  - User handler / internal handler separation
  - Exact-once provider/tool invocation semantics
  - Provider/tool error handling via event publication

Implemented today:

* event validation and normalization for all canonical subjects via [`validate_and_normalize_payload()`](rust/protocol/src/validation.rs:48)
* `runtime.agent.spawn` is observation-only in MVP
* `runtime.agent.ready` / `runtime.agent.exit` / `runtime.agent.error` update [`WorkerRegistry`](rust/runtime/src/registry.rs:68)
* `runtime.task.assign` / `runtime.task.result` update [`TaskState`](rust/runtime/src/state.rs:75)
* `runtime.llm.request` calls injected provider exactly once, publishes [`runtime.llm.response`](rust/protocol/src/subject.rs:16)
* `runtime.tool.request` invokes tool exactly once, publishes [`runtime.tool.result`](rust/protocol/src/subject.rs:14)
* Provider errors produce `runtime.llm.response` with `status="error"`
* Unknown tool errors produce `runtime.tool.result` with `status="error"`
* Invalid canonical payloads are NOT recorded by the bus
* Unknown extension subjects ARE published
* `request_id` is auto-completed for `runtime.llm.request` and `runtime.tool.request`
* [`BasicBus`](rust/bus/src/lib.rs:84) implements `Clone + Send + Sync + 'static` with `&self` methods
* [`NatsBus`](rust/bus/src/nats.rs) is an optional distributed transport backend behind `nats` feature (MVP at-most-once, no JetStream)
* [`RuntimeCore`](rust/runtime/src/runtime.rs:35) holds shared state for internal handlers
* [`ThalamusRuntime::publish()`](rust/runtime/src/runtime.rs:431) is the sole event entrance
* CLI [`RunDemo`](rust/cli/src/lib.rs:203) observes event flow via `observer.published_events()`
* Rust SDK FFI subscription rejects a NULL callback with `-1`; callers must pass a valid callback function pointer to `thalamus_subscribe()`. The callback payload pointer is NUL-terminated, non-null, and valid only for the duration of the callback invocation.

Verification commands:

```bash
cd rust
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Not implemented yet (still design-level in docs):

* full shared cognition plane integration beyond the minimal reference runtime path
* full capability-plane mediation through external gateway integrations such as MCP
* complete multi-plane architecture semantics beyond the current minimal executable subset
