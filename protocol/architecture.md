# Runtime Architecture

## Overview

Thalamus is a distributed cognitive runtime architecture designed for AI-native execution environments.

The architecture separates cognition, coordination, execution, and capability access into independent runtime planes connected through an event-driven fabric.

Thalamus models AI execution as a distributed nervous system rather than a collection of tightly coupled services.

---

# Architectural Philosophy

Traditional systems typically model execution as:

```text id="4m7k1v"
service → API → response
```

Thalamus instead models execution as:

```text id="7m2n5r"
signal → propagation → resonance → reaction
```

The runtime is fundamentally event-native.

Workers are disposable cognitive execution structures participating in a shared coordination fabric.

---

# Architectural Layers

Thalamus separates the runtime into distinct operational planes.

---

## High-level Architecture

```text id="6m1k8v"
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
Sandboxed Runtime Workers
 ↓
mcp-routing-gateway
(Capability Plane)
 ↓
MCP Servers / External Systems
```

---

# Runtime Planes

---

## 1. Cognition Plane

### Responsibility

Shared inference coordination.

### Components

Examples:

* Cingulater
* OpenAI-compatible inference endpoints
* embedding systems
* reranking systems
* reasoning models

### Purpose

Workers do not directly communicate with LLM providers.

All cognition flows through a shared cognition layer.

This enables:

* centralized model routing
* context optimization
* policy enforcement
* cost control
* unified observability

### Example Flow

```text id="2m8k4v"
Runtime Worker
 ↓
runtime.llm.request
 ↓
Cingulater
 ↓
LLM Provider
 ↓
runtime.llm.response
```

---

## 2. Event Plane

### Responsibility

Distributed cognitive coordination.

### Primary Technology

Planned:

* NATS

### Purpose

The event plane acts as the runtime nervous system.

It propagates:

* lifecycle transitions
* cognitive signals
* coordination events
* capability requests
* execution results

### Event Examples

```text id="8m1n5r"
runtime.task.assign
runtime.tool.request
runtime.agent.spawn
runtime.llm.request
```

### Characteristics

* asynchronous
* distributed
* loosely coupled
* observable
* replay-friendly

---

## 3. Execution Plane

### Responsibility

Sandboxed runtime execution.

### Components

Examples:

* containers
* microVMs
* isolated sandboxes
* ephemeral runtime workers

### Purpose

Workers execute runtime tasks inside isolated environments.

Workers are:

* disposable
* lightweight
* runtime-managed
* event-driven

Workers are not persistent intelligent entities.

---

## 4. Capability Plane

### Responsibility

Runtime-mediated capability access.

### Components

Examples:

* mcp-routing-gateway
* MCP servers
* runtime policy layers
* capability brokers

### Purpose

Workers never directly own tools.

Capabilities are leased and controlled by the runtime.

The capability plane provides:

* isolation
* observability
* policy enforcement
* abstraction
* routing

### Example Flow

```text id="5m9n2v"
Runtime Worker
 ↓
runtime.tool.request
 ↓
Thalamus Runtime
 ↓
mcp-routing-gateway
 ↓
MCP Server
 ↓
runtime.tool.result
```

---

## 5. Data Plane

### Responsibility

Persistent runtime state and artifacts.

### Examples

* workspaces
* cognition contexts
* execution artifacts
* memory stores
* replay logs
* event archives

### Purpose

The event plane coordinates runtime behavior.

The data plane stores durable runtime state.

Large payloads should remain externalized from runtime events.

---

# Runtime Components

---

## RooCode

Acts as the primary human-facing interaction environment.

Responsibilities:

* human interaction
* task initiation
* runtime observation
* developer workflows

RooCode is not part of the runtime core itself.

---

## Cingulater

Acts as the shared cognition gateway.

Responsibilities:

* LLM routing
* inference abstraction
* OpenAI-compatible API exposure
* cognition coordination

Cingulater centralizes runtime cognition.

---

## Thalamus Runtime

Acts as the cognitive coordination substrate.

Responsibilities:

* event propagation
* runtime supervision
* lifecycle coordination
* capability mediation
* worker orchestration
* distributed coordination

The runtime acts as the nervous system of the architecture.

---

## Runtime Workers

Temporary sandboxed execution structures.

Responsibilities:

* process cognitive tasks
* emit runtime events
* request capabilities
* interact with cognition systems

Workers are intentionally disposable.

---

## mcp-routing-gateway

Acts as the capability routing layer.

Responsibilities:

* MCP routing
* capability virtualization
* policy enforcement
* tool federation

The gateway decouples workers from physical tool topology.

---

## MCP Servers

Provide executable runtime capabilities.

Examples:

* filesystem operations
* git operations
* web access
* shell execution
* external integrations

MCP remains an implementation layer within the capability plane.

---

# Event-native Coordination

All runtime coordination occurs through events.

Examples:

```text id="3m7n1v"
runtime.task.assign
runtime.tool.request
runtime.llm.request
runtime.agent.error
```

The runtime avoids direct synchronous orchestration whenever possible.

This enables:

* loose coupling
* recursive execution
* distributed replay
* emergent coordination
* dynamic scaling

---

# Worker Lifecycle Integration

The architecture integrates directly with the lifecycle model.

Worker lifecycle transitions propagate through the event plane.

Examples:

```text id="7m1k5r"
CREATED
BOOTING
IDLE
RUNNING
WAITING
COMPLETED
FAILED
TERMINATED
```

Lifecycle visibility is a first-class runtime property.

---

# Capability Isolation

Capabilities remain isolated from runtime workers.

Workers never receive unrestricted infrastructure ownership.

The runtime enforces:

* sandbox boundaries
* workspace scope
* capability leases
* policy constraints
* permission isolation

This preserves disposability and runtime portability.

---

# Runtime Topology

The runtime topology is defined by:

* subjects
* event propagation
* subscriptions
* runtime scopes
* capability routes

The runtime behaves more like a nervous system than a traditional service mesh.

---

# Recursive Execution

The architecture supports recursive runtime activation.

Example:

```text id="9m2k7r"
worker
 ↓
task emission
 ↓
new worker activation
 ↓
distributed coordination
```

The runtime coordinates recursive execution through events rather than hierarchical ownership.

---

# Observability

Because coordination is event-native, the runtime naturally supports:

* replay
* tracing
* debugging
* distributed monitoring
* execution reconstruction
* cognitive analytics

The event fabric becomes the observable nervous system of the runtime.

---

# Minimal Runtime Goal

The initial runtime goal is intentionally minimal.

Target execution flow:

```text id="6m4k2v"
runtime.task.assign
 ↓
worker activation
 ↓
runtime.llm.request
 ↓
runtime.tool.request
 ↓
runtime.task.result
```

The priority is establishing stable runtime semantics before advanced orchestration features.

---

# Future Directions

Potential future extensions:

* distributed cognition graphs
* event replay systems
* runtime memory federation
* capability marketplaces
* multi-runtime federation
* cognitive load balancing
* distributed execution replay
* recursive cognition propagation

These remain outside the minimal runtime specification.

---

# Philosophy

Traditional agent systems focus on:

```text id="1m8k4v"
persistent intelligent entities
```

Thalamus focuses on:

```text id="4m2n8r"
distributed cognitive coordination
```

The intelligence is not located inside workers.

It emerges from the runtime event fabric itself.
