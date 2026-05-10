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

Start NATS.

```bash
docker run -p 4222:4222 nats
```

### Step 2

Start the worker.

### Step 3

Start the result listener.

### Step 4

Run the publisher.

---

If successful, you will observe a task result event flowing through the runtime.
