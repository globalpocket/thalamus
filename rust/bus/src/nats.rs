#[cfg(feature = "nats")]
use async_trait::async_trait;
#[cfg(feature = "nats")]
use futures::{SinkExt, StreamExt};
#[cfg(feature = "nats")]
use serde_json;
#[cfg(feature = "nats")]
use std::collections::HashMap;
#[cfg(feature = "nats")]
use std::sync::Arc;
#[cfg(feature = "nats")]
use thalamus_protocol::EventEnvelope;
#[cfg(feature = "nats")]
use tokio::sync::RwLock;
#[cfg(feature = "nats")]
use uuid::Uuid;

#[cfg(feature = "nats")]
use crate::{BusError, Handler, MessageBus, SubscriptionId};

/// Configuration for NatsBus.
///
/// # Defaults
///
/// - `url`: `"nats://127.0.0.1:4222"`
#[derive(Clone, Debug)]
pub struct NatsBusConfig {
    /// NATS server URL (default: "nats://127.0.0.1:4222")
    pub url: String,
}

#[cfg(feature = "nats")]
impl Default for NatsBusConfig {
    fn default() -> Self {
        Self {
            url: "nats://127.0.0.1:4222".to_string(),
        }
    }
}

#[cfg(feature = "nats")]
impl NatsBusConfig {
    /// Creates a new NatsBusConfig with the given URL
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

/// Internal state for NatsBus.
///
/// - `client`: Optional NATS client connection.
/// - `sub_handles`: Maps subscription ID to `JoinHandle<()` for each subscriber task.
/// - `subject_handler_counts`: Tracks the number of handlers per subject.
/// - `closed`: Flag indicating whether the bus is closed.
#[cfg(feature = "nats")]
struct NatsBusState {
    client: Option<async_nats::Client>,
    sub_handles: HashMap<String, (String, tokio::task::JoinHandle<()>)>,
    subject_handler_counts: HashMap<String, usize>,
    closed: bool,
}

/// NATS-based message bus implementation.
///
/// This is an MVP at-most-once transport backend. It uses NATS subject-based
/// publishing and subscribing only. It does NOT use JetStream,
/// durable consumers, ack/retry/replay, or persistence.
///
/// # Cloning
///
/// `Clone` shares the internal `Arc<RwLock<NatsBusState>>`, so all clones
/// operate on the same connection and subscription state.
///
/// # Subscription lifecycle
///
/// - `subscribe()`: Creates a NATS subscription, spawns a Tokio task, and stores its `JoinHandle` in `sub_handles`.
/// - `unsubscribe()`: Aborts the `JoinHandle` via `handle.abort()`.
/// - `close()`: Aborts all tracked `JoinHandle`s and sets `closed = true`.
/// - `handler_count()`: Returns the value from `subject_handler_counts` for the given subject.
#[cfg(feature = "nats")]
pub struct NatsBus {
    state: Arc<RwLock<NatsBusState>>,
    url: String,
}

#[cfg(feature = "nats")]
impl Clone for NatsBus {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            url: self.url.clone(),
        }
    }
}

#[cfg(feature = "nats")]
impl NatsBus {
    /// Creates a new NatsBus instance without connecting.
    /// Use `connect()` or `connect_url()` to establish connection.
    pub fn new(config: NatsBusConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(NatsBusState {
                client: None,
                sub_handles: HashMap::new(),
                subject_handler_counts: HashMap::new(),
                closed: true,
            })),
            url: config.url,
        }
    }

    /// Connects to the NATS server with the given config.
    pub async fn connect(config: NatsBusConfig) -> Result<Self, BusError> {
        let bus = Self::new(config);
        bus.connect_inner().await?;
        Ok(bus)
    }

    /// Connects to the NATS server with the given URL.
    pub async fn connect_url(url: impl Into<String>) -> Result<Self, BusError> {
        Self::connect(NatsBusConfig::new(url)).await
    }

    /// Returns true if the bus has been closed.
    pub async fn is_closed(&self) -> bool {
        let state = self.state.read().await;
        state.closed
    }

    async fn connect_inner(&self) -> Result<(), BusError> {
        let url = self.url.clone();

        // Check if already connected
        {
            let state = self.state.read().await;
            if state.client.is_some() {
                return Ok(());
            }
        }

        let client = async_nats::connect(&url)
            .await
            .map_err(|e| BusError::ConnectionError(e.to_string()))?;

        let mut state = self.state.write().await;
        state.client = Some(client);
        state.closed = false;
        Ok(())
    }
}

#[cfg(feature = "nats")]
impl Default for NatsBus {
    fn default() -> Self {
        Self::new(NatsBusConfig::default())
    }
}

