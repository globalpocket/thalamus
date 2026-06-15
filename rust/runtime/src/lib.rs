use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use thalamus_bus::MessageBus;
use thalamus_protocol::EventEnvelope;
use tokio::sync::RwLock;
use uuid::Uuid;

/// RuntimeError: ランタイム操作時のエラー型
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("bus error: {0}")]
    BusError(String),
    #[error("schedule error: {0}")]
    ScheduleError(String),
    #[error("lifecycle error: {0}")]
    LifecycleError(String),
}

/// RuntimeState: ランタイムのライフサイクル状態
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeState {
    /// 初期状態
    Initialized,
    /// 起動中
    Starting,
    /// 実行中
    Running,
    /// 停止中
    Stopping,
    /// 停止済み
    Stopped,
}

/// TaskHandle: スパwnされたタスクのハンドル
#[derive(Clone)]
pub struct TaskHandle {
    pub id: String,
}

impl fmt::Debug for TaskHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskHandle")
            .field("id", &self.id)
            .finish()
    }
}

/// EventHandler: イベントハンドラーの型定義
pub type EventHandler = Arc<
    dyn Fn(String, EventEnvelope) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// ThalamusRuntime: メインランタイム構造体
pub struct ThalamusRuntime<B: MessageBus> {
    bus: B,
    state: Arc<RwLock<RuntimeState>>,
    handlers: Arc<RwLock<HashMap<String, EventHandler>>>,
    task_handles: Arc<RwLock<Vec<TaskHandle>>>,
}

impl<B: MessageBus> fmt::Debug for ThalamusRuntime<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThalamusRuntime")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl<B: MessageBus> ThalamusRuntime<B> {
    /// 新しいThalamusRuntimeインスタンスを作成する
    pub fn new(bus: B) -> Self {
        Self {
            bus,
            state: Arc::new(RwLock::new(RuntimeState::Initialized)),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            task_handles: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 現在の状態を取得する
    pub async fn state(&self) -> RuntimeState {
        self.state.read().await.clone()
    }

    /// イベントハンドラーを登録する
    pub async fn register_handler(&self, subject: String, handler: EventHandler) {
        let mut handlers = self.handlers.write().await;
        handlers.insert(subject, handler);
    }

    /// イベントハンドラーを削除する
    pub async fn unregister_handler(&self, subject: &str) {
        let mut handlers = self.handlers.write().await;
        handlers.remove(subject);
    }

    /// タスクをスパwnする
    pub async fn spawn<F>(&self, future: F) -> TaskHandle
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let id = Uuid::new_v4().to_string();
        let handle = TaskHandle { id: id.clone() };

        {
            let mut task_handles = self.task_handles.write().await;
            task_handles.push(handle.clone());
        }

        let task_handles = Arc::clone(&self.task_handles);
        tokio::spawn(async move {
            future.await;

            let mut task_handles = task_handles.write().await;
            task_handles.retain(|handle| handle.id != id);
        });

        handle
    }

    /// 登録されたハンドラー数を取得する
    pub async fn handler_count(&self) -> usize {
        self.handlers.read().await.len()
    }

    /// 実行中のタスク数を取得する
    pub async fn active_task_count(&self) -> usize {
        self.task_handles.read().await.len()
    }
}

impl<B: MessageBus> ThalamusRuntime<B> {
    /// ランタイムを起動する
    pub async fn start(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.write().await;

        if *state == RuntimeState::Running {
            return Err(RuntimeError::LifecycleError("already running".to_string()));
        }

        *state = RuntimeState::Starting;

        // ハンドラーをバスに登録
        {
            let handlers = self.handlers.read().await;
            for (subject, handler) in handlers.iter() {
                let bus_subject = subject.clone();
                let handler_subject = bus_subject.clone();
                let handler_clone = handler.clone();

                let subscription_result = self
                    .bus
                    .subscribe(bus_subject, Arc::new(move |envelope| {
                        let h = handler_clone.clone();
                        let subject = handler_subject.clone();
                        Box::pin(async move {
                            h(subject, envelope).await;
                        })
                    }))
                    .await;

                match subscription_result {
                    Ok(_) => {}
                    Err(e) => {
                        *state = RuntimeState::Initialized;
                        return Err(RuntimeError::BusError(e.to_string()));
                    }
                }
            }
        }

        *state = RuntimeState::Running;
        Ok(())
    }

    /// ランタイムを停止する
    pub async fn stop(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.write().await;

        if *state == RuntimeState::Stopped || *state == RuntimeState::Stopping {
            return Err(RuntimeError::LifecycleError("already stopped".to_string()));
        }

        if *state != RuntimeState::Running {
            return Err(RuntimeError::LifecycleError(
                "not running, cannot stop".to_string(),
            ));
        }

        *state = RuntimeState::Stopping;

        // バスを閉じる
        self.bus.close().await;

        *state = RuntimeState::Stopped;
        Ok(())
    }

    /// イベントを公開する
    pub async fn publish(
        &self,
        subject: String,
        source: String,
        payload: serde_json::Value,
    ) -> Result<EventEnvelope, RuntimeError> {
        let envelope = EventEnvelope {
            id: Uuid::new_v4().to_string(),
            subject: subject.clone(),
            source,
            timestamp: chrono::Utc::now().to_rfc3339(),
            schema: format!("thalamus.{}", subject),
            payload,
            correlation_id: None,
            causation_id: None,
            metadata: serde_json::json!({}),
        };

        self.bus.publish(envelope.clone()).await.map_err(|e| {
            RuntimeError::BusError(format!("publish failed: {}", e))
        })?;

        Ok(envelope)
    }

    /// イベントを処理する
    pub async fn handle_event(&self, subject: String, event: EventEnvelope) -> Result<(), RuntimeError> {
        let handlers = self.handlers.read().await;

        if let Some(handler) = handlers.get(&subject) {
            handler(subject, event).await;
            Ok(())
        } else {
            Err(RuntimeError::ScheduleError(format!(
                "no handler for subject: {}",
                subject
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RuntimeState, ThalamusRuntime};
    use thalamus_bus::BasicBus;

    #[tokio::test]
    async fn test_new_runtime_initialized() {
        let runtime = ThalamusRuntime::new(BasicBus::new());

        assert_eq!(runtime.state().await, RuntimeState::Initialized);
    }
}
