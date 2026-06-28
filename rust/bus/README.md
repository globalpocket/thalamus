# thalamus-bus

Message bus implementation for Thalamus event-driven architecture.

## Features

- **BasicBus**: In-memory message bus for testing and simple use cases.
- **NatsBus**: NATS-based message bus for distributed systems (optional feature).

## Usage

### BasicBus

```rust
use thalamus_bus::{BasicBus, MessageBus};

let bus = BasicBus::new();
```

### NatsBus (feature = "nats")

```rust
use thalamus_bus::{NatsBus, NatsBusConfig, MessageBus};

let config = NatsBusConfig::default(); // URL: nats://127.0.0.1:4222
let bus = NatsBus::connect(config).await?;
```

## Configuration

### NatsBusConfig

| Field | Type | Default |
|-------|------|---------|
| `url` | `String` | `"nats://127.0.0.1:4222"` |

## API

### MessageBus Trait

| Method | Description |
|--------|-------------|
| `subscribe` | Register a handler for a subject |
| `publish` | Publish an event envelope |
| `unsubscribe` | Remove a subscription by ID |
| `close` | Close the bus and stop all handlers |
| `is_closed` | Check if the bus is closed |
| `handler_count` | Get the number of handlers for a subject |

## NatsBus Implementation Details

### Subscription Lifecycle

- **subscribe**: Creates a NATS subscription, spawns a Tokio task, and stores its `JoinHandle`.
- **unsubscribe**: Aborts the `JoinHandle` via `handle.abort()`.
- **close**: Aborts all `JoinHandle`s and sets `closed = true`.

### State Management

`NatsBusState` contains:

- `client`: Optional NATS client connection.
- `sub_handles`: Maps subscription ID to `JoinHandle<()>` for each subscriber task.
- `subject_handler_counts`: Tracks the number of handlers per subject.
- `closed`: Flag indicating whether the bus is closed.

### Cloning

`Clone` shares the internal `Arc<RwLock<NatsBusState>>`, so all clones operate on the same connection and subscription state.

## Testing

### NATS Tests

NATS backend tests require a running NATS server. Set the `THALAMUS_NATS_TEST_URL` environment variable to specify the server URL.

```bash
THALAMUS_NATS_TEST_URL=nats://127.0.0.1:4222 cargo test --features nats
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `nats` | Enable NATS backend (`NatsBus`) |
