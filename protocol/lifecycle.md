# Lifecycle Model

## Overview

Thalamus defines a canonical lifecycle model for runtime workers and cognitive execution units.

The lifecycle model standardizes:

* worker state transitions
* runtime supervision
* execution coordination
* failure semantics
* observability
* event propagation behavior

Lifecycle events are propagated through the runtime event fabric using the subject naming and event model specifications.

---

# Lifecycle Philosophy

Thalamus does not model workers as persistent intelligent entities.

Workers are:

* disposable
* runtime-managed
* event-driven
* sandboxed
* temporary cognitive execution units

Workers exist only as long as they are useful to runtime coordination.

The runtime itself is the persistent cognitive substrate.

---

# Core Principles

---

## 1. Workers Are Disposable

Workers are expected to:

* spawn dynamically
* execute tasks
* emit events
* terminate safely

Long-lived worker identity is not required.

---

## 2. Lifecycle Is Event-native

All lifecycle transitions are emitted as runtime events.

Examples:

```text id="4k8r2m"
runtime.agent.spawn
runtime.agent.exit
runtime.agent.error
```

Lifecycle state is observable through the event fabric.

---

## 3. State Transitions Are Explicit

Workers must never silently transition between states.

All meaningful state changes must emit corresponding lifecycle events.

---

## 4. Runtime Owns Supervision

Workers do not self-govern their lifecycle.

Lifecycle ownership belongs to:

* supervisor systems
* runtime coordination layers
* sandbox managers

---

# Canonical Worker Lifecycle

---

## Lifecycle States

```text id="7m2v9k"
CREATED
BOOTING
IDLE
RUNNING
WAITING
BLOCKED
COMPLETED
FAILED
TERMINATED
```

---

# State Definitions

---

## CREATED

The worker has been logically created but execution has not yet started.

Possible operations:

* allocate runtime identifiers
* allocate sandbox metadata
* prepare execution scope

The worker is not yet active.

---

## BOOTING

The runtime is initializing the worker environment.

Typical operations:

* start sandbox container
* mount workspace
* inject runtime SDK
* establish event subscriptions
* initialize cognition session

The worker is becoming operational.

---

## IDLE

The worker is operational but currently unassigned.

The worker is available to receive runtime tasks.

Typical behavior:

* subscribe to runtime subjects
* await task assignment
* maintain heartbeat

---

## RUNNING

The worker is actively processing runtime tasks.

Typical operations:

* inference execution
* capability requests
* workspace mutation
* event propagation

RUNNING represents active cognitive execution.

---

## WAITING

The worker is paused awaiting an external runtime condition.

Examples:

* tool result pending
* LLM inference pending
* event dependency pending
* coordination barrier pending

WAITING is considered healthy runtime behavior.

---

## BLOCKED

The worker cannot proceed because of an abnormal runtime condition.

Examples:

* capability unavailable
* network partition
* sandbox restriction
* permission denial
* runtime dependency failure

BLOCKED differs from WAITING because forward progress is currently impossible.

Supervisor intervention may be required.

---

## COMPLETED

The worker has successfully completed its assigned execution responsibilities.

Typical operations:

* emit final result event
* flush runtime state
* release capability leases

The worker is ready for termination.

---

## FAILED

The worker encountered an unrecoverable execution failure.

Examples:

* runtime exception
* cognition failure
* sandbox crash
* policy violation
* fatal dependency error

FAILED workers should emit diagnostic runtime events before termination.

---

## TERMINATED

The worker has been destroyed and all runtime resources released.

Typical operations:

* destroy sandbox
* release mounts
* release memory references
* close event subscriptions

TERMINATED is the final lifecycle state.

---

# Lifecycle Transition Graph

```text id="9m4k1v"
CREATED
  ↓
BOOTING
  ↓
IDLE
  ↓
RUNNING
  ↓
WAITING ─────┐
  ↓          │
RUNNING ◄────┘
  ↓
COMPLETED
  ↓
TERMINATED
```

Failure transitions:

```text id="1m8k5r"
BOOTING → FAILED
RUNNING → FAILED
WAITING → BLOCKED
BLOCKED → FAILED
FAILED → TERMINATED
```

---

# Lifecycle Events

Each transition should emit corresponding runtime events.

---

## Spawn Events

```text id="5m2n8v"
runtime.agent.spawn
```

Triggered when:

* worker allocation begins
* runtime instantiation occurs

---

## Exit Events

```text id="7m1k4r"
runtime.agent.exit
```

Triggered when:

* worker terminates normally
* execution completes successfully

---

## Error Events

```text id="2m9n5v"
runtime.agent.error
```

Triggered when:

* worker failure occurs
* abnormal runtime state detected
* unrecoverable execution issue appears

---

# Lifecycle Event Example

```json id="8m3k7v"
{
  "id": "evt_01HX...",
  "type": "runtime.agent.spawn",
  "timestamp": "2026-05-10T12:00:00Z",
  "source": "runtime.supervisor",
  "scope": {
    "sandbox": "sb-123",
    "worker": "wk-9"
  },
  "payload": {
    "state": "BOOTING"
  }
}
```

---

# Supervisor Responsibilities

Supervisors are responsible for:

* lifecycle enforcement
* sandbox management
* worker monitoring
* health observation
* cleanup operations
* recovery coordination

Workers should remain lightweight and execution-focused.

---

# Lifecycle and Event Propagation

Lifecycle transitions are propagated through the runtime event fabric.

Other runtime systems may react to lifecycle events:

Examples:

* metrics systems
* replay systems
* orchestration layers
* tracing systems
* debugging tools
* recovery coordinators

The lifecycle model therefore becomes part of the runtime nervous system.

---

# Heartbeats and Health

Future versions may introduce explicit health signaling.

Possible events:

```text id="6m4k2v"
runtime.agent.heartbeat
runtime.agent.timeout
runtime.agent.unresponsive
```

These are intentionally excluded from the minimal lifecycle specification.

---

# Recovery Semantics

The runtime may choose to:

* respawn workers
* replay events
* reassign tasks
* reconstruct cognition state

Recovery behavior is implementation-specific.

The lifecycle model only standardizes observable transitions.

---

# Cognitive Interpretation

Workers are not persistent autonomous entities.

They are temporary runtime structures participating in distributed cognition.

Lifecycle transitions represent:

```text id="3m7n1v"
activation
coordination
suspension
failure
termination
```

within the runtime nervous system.

The runtime persists.

Workers emerge and disappear dynamically.

---

# Philosophy

Traditional systems model execution as:

```text id="8m1k5v"
persistent service instances
```

Thalamus models execution as:

```text id="2m4n8r"
temporary cognitive activation patterns
```

Workers are not the intelligence.

The runtime coordination fabric is.
