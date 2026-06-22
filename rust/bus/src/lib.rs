use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use thalamus_protocol::EventEnvelope;
use tokio::sync::RwLock;
use uuid::Uuid;

/// BusError: バス操作時のエラー型
#[derive(Debug, thiserror::Error)]
pub enum BusError {
    #[error("subject not found: {0}")]
    NotFound(String),
    #[error("already subscribed: {0}")]
    AlreadySubscribed(String),
    #[error("bus is closed")]
    Closed,
    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("protocol error")]
    ProtocolError,
}

/// Handler: イベントハンドラーの型定義
pub type Handler = Arc<
    dyn Fn(EventEnvelope) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// Subscription: サブスクリプション情報
pub struct Subscription {
    pub id: String,
    pub subject: String,
    pub handler: Handler,
}

impl fmt::Debug for Subscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Subscription")
            .field("id", &self.id)
            .field("subject", &self.subject)
            .finish_non_exhaustive()
    }
}

/// SubscriptionId: サブスクリプションID
#[derive(Debug, Clone)]
pub struct SubscriptionId(pub String);

impl fmt::Display for SubscriptionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SubscriptionId({})", self.0)
    }
}

/// MessageBus: メッセージバスのトレイト定義
#[async_trait]
pub trait MessageBus: Send + Sync + Clone {
    /// 新しいサブスクリプションを登録する
    async fn subscribe(
        &mut self,
        subject: String,
        handler: Handler,
    ) -> Result<SubscriptionId, BusError>;

    /// メッセージを公開する（サブジェクトにマッチするハンドラーに配信）
    async fn publish(&self, envelope: EventEnvelope) -> Result<(), BusError>;

    /// サブスクリプションを解除する
    async fn unsubscribe(&mut self, id: SubscriptionId) -> Result<(), BusError>;

    /// バスを閉じる（すべてのハンドラーを停止）
    async fn close(&mut self);

    /// バスが閉じられているか
    async fn is_closed(&self) -> bool;

    /// サブジェクトに登録されたハンドラー数を取得
    async fn handler_count(&self, subject: &str) -> usize;
}

/// BasicBus: 基本バス構造体
#[derive(Clone)]
pub struct BasicBus {
    subscribers: Arc<RwLock<HashMap<String, Vec<Subscription>>>>,
    closed: Arc<RwLock<bool>>,
    published_events: Arc<RwLock<Vec<EventEnvelope>>>,
}

impl BasicBus {
    /// 新しいBasicBusを作成する
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            closed: Arc::new(RwLock::new(false)),
            published_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 公開済みイベントを観測用に取得する
    pub async fn published_events(&self) -> Vec<EventEnvelope> {
        self.published_events.read().await.clone()
    }

    /// サブジェクトにマッチするハンドラーを取得する
    async fn get_handlers(&self, subject: &str) -> Vec<Handler> {
        let subscribers = self.subscribers.read().await;
        subscribers
            .get(subject)
            .map(|subs| subs.iter().map(|s| s.handler.clone()).collect())
            .unwrap_or_default()
    }
}

impl Default for BasicBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageBus for BasicBus {
    async fn subscribe(
        &mut self,
        subject: String,
        handler: Handler,
    ) -> Result<SubscriptionId, BusError> {
        let mut subscribers = self.subscribers.write().await;
        let id = Uuid::new_v4().to_string();
        let subscription_id = SubscriptionId(id.clone());

        let subscription = Subscription {
            id,
            subject: subject.clone(),
            handler,
        };

        subscribers
            .entry(subject)
            .or_insert_with(Vec::new)
            .push(subscription);

        Ok(subscription_id)
    }

    async fn publish(&self, envelope: EventEnvelope) -> Result<(), BusError> {
        let is_closed = *self.closed.read().await;
        if is_closed {
            return Err(BusError::Closed);
        }

        let handlers = self.get_handlers(&envelope.subject).await;

        self.published_events.write().await.push(envelope.clone());

        // ハンドラーを直列実行（状態競合防止）
        for handler in handlers {
            let env = envelope.clone();
            handler(env).await;
        }

        Ok(())
    }

    async fn unsubscribe(&mut self, id: SubscriptionId) -> Result<(), BusError> {
        let mut subscribers = self.subscribers.write().await;

        for (_subject, subs) in subscribers.iter_mut() {
            let original_len = subs.len();
            subs.retain(|s| s.id != id.0);
            if subs.len() < original_len {
                return Ok(());
            }
        }

        Err(BusError::NotFound(id.0))
    }

    async fn close(&mut self) {
        let mut closed = self.closed.write().await;
        *closed = true;
        self.subscribers.write().await.clear();
    }

    async fn is_closed(&self) -> bool {
        *self.closed.read().await
    }

    async fn handler_count(&self, subject: &str) -> usize {
        let subscribers = self.subscribers.read().await;
        subscribers.get(subject).map(|s| s.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_basic_bus_new() {
        let bus = BasicBus::new();
        assert!(!bus.is_closed().await);
    }

    #[test]
    async fn test_subscribe_and_handler_count() {
        let mut bus = BasicBus::new();
        let handler: Handler = Arc::new(|_| Box::pin(async {}));

        let id = bus
            .subscribe("test.subject".to_string(), handler)
            .await
            .unwrap();

        assert_eq!(bus.handler_count("test.subject").await, 1);
        assert_eq!(id.0.len(), 36); // UUID v4 format
    }

    #[test]
    async fn test_unsubscribe() {
        let mut bus = BasicBus::new();
        let handler: Handler = Arc::new(|_| Box::pin(async {}));

        let id = bus
            .subscribe("test.subject".to_string(), handler)
            .await
            .unwrap();
        assert_eq!(bus.handler_count("test.subject").await, 1);

        bus.unsubscribe(id).await.unwrap();
        assert_eq!(bus.handler_count("test.subject").await, 0);
    }

    #[test]
    async fn test_unsubscribe_not_found() {
        let mut bus = BasicBus::new();
        let fake_id = SubscriptionId("non-existent".to_string());
        assert!(bus.unsubscribe(fake_id).await.is_err());
    }

    #[test]
    async fn test_close() {
        let mut bus = BasicBus::new();
        bus.close().await;
        assert!(bus.is_closed().await);
        assert_eq!(bus.handler_count("any").await, 0);
    }

    #[test]
    async fn test_publish_no_handlers() {
        let bus = BasicBus::new();
        let envelope = EventEnvelope {
            id: "test-id".to_string(),
            r#type: "test.event".to_string(),
            subject: "test.subject".to_string(),
            source: "test".to_string(),
            timestamp: Uuid::new_v4().to_string(),
            schema: "1.0".to_string(),
            scope: None,
            refs: None,
            payload: serde_json::json!({}),
            correlation_id: None,
            causation_id: None,
            metadata: serde_json::json!({}),
        };

        assert!(bus.publish(envelope).await.is_ok());
        assert_eq!(bus.published_events().await.len(), 1);
    }
}
