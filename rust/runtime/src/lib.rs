use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use thalamus_bus::MessageBus;
use thalamus_protocol::{
    payload::{
        RuntimeLLMRequestPayload, RuntimeLLMResponsePayload, RuntimeToolRequestPayload,
        RuntimeToolResultPayload,
    },
    subject::{
        RUNTIME_AGENT_SPAWN, RUNTIME_LLM_REQUEST, RUNTIME_LLM_RESPONSE, RUNTIME_TASK_ASSIGN,
        RUNTIME_TOOL_REQUEST, RUNTIME_TOOL_RESULT,
    },
    EventEnvelope,
};
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
        f.debug_struct("TaskHandle").field("id", &self.id).finish()
    }
}

/// EventHandler: イベントハンドラーの型定義
pub type EventHandler = Arc<
    dyn Fn(String, EventEnvelope) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// TaskState: ランタイムが追跡するタスク状態
#[derive(Debug, Clone)]
pub struct TaskState {
    id: String,
    assigned_agent: Arc<RwLock<Option<String>>>,
}

impl TaskState {
    pub fn new(id: String) -> Self {
        Self {
            id,
            assigned_agent: Arc::new(RwLock::new(None)),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub async fn assign_to(&self, agent_id: String) {
        *self.assigned_agent.write().await = Some(agent_id);
    }

    pub async fn assigned_agent(&self) -> Option<String> {
        self.assigned_agent.read().await.clone()
    }
}

/// WorkerInfo: 登録済みワーカーの公開情報
#[derive(Debug, Clone, PartialEq)]
pub struct WorkerInfo {
    pub id: String,
    pub capabilities: Vec<String>,
}

/// WorkerRegistry: ワーカーIDから能力を検索する最小レジストリ
#[derive(Debug, Default, Clone)]
pub struct WorkerRegistry {
    workers: HashMap<String, WorkerInfo>,
}

impl WorkerRegistry {
    pub fn register(&mut self, id: String, capabilities: Vec<String>) {
        let worker = WorkerInfo {
            id: id.clone(),
            capabilities,
        };
        self.workers.insert(id, worker);
    }

    pub fn lookup(&self, id: &str) -> Option<&WorkerInfo> {
        self.workers.get(id)
    }
}

/// MockLlmProvider: 入力プロンプトから決定的なモック応答を返すLLMプロバイダ
#[derive(Debug, Default, Clone)]
pub struct MockLlmProvider;

impl MockLlmProvider {
    pub async fn complete(
        &self,
        request: RuntimeLLMRequestPayload,
    ) -> Result<RuntimeLLMResponsePayload, RuntimeError> {
        let response_input = request.prompt;

        Ok(RuntimeLLMResponsePayload {
            request_id: request.request_id,
            task_id: request.task_id.unwrap_or_default(),
            status: "completed".to_string(),
            text: Some(format!("Mock response: {}", response_input)),
            model: request.model.unwrap_or_else(|| "mock".to_string()),
            error: None,
        })
    }
}

/// EchoTool: 入力payloadをそのまま結果として返すツール
#[derive(Debug, Default, Clone)]
pub struct EchoTool;

impl EchoTool {
    pub async fn invoke(
        &self,
        request: RuntimeToolRequestPayload,
    ) -> Result<RuntimeToolResultPayload, RuntimeError> {
        Ok(RuntimeToolResultPayload {
            request_id: request.request_id,
            task_id: request.task_id.unwrap_or_default(),
            status: "completed".to_string(),
            output: Some(request.input),
            error: None,
        })
    }
}

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
    async fn ensure_default_handlers(&self) {
        let mut handlers = self.handlers.write().await;
        for subject in [
            RUNTIME_AGENT_SPAWN,
            RUNTIME_TASK_ASSIGN,
            RUNTIME_LLM_REQUEST,
            RUNTIME_TOOL_REQUEST,
        ] {
            handlers
                .entry(subject.to_string())
                .or_insert_with(|| Arc::new(|_subject, _event| Box::pin(async {})));
        }
    }

    fn envelope(subject: String, source: String, payload: serde_json::Value) -> EventEnvelope {
        EventEnvelope {
            id: Uuid::new_v4().to_string(),
            subject: subject.clone(),
            r#type: subject.clone(),
            source,
            timestamp: chrono::Utc::now().to_rfc3339(),
            scope: None,
            schema: format!("thalamus.{}", subject),
            payload,
            refs: Vec::new(),
            correlation_id: None,
            causation_id: None,
            metadata: serde_json::json!({}),
        }
    }

    async fn publish_default_runtime_result(
        &self,
        subject: &str,
        event: &EventEnvelope,
    ) -> Result<(), RuntimeError> {
        match subject {
            RUNTIME_LLM_REQUEST => {
                let Ok(request) =
                    serde_json::from_value::<RuntimeLLMRequestPayload>(event.payload.clone())
                else {
                    return Ok(());
                };
                let response = MockLlmProvider.complete(request).await?;
                let payload = serde_json::to_value(response)
                    .map_err(|e| RuntimeError::BusError(format!("serialize failed: {}", e)))?;
                let response_event = Self::envelope(
                    RUNTIME_LLM_RESPONSE.to_string(),
                    "thalamus-runtime".to_string(),
                    payload,
                );
                self.bus
                    .publish(response_event)
                    .await
                    .map_err(|e| RuntimeError::BusError(format!("publish failed: {}", e)))?;
            }
            RUNTIME_TOOL_REQUEST => {
                let Ok(request) =
                    serde_json::from_value::<RuntimeToolRequestPayload>(event.payload.clone())
                else {
                    return Ok(());
                };
                let result = EchoTool.invoke(request).await?;
                let payload = serde_json::to_value(result)
                    .map_err(|e| RuntimeError::BusError(format!("serialize failed: {}", e)))?;
                let result_event = Self::envelope(
                    RUNTIME_TOOL_RESULT.to_string(),
                    "thalamus-runtime".to_string(),
                    payload,
                );
                self.bus
                    .publish(result_event)
                    .await
                    .map_err(|e| RuntimeError::BusError(format!("publish failed: {}", e)))?;
            }
            _ => {}
        }

        Ok(())
    }

    /// ランタイムを起動する
    pub async fn start(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.write().await;

        if *state == RuntimeState::Running {
            return Err(RuntimeError::LifecycleError("already running".to_string()));
        }

        *state = RuntimeState::Starting;

        self.ensure_default_handlers().await;

        // ハンドラーをバスに登録
        {
            let handlers = self.handlers.read().await;
            for (subject, handler) in handlers.iter() {
                let bus_subject = subject.clone();
                let handler_subject = bus_subject.clone();
                let handler_clone = handler.clone();

                let subscription_result = self
                    .bus
                    .subscribe(
                        bus_subject,
                        Arc::new(move |envelope| {
                            let h = handler_clone.clone();
                            let subject = handler_subject.clone();
                            Box::pin(async move {
                                h(subject, envelope).await;
                            })
                        }),
                    )
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
        if self.bus.handler_count(&subject).await == 0 {
            return Err(RuntimeError::BusError(format!(
                "publish failed: subject not found: {}",
                subject
            )));
        }

        let envelope = Self::envelope(subject.clone(), source, payload);

        self.bus
            .publish(envelope.clone())
            .await
            .map_err(|e| RuntimeError::BusError(format!("publish failed: {}", e)))?;

        self.publish_default_runtime_result(&subject, &envelope)
            .await?;

        Ok(envelope)
    }

    /// イベントを処理する
    pub async fn handle_event(
        &self,
        subject: String,
        event: EventEnvelope,
    ) -> Result<(), RuntimeError> {
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
