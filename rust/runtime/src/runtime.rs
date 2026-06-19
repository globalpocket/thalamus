use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use thalamus_bus::MessageBus;
use thalamus_protocol::payload::{
    RuntimeAgentErrorPayload, RuntimeAgentExitPayload, RuntimeAgentReadyPayload,
    RuntimeLLMRequestPayload, RuntimeTaskAssignPayload, RuntimeTaskResultPayload,
    RuntimeToolRequestPayload,
};
use thalamus_protocol::subject::{
    RUNTIME_AGENT_ERROR, RUNTIME_AGENT_EXIT, RUNTIME_AGENT_READY, RUNTIME_AGENT_SPAWN,
    RUNTIME_LLM_REQUEST, RUNTIME_LLM_RESPONSE, RUNTIME_TASK_ASSIGN, RUNTIME_TASK_RESULT,
    RUNTIME_TOOL_REQUEST, RUNTIME_TOOL_RESULT,
};
use thalamus_protocol::EventEnvelope;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::RuntimeError;
use crate::llm::{LlmProvider, MockLlmProvider};
use crate::registry::WorkerRegistry;
use crate::state::{RuntimeState, TaskHandle, TaskState, TaskStatus};
use crate::tool::{EchoTool, ToolRegistry};

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
    worker_registry: Arc<RwLock<WorkerRegistry>>,
    task_states: Arc<RwLock<HashMap<String, TaskState>>>,
    tool_registry: Arc<RwLock<ToolRegistry>>,
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
        let mut registry = ToolRegistry::new();
        registry.register("echo".to_string(), Box::new(EchoTool::default()));
        Self {
            bus,
            state: Arc::new(RwLock::new(RuntimeState::Initialized)),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            task_handles: Arc::new(RwLock::new(Vec::new())),
            worker_registry: Arc::new(RwLock::new(WorkerRegistry::default())),
            task_states: Arc::new(RwLock::new(HashMap::new())),
            tool_registry: Arc::new(RwLock::new(registry)),
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

    pub async fn worker_registry(&self) -> WorkerRegistry {
        self.worker_registry.read().await.clone()
    }

    pub async fn task_state(&self, id: &str) -> Option<TaskState> {
        self.task_states.read().await.get(id).cloned()
    }

    /// ツールを登録する
    pub async fn register_tool(&self, capability: String, tool: Box<dyn crate::tool::Tool>) {
        let mut registry = self.tool_registry.write().await;
        registry.register(capability, tool);
    }
}

impl<B: MessageBus> ThalamusRuntime<B> {
    async fn ensure_default_handlers(&self) {
        let mut handlers = self.handlers.write().await;
        for subject in [
            RUNTIME_AGENT_SPAWN,
            RUNTIME_AGENT_READY,
            RUNTIME_AGENT_EXIT,
            RUNTIME_AGENT_ERROR,
            RUNTIME_TASK_ASSIGN,
            RUNTIME_TASK_RESULT,
            RUNTIME_LLM_REQUEST,
            RUNTIME_TOOL_REQUEST,
        ] {
            handlers
                .entry(subject.to_string())
                .or_insert_with(|| Arc::new(|_subject, _event| Box::pin(async {})));
        }
    }

    async fn update_agent_ready(&self, event: &EventEnvelope) {
        let Ok(payload) = serde_json::from_value::<RuntimeAgentReadyPayload>(event.payload.clone())
        else {
            return;
        };
        self.worker_registry
            .write()
            .await
            .mark_ready(payload.agent_id, payload.capabilities);
    }

    async fn update_agent_exit(&self, event: &EventEnvelope) {
        let Ok(payload) = serde_json::from_value::<RuntimeAgentExitPayload>(event.payload.clone())
        else {
            return;
        };
        self.worker_registry
            .write()
            .await
            .mark_exited(payload.agent_id, payload.reason);
    }

    async fn update_agent_error(&self, event: &EventEnvelope) {
        let Ok(payload) = serde_json::from_value::<RuntimeAgentErrorPayload>(event.payload.clone())
        else {
            return;
        };
        // Use payload.agent_id if present. Do not guess a worker when agent_id is unknown.
        let agent_id = payload.agent_id.clone().or_else(|| {
            event
                .payload
                .get("agent_id")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        });
        if let Some(agent_id) = agent_id {
            let mut registry = self.worker_registry.write().await;
            registry.mark_error(agent_id, payload.task_id, payload.error);
        }
    }

    async fn update_task_assign(&self, event: &EventEnvelope) {
        let Ok(payload) = serde_json::from_value::<RuntimeTaskAssignPayload>(event.payload.clone())
        else {
            return;
        };
        let task = {
            let mut task_states = self.task_states.write().await;
            task_states
                .entry(payload.task_id.clone())
                .or_insert_with(|| TaskState::new(payload.task_id.clone()))
                .clone()
        };
        *task.parent_task_id.write().await = payload.parent_task_id;
        *task.input.write().await = payload.input;
        *task.capabilities.write().await = payload.capabilities;
        *task.metadata.write().await = payload.metadata;
        *task.correlation_id.write().await = payload.correlation_id;
        if let Some(agent_id) = payload.agent_id {
            task.assign_to(agent_id).await;
        } else {
            task.set_task_status(TaskStatus::Assigned).await;
        }
    }

