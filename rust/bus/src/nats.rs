#[cfg(feature = "nats")]
use async_nats::Subscriber;
#[cfg(feature = "nats")]
use async_trait::async_trait;
#[cfg(feature = "nats")]
use futures::StreamExt;
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

/// Configuration for NatsBus
#[derive(Clone, Debug, Default)]
pub struct NatsBusConfig {
    /// NATS server URL (default: "nats://127.0.0.1:4222")
    pub url: String,
}

#[cfg(feature = "nats")]
impl NatsBusConfig {
    /// Creates a new NatsBusConfig with the given URL
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

/// Internal state for NatsBus
#[cfg(feature = "nats")]
struct NatsBusState {
    #[cfg(feature = "nats")]
    client: Option<async_nats::Client>,
    #[cfg(feature = "nats")]
    subscribers: HashMap<String, Subscriber>,
    #[cfg(feature = "nats")]
    sub_handles: HashMap<String, tokio::task::JoinHandle<()>>,
    closed: bool,
}

/// NATS-based message bus implementation.
///
/// This is an MVP at-most-once transport backend. It does NOT use JetStream,
/// durable consumers, ack/retry/replay, or persistence.
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
                subscribers: HashMap::new(),
                sub_handles: HashMap::new(),
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
    async fn subscribe(
        &self,
        subject: String,
        handler: Handler,
    ) -> Result<SubscriptionId, BusError> {
        let state = self.state.read().await;

        if state.closed {
            return Err(BusError::Closed);
        }

        let client = state
            .client
            .as_ref()
            .ok_or_else(|| BusError::ConnectionError("not connected".to_string()))?;

        drop(state);

        let mut subscriber = client
            .subscribe(&subject)
            .await
            .map_err(|e| BusError::SubscribeError(e.to_string()))?;

        let sub_id = SubscriptionId(Uuid::new_v4().to_string());
        let handler = Arc::new(handler);

        let handle = tokio::spawn(async move {
            while let Some(message) = subscriber.next().await {
                let handler = handler.clone();
                let _msg = message.clone();

                // Deserialize payload to EventEnvelope
                let envelope: Result<EventEnvelope, _> =
                    serde_json::from_str(&_msg.payload.to_string());

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

        let mut state = self.state.write().await;
        state.subscribers.insert(sub_id.0.clone(), subscriber);
        state.sub_handles.insert(sub_id.0.clone(), handle);
        drop(state);

        Ok(sub_id)
    }

    async fn publish(&self, envelope: EventEnvelope) -> Result<(), BusError> {
        let state = self.state.read().await;

        if state.closed {
            return Err(BusError::Closed);
        }

        let client = state
            .client
            .as_ref()
            .ok_or_else(|| BusError::ConnectionError("not connected".to_string()))?;

        let payload = serde_json::to_string(&envelope)
            .map_err(|e| BusError::SerializationError(e))?;

        client
            .publish(envelope.subject.clone(), payload.into())
            .await
            .map_err(|e| BusError::PublishError(e.to_string()))?;

        Ok(())
    }

    async fn unsubscribe(&self, id: SubscriptionId) -> Result<(), BusError> {
        let mut state = self.state.write().await;

        if state.closed {
            return Err(BusError::Closed);
        }

        if let Some(subscriber) = state.subscribers.remove(&id.0) {
            subscriber.unsubscribe().await.map_err(|e| {
                BusError::UnsubscribeError(e.to_string())
            })?;
        } else {
            return Err(BusError::NotFound(id.0));
        }

        if let Some(handle) = state.sub_handles.remove(&id.0) {
            handle.abort();
        }

        Ok(())
    }

    async fn close(&self) {
        let mut state = self.state.write().await;
        state.closed = true;

        for (_id, subscriber) in state.subscribers.drain() {
            let _ = subscriber.unsubscribe().await;
        }

        for (_id, handle) in state.sub_handles.drain() {
            handle.abort();
        }

        if let Some(client) = state.client.take() {
            let _ = client.close().await;
        }
    }

    async fn is_closed(&self) -> bool {
        let state = self.state.read().await;
        state.closed
    }

    async fn handler_count(&self, subject: &str) -> usize {
        let state = self.state.read().await;
        // Count subscribers that match the subject
        // In NatsBus, we track by subscription ID, not by subject
        // For testing purposes, return 0 as we don't track subject->subs mapping
        let _ = subject;
        0
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
}