#[cfg(feature = "nats")]
#[async_trait]
impl MessageBus for NatsBus {
    /// Subscribes to a subject.
    ///
    /// - Clones the NATS client and creates a subscription.
    /// - Spawns a Tokio task to process messages.
    /// - Stores the `JoinHandle` in `sub_handles` and increments `subject_handler_counts`.
    async fn subscribe(
        &self,
        subject: String,
        handler: Handler,
    ) -> Result<SubscriptionId, BusError> {
        // Check if closed and get client
        let client = {
            let state = self.state.read().await;

            if state.closed {
                return Err(BusError::Closed);
            }

            state
                .client
                .as_ref()
                .ok_or_else(|| BusError::ConnectionError("not connected".to_string()))?
                .clone()
        };

        let subscriber = client
            .subscribe(subject.clone())
            .await
            .map_err(|e| BusError::SubscribeError(e.to_string()))?;

        let sub_id = SubscriptionId(Uuid::new_v4().to_string());
        let handler_for_task = handler.clone();

        let handle = tokio::spawn(async move {
            let mut subscriber = subscriber;
            while let Some(message) = subscriber.next().await {
                let handler = handler_for_task.clone();
                let payload = message.payload.clone();

                // Deserialize payload to EventEnvelope
                let envelope: Result<EventEnvelope, _> = serde_json::from_slice(&payload);

                let envelope = match envelope {
                    Ok(e) => e,
                    Err(_) => {
                        // Drop deserialization failures
                        continue;
                    }
                };

                // Execute handler
                handler(envelope).await;
            }
        });

        // Atomicity: handler_count increment and subscription insert under the same lock
        let mut state = self.state.write().await;

        // Re-check closed flag after acquiring Write lock
        if state.closed {
            handle.abort();
            return Err(BusError::Closed);
        }

        *state
            .subject_handler_counts
            .entry(subject.clone())
            .or_insert(0) += 1;
        state
            .sub_handles
            .insert(sub_id.0.clone(), (subject.clone(), handle));
        drop(state);

        Ok(sub_id)
    }

    /// Publishes an event envelope to the subject.
    ///
    /// Serializes the `EventEnvelope` to JSON and publishes it via the NATS client.
    async fn publish(&self, envelope: EventEnvelope) -> Result<(), BusError> {
        let client = {
            let state = self.state.read().await;
            if state.closed {
                return Err(BusError::Closed);
            }
            state
                .client
                .clone()
                .ok_or_else(|| BusError::ConnectionError("not connected".to_string()))?
        };
        let payload = serde_json::to_string(&envelope).map_err(BusError::SerializationError)?;
        client
            .publish(envelope.subject.clone(), payload.into())
            .await
            .map_err(|e| BusError::PublishError(e.to_string()))?;
        Ok(())
    }

    /// Unsubscribes by ID.
    ///
    /// Aborts the `JoinHandle` via `handle.abort()` and decrements `subject_handler_counts`.
    async fn unsubscribe(&self, id: SubscriptionId) -> Result<(), BusError> {
        let handle = {
            let mut state = self.state.write().await;

            if state.closed {
                return Err(BusError::Closed);
            }

            let (subject, handle) = state
                .sub_handles
                .remove(&id.0)
                .ok_or_else(|| BusError::NotFound(id.0.clone()))?;

            if let Some(count) = state.subject_handler_counts.get_mut(&subject) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    state.subject_handler_counts.remove(&subject);
                }
            }

            handle
        };

        handle.abort();
        Ok(())
    }

    /// Closes the bus.
    ///
    /// Sets `closed = true`, aborts all `JoinHandle`s, clears `subject_handler_counts`, and closes the NATS client.
    ///
    /// Lock boundary: State mutations (`closed`, `sub_handles`, `subject_handler_counts`, `client`)
    /// happen under a single write lock. The `client.close().await` call happens after the lock
    /// is released to avoid holding a Write lock during an async operation.
    async fn close(&self) {
        let (handles, client) = {
            let mut state = self.state.write().await;
            state.closed = true;

            let handles = state
                .sub_handles
                .drain()
                .map(|(_, (_, handle))| handle)
                .collect::<Vec<_>>();

            state.subject_handler_counts.clear();
            let client = state.client.take();

            (handles, client)
        };

        for handle in handles {
            handle.abort();
        }

        if let Some(mut client) = client {
            let _ = client.close().await;
        }
    }

    /// Returns true if the bus has been closed.
    async fn is_closed(&self) -> bool {
        let state = self.state.read().await;
        state.closed
    }

    /// Returns the number of handlers registered for the given subject.
    ///
    /// Reads from `subject_handler_counts`.
    async fn handler_count(&self, subject: &str) -> usize {
        let state = self.state.read().await;
        state
            .subject_handler_counts
            .get(subject)
            .copied()
            .unwrap_or(0)
    }
}

// ==================== Tests ====================

#[cfg(all(test, feature = "nats"))]
mod tests {
    use super::*;

    #[test]
    fn test_nats_bus_config_default() {
        let config = NatsBusConfig::default();
        assert_eq!(config.url, "nats://127.0.0.1:4222");
    }

    #[test]
    fn test_nats_bus_config_new() {
        let config = NatsBusConfig::new("nats://test:4222");
        assert_eq!(config.url, "nats://test:4222");
    }

    #[test]
    fn test_nats_bus_new_is_closed() {
        let bus = NatsBus::new(NatsBusConfig::default());
        assert!(tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(bus.is_closed()));
    }

    #[test]
    fn test_nats_bus_handler_count_zero() {
        let bus = NatsBus::new(NatsBusConfig::default());
        assert_eq!(
            tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap()
                .block_on(bus.handler_count("test.subject")),
            0
        );
    }

    #[test]
    fn test_nats_bus_close_sets_state() {
        let bus = NatsBus::new(NatsBusConfig::default());
        // Initial state: closed = true
        assert!(
            tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap()
                .block_on(bus.is_closed()),
            "Initial state should be closed"
        );
        // close() sets closed = true (already true, but verifies the operation)
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(bus.close());
        // State should still be closed
        assert!(
            tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap()
                .block_on(bus.is_closed()),
            "State should remain closed after close()"
        );
    }
}
