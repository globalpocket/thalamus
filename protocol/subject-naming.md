# Subject Naming Convention

## Overview

Thalamus uses hierarchical event subjects to coordinate runtime workers through an event-driven cognitive fabric.

Subjects are designed to:

* enable loose coupling
* support distributed execution
* model cognitive signal propagation
* allow scoped subscriptions
* provide predictable routing semantics

The subject hierarchy forms the runtime's cognitive topology.

---

# Naming Grammar

All subjects follow the canonical format:

```text
<domain>.<resource>.<action>
```

Example:

```text
runtime.task.assign
```

Where:

| Segment  | Description                   |
| -------- | ----------------------------- |
| domain   | Runtime subsystem             |
| resource | Entity or target              |
| action   | State transition or operation |

---

# Design Principles

## 1. Subjects Represent Events

Subjects represent:

* state transitions
* runtime signals
* coordination events
* capability requests
* cognition operations

Subjects should describe:

```text
"What happened"
```

rather than:

```text
"What function to call"
```

---

## 2. Event-driven First

Subjects are designed for asynchronous runtime coordination.

The runtime prioritizes:

* propagation
* resonance
* reaction
* coordination

over direct invocation.

---

## 3. Hierarchical Routing

Subjects are hierarchical to support:

* wildcard subscriptions
* scoped execution
* runtime partitioning
* selective observation

Example:

```text
runtime.task.*
```

subscribes to all task-related runtime events.

---

## 4. Stable Semantic Structure

Subjects should remain stable across runtime implementations.

Avoid embedding:

* transport details
* implementation specifics
* provider names
* infrastructure topology

inside subject names.

---

# Canonical Runtime Subjects

## Runtime Lifecycle

```text
runtime.agent.spawn
runtime.agent.exit
runtime.agent.error
```

### Description

| Subject             | Purpose                                        |
| ------------------- | ---------------------------------------------- |
| runtime.agent.spawn | Runtime worker creation                        |
| runtime.agent.exit  | Normal runtime worker termination              |
| runtime.agent.error | Runtime worker failure or abnormal termination |

---

## Task Coordination

```text
runtime.task.assign
runtime.task.result
```

### Description

| Subject             | Purpose                                    |
| ------------------- | ------------------------------------------ |
| runtime.task.assign | Assign a cognitive task to runtime workers |
| runtime.task.result | Publish task execution results             |

---

## Capability Requests

```text
runtime.tool.request
runtime.tool.result
```

### Description

| Subject              | Purpose                             |
| -------------------- | ----------------------------------- |
| runtime.tool.request | Request capability execution        |
| runtime.tool.result  | Publish capability execution result |

---

## Cognition Operations

```text
runtime.llm.request
runtime.llm.response
```

### Description

| Subject              | Purpose                            |
| -------------------- | ---------------------------------- |
| runtime.llm.request  | Request shared cognition inference |
| runtime.llm.response | Publish cognition inference result |

---

# Wildcard Semantics

Thalamus relies heavily on wildcard subscriptions for cognitive coordination.

Examples:

```text
runtime.task.*
```

Subscribe to all task events.

---

```text
runtime.tool.*
```

Subscribe to all capability events.

---

```text
runtime.>
```

Subscribe to the entire runtime event fabric.

---

# Future Scoped Extensions

The canonical grammar may later expand into scoped runtime routing.

Examples:

```text
runtime.sandbox.sb-123.task.assign
```

```text
runtime.worker.wk-9.tool.request
```

```text
runtime.session.session-abc.llm.request
```

These extensions preserve the same hierarchical naming principles.

---

# Naming Rules

## Use lowercase

Correct:

```text
runtime.task.assign
```

Incorrect:

```text
Runtime.Task.Assign
```

---

## Use nouns for resources

Correct:

```text
runtime.task.assign
```

Incorrect:

```text
runtime.assign.task
```

---

## Use verbs for actions

Correct:

```text
runtime.task.assign
```

Incorrect:

```text
runtime.task.assignment
```

---

## Keep subjects semantic

Correct:

```text
runtime.tool.request
```

Incorrect:

```text
runtime.grpc.call
```

---

# Cognitive Interpretation

Subjects should be interpreted as cognitive signals propagating through the runtime.

Example:

```text
runtime.task.assign
```

does not mean:

```text
"call this function"
```

It means:

```text
"a task assignment signal has entered the runtime"
```

Runtime workers may:

* react
* ignore
* amplify
* transform
* propagate

the signal depending on runtime state and subscriptions.

---

# Philosophy

REST models communication as:

```text
A → B
```

Thalamus models communication as:

```text
signal → propagation → resonance → reaction
```

The subject hierarchy defines the nervous system topology of the runtime.

Subjects are not API routes.

They are cognitive pathways.
