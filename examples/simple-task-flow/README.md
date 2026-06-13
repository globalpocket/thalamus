## Simple Task Flow

This is an in-memory reference runtime example. It demonstrates the
runtime task contract without requiring any external services, network
listeners, containers, or real HTTP calls.

## Architecture Flow

```text
publisher
 ↓
runtime.task.assign
 ↓
worker
 ↓
runtime.task.result
 ↓
result_listener
```

## Execution Steps

### Step 1

Run the example worker directly from the repository root.

```bash
python examples/simple-task-flow/worker.py
```

### Step 2

The script creates a `ThalamusRuntime` with an in-memory event bus and
subscribes a worker handler to `runtime.task.assign`.

### Step 3

The same process publishes one task assignment event.

### Step 4

The worker publishes a `runtime.task.result` event with a successful
summary payload.

---

If successful, you will observe both task assignment and task result events
printed from the in-memory reference runtime example.