    async fn update_task_result(&self, event: &EventEnvelope) {
        let Ok(payload) = serde_json::from_value::<RuntimeTaskResultPayload>(event.payload.clone())
        else {
            return;
        };
        let has_error = !payload.error.is_null()
            && !payload
                .error
                .as_object()
                .is_some_and(|object| object.is_empty());
        let status = if has_error {
            TaskStatus::Failed
        } else {
            TaskStatus::from_runtime_status(&payload.status)
        };
        let task = {
            let mut task_states = self.task_states.write().await;
            task_states
                .entry(payload.task_id.clone())
                .or_insert_with(|| TaskState::new(payload.task_id.clone()))
                .clone()
        };
        *task.result.write().await = payload.result;
        *task.error.write().await = payload.error;
        *task.correlation_id.write().await = payload.correlation_id;
        task.set_task_status(status).await;
    }

    async fn update_task_waiting_for_llm(&self, request: &RuntimeLLMRequestPayload) {
        let task = {
            let mut task_states = self.task_states.write().await;
            task_states
                .entry(request.task_id.clone())
                .or_insert_with(|| TaskState::new(request.task_id.clone()))
                .clone()
        };
        *task.correlation_id.write().await = request.correlation_id.clone();
        task.set_task_status(TaskStatus::WaitingForLlm).await;
    }

    async fn update_task_waiting_for_tool(&self, request: &RuntimeToolRequestPayload) {
        let task = {
            let mut task_states = self.task_states.write().await;
            task_states
                .entry(request.task_id.clone())
                .or_insert_with(|| TaskState::new(request.task_id.clone()))
                .clone()
        };
        *task.correlation_id.write().await = request.correlation_id.clone();
        task.set_task_status(TaskStatus::WaitingForTool).await;
    }

    /// Create an envelope with a pre-generated event_id for validation
    fn envelope_with_id(
        subject: String,
        source: String,
        event_id: String,
        payload: serde_json::Value,
    ) -> EventEnvelope {
        EventEnvelope {
            id: event_id,
            subject: subject.clone(),
            r#type: subject.clone(),
            source,
            timestamp: chrono::Utc::now().to_rfc3339(),
            scope: None,
            schema: format!("thalamus.{}", subject),
            payload,
            refs: None,
            correlation_id: None,
            causation_id: None,
            metadata: serde_json::json!({}),
        }
    }

    fn envelope(subject: String, source: String, payload: serde_json::Value) -> EventEnvelope {
        let event_id = Uuid::new_v4().to_string();
        Self::envelope_with_id(subject, source, event_id, payload)
    }

    async fn publish_default_runtime_result(
        &self,
        subject: &str,
        event: &EventEnvelope,
    ) -> Result<(), RuntimeError> {
        match subject {
            RUNTIME_AGENT_READY => self.update_agent_ready(event).await,
            RUNTIME_AGENT_EXIT => self.update_agent_exit(event).await,
            RUNTIME_AGENT_ERROR => self.update_agent_error(event).await,
            RUNTIME_TASK_ASSIGN => self.update_task_assign(event).await,
            RUNTIME_TASK_RESULT => self.update_task_result(event).await,
            RUNTIME_LLM_REQUEST => {
                let Ok(mut request) =
                    serde_json::from_value::<RuntimeLLMRequestPayload>(event.payload.clone())
                else {
                    return Ok(());
                };
                // 补完: request_id が None の場合は request_event.id を設定する
                if request.request_id.is_none() {
                    request.request_id = Some(event.id.clone());
                }
                self.update_task_waiting_for_llm(&request).await;
                let response: thalamus_protocol::payload::RuntimeLLMResponsePayload = MockLlmProvider.complete(request).await?;
                let payload = serde_json::to_value(&response)
                    .map_err(|e| RuntimeError::BusError(format!("serialize failed: {}", e)))?;
                let request_event_id = event.id.clone();
                let mut response_event = Self::envelope(
                    RUNTIME_LLM_RESPONSE.to_string(),
                    "thalamus-runtime".to_string(),
                    payload,
                );
                response_event.correlation_id = Some(request_event_id.clone());
                response_event.causation_id = Some(request_event_id);
                self.bus
                    .publish(response_event)
                    .await
                    .map_err(|e| RuntimeError::BusError(format!("publish failed: {}", e)))?;
            }
            RUNTIME_TOOL_REQUEST => {
                let Ok(mut request) =
                    serde_json::from_value::<RuntimeToolRequestPayload>(event.payload.clone())
                else {
                    return Ok(());
                };
                // 补完: request_id が None の場合は request_event.id を設定する
                if request.request_id.is_none() {
                    request.request_id = Some(event.id.clone());
                }
                self.update_task_waiting_for_tool(&request).await;
                let tool_registry = self.tool_registry.read().await;
                let tool = tool_registry
                    .get(&request.capability)
                    .ok_or_else(|| {
                        RuntimeError::BusError(format!("tool not found: {}", request.capability))
                    })?;
                let result = tool.invoke(request).await?;
                let payload = serde_json::to_value(&result)
                    .map_err(|e| RuntimeError::BusError(format!("serialize failed: {}", e)))?;
                let request_event_id = event.id.clone();
                let mut result_event = Self::envelope(
                    RUNTIME_TOOL_RESULT.to_string(),
                    "thalamus-runtime".to_string(),
                    payload,
                );
                result_event.correlation_id = Some(request_event_id.clone());
                result_event.causation_id = Some(request_event_id);
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
        // Generate event_id for validation
        let event_id = uuid::Uuid::new_v4().to_string();

        // Validate and normalize the payload
        let normalized = thalamus_protocol::validation::validate_and_normalize_payload(
            &subject,
            &event_id,
            payload,
        )
        .map_err(|e| RuntimeError::InvalidPayload(format!("{}: {}", e.subject, e.reason)))?;

        let envelope = Self::envelope_with_id(subject.clone(), source, event_id, normalized);

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
