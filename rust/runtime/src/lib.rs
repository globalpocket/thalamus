use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use thalamus_bus::MessageBus;
use thalamus_protocol::{
    payload::{
        RuntimeAgentErrorPayload, RuntimeAgentExitPayload, RuntimeAgentReadyPayload,
        RuntimeLLMRequestPayload, RuntimeLLMResponsePayload, RuntimeTaskAssignPayload,
        RuntimeTaskResultPayload, RuntimeToolRequestPayload, RuntimeToolResultPayload,
    },
    subject::{
        RUNTIME_AGENT_ERROR, RUNTIME_AGENT_EXIT, RUNTIME_AGENT_READY, RUNTIME_AGENT_SPAWN,
        RUNTIME_LLM_REQUEST, RUNTIME_LLM_RESPONSE, RUNTIME_TASK_ASSIGN, RUNTIME_TASK_RESULT,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Assigned,
    Running,
    WaitingForLlm,
    WaitingForTool,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Assigned => "assigned",
            Self::Running => "running",
            Self::WaitingForLlm => "waiting_for_llm",
            Self::WaitingForTool => "waiting_for_tool",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    fn from_runtime_status(status: &str) -> Self {
        match status {
            "assigned" => Self::Assigned,
            "running" => Self::Running,
            "waiting_for_llm" => Self::WaitingForLlm,
            "waiting_for_tool" => Self::WaitingForTool,
            "completed" | "success" => Self::Completed,
            "failed" | "failure" | "error" => Self::Failed,
            "cancelled" | "canceled" => Self::Cancelled,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskState {
    pub task_id: String,
    pub parent_task_id: Arc<RwLock<Option<String>>>,
    pub agent_id: Arc<RwLock<Option<String>>>,
    pub status: Arc<RwLock<TaskStatus>>,
    pub input: Arc<RwLock<serde_json::Value>>,
    pub capabilities: Arc<RwLock<Vec<String>>>,
    pub metadata: Arc<RwLock<serde_json::Value>>,
    pub result: Arc<RwLock<Option<serde_json::Value>>>,
    pub error: Arc<RwLock<serde_json::Value>>,
    pub correlation_id: Arc<RwLock<Option<String>>>,
}

impl TaskState {
    pub fn new(id: String) -> Self {
        Self {
            task_id: id,
            parent_task_id: Arc::new(RwLock::new(None)),
            agent_id: Arc::new(RwLock::new(None)),
            status: Arc::new(RwLock::new(TaskStatus::Pending)),
            input: Arc::new(RwLock::new(serde_json::Value::Null)),
            capabilities: Arc::new(RwLock::new(Vec::new())),
            metadata: Arc::new(RwLock::new(serde_json::json!({}))),
            result: Arc::new(RwLock::new(None)),
            error: Arc::new(RwLock::new(serde_json::Value::Null)),
            correlation_id: Arc::new(RwLock::new(None)),
        }
    }

    pub fn id(&self) -> &str {
        &self.task_id
    }

    pub async fn assign_to(&self, agent_id: String) {
        *self.agent_id.write().await = Some(agent_id);
        self.set_task_status(TaskStatus::Assigned).await;
    }

    pub async fn assigned_agent(&self) -> Option<String> {
        self.agent_id.read().await.clone()
    }

    pub async fn set_status(&self, status: String) {
        self.set_task_status(TaskStatus::from_runtime_status(&status))
            .await;
    }

    pub async fn set_task_status(&self, status: TaskStatus) {
        *self.status.write().await = status;
    }

    pub async fn status(&self) -> String {
        self.status.read().await.as_str().to_string()
    }
}

/// WorkerState: 登録済みワーカーの状態
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerState {
    Ready,
    Exited,
    Error,
}

impl WorkerState {
    fn from_runtime_state(state: &str) -> Self {
        match state {
            "exited" => Self::Exited,
            "error" => Self::Error,
            _ => Self::Ready,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Exited => "exited",
            Self::Error => "error",
        }
    }
}

impl PartialEq<&str> for WorkerState {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<WorkerState> for &str {
    fn eq(&self, other: &WorkerState) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<String> for WorkerState {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<WorkerState> for String {
    fn eq(&self, other: &WorkerState) -> bool {
        self == other.as_str()
    }
}

/// WorkerRecord: 登録済みワーカーの公開情報
#[derive(Debug, Clone, PartialEq)]
pub struct WorkerRecord {
    pub id: String,
    pub agent_id: String,
    pub capabilities: Vec<String>,
    pub state: WorkerState,
    pub last_error: serde_json::Value,
    pub metadata: serde_json::Value,
}

pub type WorkerInfo = WorkerRecord;

/// WorkerRegistry: ワーカーIDから能力を検索する最小レジストリ
#[derive(Debug, Default, Clone)]
pub struct WorkerRegistry {
    workers: HashMap<String, WorkerInfo>,
}

impl WorkerRegistry {
    pub fn mark_ready(&mut self, agent_id: String, capabilities: Vec<String>) {
        let worker = WorkerRecord {
            id: agent_id.clone(),
            agent_id: agent_id.clone(),
            capabilities,
            state: WorkerState::Ready,
            last_error: serde_json::Value::Null,
            metadata: serde_json::json!({}),
        };
        self.workers.insert(agent_id, worker);
    }

    pub fn mark_exited(&mut self, agent_id: String, reason: Option<String>) {
        let metadata = reason
            .map(|reason| serde_json::json!({ "reason": reason }))
            .unwrap_or_else(|| serde_json::json!({}));
        self.workers
            .entry(agent_id.clone())
            .and_modify(|worker| {
                worker.state = WorkerState::Exited;
                worker.metadata = metadata.clone();
            })
            .or_insert_with(|| WorkerRecord {
                id: agent_id.clone(),
                agent_id,
                capabilities: Vec::new(),
                state: WorkerState::Exited,
                last_error: serde_json::Value::Null,
                metadata,
            });
    }

    pub fn mark_error(
        &mut self,
        agent_id: String,
        task_id: Option<String>,
        error: serde_json::Value,
    ) {
        let metadata = task_id
            .map(|task_id| serde_json::json!({ "task_id": task_id }))
            .unwrap_or_else(|| serde_json::json!({}));
        self.workers
            .entry(agent_id.clone())
            .and_modify(|worker| {
                worker.state = WorkerState::Error;
                worker.last_error = error.clone();
                worker.metadata = metadata.clone();
            })
            .or_insert_with(|| WorkerRecord {
                id: agent_id.clone(),
                agent_id,
                capabilities: Vec::new(),
                state: WorkerState::Error,
                last_error: error,
                metadata,
            });
    }

    pub fn find_by_capability(&self, capability: &str) -> Vec<String> {
        self.workers
            .values()
            .filter(|worker| {
                worker.state == WorkerState::Ready
                    && worker
                        .capabilities
                        .iter()
                        .any(|worker_capability| worker_capability == capability)
            })
            .map(|worker| worker.agent_id.clone())
            .collect()
    }

    pub fn register(&mut self, id: String, capabilities: Vec<String>) {
        self.mark_ready(id, capabilities);
    }

    pub fn set_state(&mut self, id: String, state: String) {
        let worker_state = WorkerState::from_runtime_state(&state);
        self.workers
            .entry(id.clone())
            .and_modify(|worker| worker.state = worker_state.clone())
            .or_insert_with(|| WorkerRecord {
                id: id.clone(),
                agent_id: id,
                capabilities: Vec::new(),
                state: worker_state,
                last_error: serde_json::Value::Null,
                metadata: serde_json::json!({}),
            });
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
        let response_input = request.prompt.unwrap_or_else(|| {
            request
                .messages
                .last()
                .and_then(|message| message.get("content"))
                .and_then(|content| content.as_str())
                .unwrap_or_default()
                .to_string()
        });

        Ok(RuntimeLLMResponsePayload {
            task_id: request.task_id,
            model: request.model.or_else(|| Some("mock".to_string())),
            message: serde_json::json!({
                "content": format!("Mock response: {}", response_input)
            }),
            usage: serde_json::Value::Null,
            error: serde_json::Value::Null,
            correlation_id: request.correlation_id,
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
            task_id: request.task_id,
            capability: request.capability,
            result: Some(request.input),
            error: serde_json::Value::Null,
            correlation_id: request.correlation_id,
        })
    }
}

/// ThalamusRuntime: メインランタイム構造体
pub struct ThalamusRuntime<B: MessageBus> {
    bus: B,
    state: Arc<RwLock<RuntimeState>>,
    handlers: Arc<RwLock<HashMap<String, EventHandler>>>,
    task_handles: Arc<RwLock<Vec<TaskHandle>>>,
    worker_registry: Arc<RwLock<WorkerRegistry>>,
    task_states: Arc<RwLock<HashMap<String, TaskState>>>,
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
            worker_registry: Arc::new(RwLock::new(WorkerRegistry::default())),
            task_states: Arc::new(RwLock::new(HashMap::new())),
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
        let explicit_agent_id = event
            .payload
            .get("agent_id")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        let task_id = event
            .payload
            .get("task_id")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        let mut registry = self.worker_registry.write().await;
        let agent_id = explicit_agent_id.or_else(|| {
            if registry.workers.len() == 1 {
                registry.workers.keys().next().cloned()
            } else {
                registry.workers.iter().find_map(|(id, worker)| {
                    if worker.state != WorkerState::Ready {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
            }
        });
        if let Some(agent_id) = agent_id {
            registry.mark_error(agent_id, task_id, payload.error);
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
            refs: None,
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
            RUNTIME_AGENT_READY => self.update_agent_ready(event).await,
            RUNTIME_AGENT_EXIT => self.update_agent_exit(event).await,
            RUNTIME_AGENT_ERROR => self.update_agent_error(event).await,
            RUNTIME_TASK_ASSIGN => self.update_task_assign(event).await,
            RUNTIME_TASK_RESULT => self.update_task_result(event).await,
            RUNTIME_LLM_REQUEST => {
                let Ok(request) =
                    serde_json::from_value::<RuntimeLLMRequestPayload>(event.payload.clone())
                else {
                    return Ok(());
                };
                self.update_task_waiting_for_llm(&request).await;
                let payload_correlation_id = request.correlation_id.clone().or_else(|| {
                    event
                        .payload
                        .get("correlation_id")
                        .and_then(|v| v.as_str().map(String::from))
                });
                let response = MockLlmProvider.complete(request).await?;
                let payload = serde_json::to_value(&response)
                    .map_err(|e| RuntimeError::BusError(format!("serialize failed: {}", e)))?;
                let mut payload = payload;
                let request_event_id = event.id.clone();
                if let serde_json::Value::Object(object) = &mut payload {
                    let request_id = event
                        .payload
                        .get("request_id")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!(event.id.clone()));
                    object.insert("request_id".to_string(), request_id);
                    object.insert("status".to_string(), serde_json::json!("completed"));
                    let text = response
                        .message
                        .get("content")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .to_string();
                    object.insert("text".to_string(), serde_json::json!(text));
                    if let Some(ref corr_id) = payload_correlation_id {
                        object.insert("correlation_id".to_string(), serde_json::json!(corr_id));
                    }
                }
                let mut response_event = Self::envelope(
                    RUNTIME_LLM_RESPONSE.to_string(),
                    "thalamus-runtime".to_string(),
                    payload,
                );
                if let serde_json::Value::Object(object) = &mut response_event.payload {
                    if let Some(ref corr_id) = payload_correlation_id {
                        object.insert("correlation_id".to_string(), serde_json::json!(corr_id));
                    }
                }
                response_event.correlation_id = Some(request_event_id.clone());
                response_event.causation_id = Some(request_event_id);
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
                self.update_task_waiting_for_tool(&request).await;
                let payload_correlation_id = request.correlation_id.clone().or_else(|| {
                    event
                        .payload
                        .get("correlation_id")
                        .and_then(|v| v.as_str().map(String::from))
                });
                let result = EchoTool.invoke(request).await?;
                let payload = serde_json::to_value(&result)
                    .map_err(|e| RuntimeError::BusError(format!("serialize failed: {}", e)))?;
                let mut payload = payload;
                let request_event_id = event.id.clone();
                if let serde_json::Value::Object(object) = &mut payload {
                    let request_id = event
                        .payload
                        .get("request_id")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!(event.id.clone()));
                    object.insert("request_id".to_string(), request_id);
                    object.insert("status".to_string(), serde_json::json!("completed"));
                    object.insert("output".to_string(), serde_json::json!(result.result));
                }
                let mut result_event = Self::envelope(
                    RUNTIME_TOOL_RESULT.to_string(),
                    "thalamus-runtime".to_string(),
                    payload,
                );
                if let serde_json::Value::Object(object) = &mut result_event.payload {
                    if let Some(ref corr_id) = payload_correlation_id {
                        object.insert("correlation_id".to_string(), serde_json::json!(corr_id));
                    }
                }
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
