## Architecture Flow

```
publisher
 ↓
runtime.task.assign
 ↓
worker
 ↓
runtime.llm.request
 ↓
Cingulater
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
````

### Step 2

Start the worker.

### Step 3

Start the result listener.

### Step 4

Run the publisher.

---

If successful, you will observe the first:

## "Cognitive Pulse"

flowing through the runtime.

````
