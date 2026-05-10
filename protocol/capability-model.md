# Capability Model

## Overview

Thalamus defines capabilities as runtime-managed operational permissions and execution interfaces.

Capabilities represent controlled access to external functionality such as:

* tools
* file systems
* network access
* memory systems
* MCP servers
* execution environments
* external APIs

Capabilities are not owned by runtime workers.

They are owned, mediated, and enforced by the runtime itself.

---

# Capability Philosophy

Thalamus separates:

* cognition
* coordination
* execution
* capability access

into independent runtime layers.

Workers are intentionally lightweight and disposable.

Persistent capability ownership would tightly couple workers to infrastructure and violate runtime isolation principles.

Instead:

```text id="2m7k4v"
Workers borrow capabilities from the runtime.
```

The runtime acts as the authoritative capability broker.

---

# Core Principles

---

## 1. Runtime Owns Capabilities

Workers never directly own tools or infrastructure access.

Capabilities are:

* leased
* scoped
* mediated
* observable
* revocable

by the runtime.

---

## 2. Capability Access Is Event-native

All capability interactions are represented as runtime events.

Examples:

```text id="8m1n5r"
runtime.tool.request
runtime.tool.result
```

Capability execution is coordinated through the event fabric.

---

## 3. Capabilities Are Scoped

Capabilities must always operate within explicit runtime boundaries.

Examples:

* workspace scope
* sandbox scope
* session scope
* policy scope
* time scope

No capability should be globally unrestricted by default.

---

## 4. Capability Execution Is Observable

All capability requests and results should be traceable through the runtime event system.

This enables:

* replay
* auditing
* debugging
* runtime analytics
* policy enforcement

---

## 5. Workers Are Capability Consumers

Workers consume capabilities temporarily.

They do not become infrastructure owners.

This preserves:

* disposability
* isolation
* runtime portability
* distributed coordination

---

# Capability Architecture

```text id="5m8k2v"
Runtime Worker
 ↓
Thalamus Runtime
 ↓
Capability Layer
 ↓
mcp-routing-gateway
 ↓
MCP Servers / External Systems
```

The worker never directly communicates with external systems.

The runtime mediates all capability interactions.

---

# Capability Categories

---

## Tool Capabilities

Examples:

* filesystem.read
* filesystem.write
* git.clone
* shell.execute
* web.fetch

These capabilities typically route through MCP infrastructure.

---

## Cognition Capabilities

Examples:

* llm.inference
* llm.embedding
* llm.rerank

These capabilities route through the shared cognition plane.

---

## Workspace Capabilities

Examples:

* workspace.mount
* workspace.snapshot
* workspace.persist

These capabilities manage runtime execution environments.

---

## Memory Capabilities

Examples:

* memory.read
* memory.write
* memory.search

These capabilities access runtime cognition state.

---

## Network Capabilities

Examples:

* network.http
* network.websocket
* network.dns

These are often highly restricted.

---

# Capability Lifecycle

Capabilities are leased dynamically during runtime execution.

Typical lifecycle:

```text id="7m2n4r"
request
 ↓
validation
 ↓
lease
 ↓
execution
 ↓
release
```

Workers should not retain persistent capability ownership beyond execution scope.

---

# Capability Request Flow

---

## Step 1 — Worker Emits Request

Example event:

```text id="1m9k4v"
runtime.tool.request
```

Example payload:

```json id="4m7k1v"
{
  "capability": "filesystem.read",
  "target": "/workspace/src/app.py"
}
```

---

## Step 2 — Runtime Validates Scope

The runtime evaluates:

* sandbox policy
* worker permissions
* execution scope
* runtime restrictions
* security rules

---

## Step 3 — Capability Lease Issued

If approved, the runtime grants temporary capability access.

---

## Step 4 — Capability Executes

Execution may route through:

* MCP servers
* internal runtime services
* external APIs
* cognition systems

---

## Step 5 — Result Event Emitted

Example:

```text id="9m3k7v"
runtime.tool.result
```

---

# Capability Lease Model

Capabilities are represented as temporary leases.

Example:

```json id="6m1k8v"
{
  "capability": "filesystem.read",
  "scope": {
    "workspace": "workspace://task-123"
  },
  "expires_at": "2026-05-10T12:05:00Z"
}
```

---

# Capability Scope

Capabilities should always be scope-aware.

---

## Workspace Scope

Restrict capability access to a specific mounted workspace.

Example:

```json id="2m4n8r"
{
  "workspace": "workspace://task-123"
}
```

---

## Sandbox Scope

Restrict capability access to a specific sandbox.

Example:

```json id="8m2k5v"
{
  "sandbox": "sb-123"
}
```

---

## Session Scope

Restrict capability access to a cognition session.

Example:

```json id="5m7n1r"
{
  "session": "sess-abc"
}
```

---

## Time Scope

Capabilities may automatically expire.

Example:

```json id="1m8k4v"
{
  "expires_at": "2026-05-10T12:05:00Z"
}
```

---

# Capability Events

---

## Request Events

```text id="7m1k5r"
runtime.tool.request
```

Represents a request for runtime-mediated capability execution.

---

## Result Events

```text id="3m8n2v"
runtime.tool.result
```

Represents a completed capability operation.

---

## Future Events

Possible future extensions:

```text id="4m9k1r"
runtime.tool.denied
runtime.tool.timeout
runtime.tool.revoked
runtime.tool.streaming
```

These are intentionally excluded from the minimal specification.

---

# Capability Isolation

Capabilities should remain isolated from workers whenever possible.

Workers should not receive:

* raw infrastructure credentials
* unrestricted file system access
* unrestricted network access
* direct provider secrets

The runtime acts as the security and policy boundary.

---

# Capability Virtualization

The runtime abstracts physical execution systems behind logical capabilities.

Example:

```text id="9m2k7r"
filesystem.read
```

may internally route to:

* local filesystem
* remote storage
* MCP server
* sandbox overlay
* object storage

Workers should not depend on physical implementation details.

---

# Capability References

Large capability outputs should not be embedded directly in events.

Instead, events should emit references.

Correct:

```json id="6m4k2v"
{
  "refs": {
    "artifact": "artifact://tool-result-123"
  }
}
```

Avoid:

```json id="2m6n8r"
{
  "content": "very large output ..."
}
```

---

# Runtime Responsibilities

The runtime is responsible for:

* capability routing
* permission enforcement
* lease management
* auditing
* observability
* policy enforcement
* isolation boundaries

Workers remain execution-focused.

---

# Relationship to MCP

MCP is treated as an implementation layer within the capability plane.

Thalamus capability events may internally route through:

```text id="5m8k1v"
Thalamus Runtime
 ↓
mcp-routing-gateway
 ↓
MCP Servers
```

MCP provides execution interfaces.

Thalamus provides runtime coordination semantics.

---

# Cognitive Interpretation

Capabilities are not tools attached to intelligent entities.

They are temporary operational affordances exposed by the runtime.

Workers do not possess capabilities.

They temporarily resonate with them during execution.

---

# Philosophy

Traditional agent systems model tools as:

```text id="8m3k1r"
agent-owned abilities
```

Thalamus models capabilities as:

```text id="1m7k4v"
runtime-mediated cognitive infrastructure
```

The runtime owns the nervous system.

Workers temporarily participate within it.
