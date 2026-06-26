use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use thalamus_bus::{Handler, MessageBus, SubscriptionId};
use thalamus_protocol::payload::{
    RuntimeAgentErrorPayload, RuntimeAgentExitPayload, RuntimeAgentReadyPayload,
    RuntimeLLMRequestPayload, RuntimeLLMResponsePayload, RuntimeTaskAssignPayload,
    RuntimeTaskResultPayload, RuntimeToolRequestPayload, RuntimeToolResultPayload,
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
use crate::llm::LlmProvider;
use crate::registry::WorkerRegistry;
use crate::state::{RuntimeState, TaskHandle, TaskState, TaskStatus};
use crate::tool::ToolRegistry;

/// EventHandler: ユーザーイベントハンドラーの型定義
/// Takes (subject, envelope) — used by user-facing API.
pub type EventHandler = Arc<
    dyn Fn(String, EventEnvelope) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// RuntimeCore: 共有ランタイム状態
#[derive(Clone)]
pub struct RuntimeCore<B: MessageBus> {
    #[allow(dead_code)]
    bus: B,
    worker_registry: Arc<RwLock<WorkerRegistry>>,
    task_states: Arc<RwLock<HashMap<String, TaskState>>>,
    llm_provider: Arc<RwLock<Arc<dyn LlmProvider>>>,
    tool_registry: Arc<RwLock<ToolRegistry>>,
}

/// ThalamusRuntime: メインランタイム構造体
pub struct ThalamusRuntime<B: MessageBus> {
    bus: B,
    state: Arc<RwLock<RuntimeState>>,
    /// Internal subscriptions created by start() — used for cleanup in stop()
    internal_subscriptions: Arc<RwLock<Vec<SubscriptionId>>>,
    /// User-submitted subscriptions — also tracked for potential cleanup
    user_subscriptions: Arc<RwLock<Vec<SubscriptionId>>>,
    /// Task handles for tracking spawned futures
    task_handles: Arc<RwLock<Vec<TaskHandle>>>,
    core: RuntimeCore<B>,
}

impl<B: MessageBus + 'static> fmt::Debug for ThalamusRuntime<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThalamusRuntime")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl<B: MessageBus + 'static> ThalamusRuntime<B> {
    /// 新しいThalamusRuntimeインスタンスを作成する
    pub fn new(bus: B, llm_provider: Arc<dyn LlmProvider>) -> Self {
        let mut registry = ToolRegistry::new();
        registry.register("tool.echo".to_string(), Arc::new(crate::tool::EchoTool));
        // Register "echo" alias for backwards compatibility
        registry.register_alias("echo".to_string(), "tool.echo".to_string());

        Self {
            bus: bus.clone(),
            state: Arc::new(RwLock::new(RuntimeState::Initialized)),
            internal_subscriptions: Arc::new(RwLock::new(Vec::new())),
            user_subscriptions: Arc::new(RwLock::new(Vec::new())),
            task_handles: Arc::new(RwLock::new(Vec::new())),
            core: RuntimeCore {
                bus,
                worker_registry: Arc::new(RwLock::new(WorkerRegistry::default())),
                task_states: Arc::new(RwLock::new(HashMap::new())),
                llm_provider: Arc::new(RwLock::new(llm_provider)),
                tool_registry: Arc::new(RwLock::new(registry)),
            },
        }
    }

    /// 現在の状態を取得する
    pub async fn state(&self) -> RuntimeState {
        self.state.read().await.clone()
    }

    /// ユーザーイベントハンドラーを登録する（busにもsubscribe）
    ///
    /// Multiple user handlers can be registered for the same subject.
    pub async fn register_handler(
        &self,
        subject: String,
        handler: EventHandler,
    ) -> Result<SubscriptionId, RuntimeError> {
        // Convert EventHandler to Handler for bus.subscribe()
        let subject_for_closure = subject.clone();
        let bus_handler: Handler = Arc::new(move |envelope| {
            let user_handler = Arc::clone(&handler);
            let subj = subject_for_closure.clone();
            Box::pin(async move {
                user_handler(subj, envelope).await;
            })
        });
        let id = self
            .bus
            .subscribe(subject, bus_handler)
            .await
            .map_err(|e| RuntimeError::BusError(format!("subscribe: {}", e)))?;
        let mut subs = self.user_subscriptions.write().await;
        subs.push(id.clone());
        Ok(id)
    }

    /// ユーザーイベントハンドラーを削除する（busからもunsubscribe）
    pub async fn unregister_handler(&self, id: SubscriptionId) -> Result<(), RuntimeError> {
        self.bus
            .unsubscribe(id.clone())
            .await
            .map_err(|e| RuntimeError::BusError(format!("unsubscribe: {}", e)))?;
        let mut subs = self.user_subscriptions.write().await;
        subs.retain(|s| s != &id);
        Ok(())
    }

    /// ユーザーハンドラー数を返す
    pub async fn user_handler_count(&self) -> usize {
        self.user_subscriptions.read().await.len()
    }

    /// タスクをspawnする
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
            task_handles.retain(|h| h.id != id);
        });

        handle
    }

    /// 実行中のタスク数を取得する
    pub async fn active_task_count(&self) -> usize {
        self.task_handles.read().await.len()
    }

    pub async fn worker_registry(&self) -> WorkerRegistry {
        self.core.worker_registry.read().await.clone()
    }

    pub async fn task_state(&self, id: &str) -> Option<TaskState> {
        self.core.task_states.read().await.get(id).cloned()
    }

    /// ツールを登録する
    pub async fn register_tool(&self, capability: String, tool: Arc<dyn crate::tool::Tool>) {
        let mut registry = self.core.tool_registry.write().await;
        registry.register(capability, tool);
    }

    /// ツールにエイリアスを登録する
    pub async fn register_tool_alias(&self, alias: String, target: String) -> bool {
        let mut registry = self.core.tool_registry.write().await;
        registry.register_alias(alias, target)
    }

    /// ツールを削除する
    pub async fn unregister_tool(&self, capability: &str) -> bool {
        let mut registry = self.core.tool_registry.write().await;
        registry.unregister(capability).is_some()
    }

    /// 登録済みツールの一覧を返す（ソート済み）
    pub async fn list_tool_capabilities(&self) -> Vec<String> {
        let registry = self.core.tool_registry.read().await;
        registry.list_capabilities()
    }

    /// LLMプロバイダーを設定し直す
    pub async fn set_llm_provider(&self, provider: Arc<dyn LlmProvider>) {
        *self.core.llm_provider.write().await = provider;
    }

    /// Create an envelope with a pre-generated event_id
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

    /// ランタイムを起動する
    ///
    /// Registers internal handlers for all canonical subjects.
    /// Calling start() while already running returns LifecycleError.
    pub async fn start(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.write().await;

        if *state == RuntimeState::Running {
            return Err(RuntimeError::LifecycleError(
                "already running, cannot start again".to_string(),
            ));
        }

        *state = RuntimeState::Starting;

        // Register internal handlers
        let core = self.core.clone();
        let bus = self.bus.clone();

        // runtime.agent.spawn — observation only in MVP (do NOT re-publish)
        {
            let _core = core.clone();
            let _b = bus.clone();
            let spawn_handler: Handler = Arc::new(move |_envelope| {
                Box::pin(async move {
                    // Observation only — no processing, no re-publish
                })
            });
            let spawn_sub = bus
                .subscribe(RUNTIME_AGENT_SPAWN.to_string(), spawn_handler)
                .await
                .map_err(|e| RuntimeError::BusError(format!("subscribe spawn: {}", e)))?;
            self.internal_subscriptions.write().await.push(spawn_sub);
        }

        // runtime.agent.ready — process only (do NOT re-publish)
        {
            let core = core.clone();
            let b = bus.clone();
            let ready_handler: Handler = Arc::new(move |envelope| {
                let core = core.clone();
                let _b = b.clone();
                Box::pin(async move {
                    if let Ok(payload) =
                        serde_json::from_value::<RuntimeAgentReadyPayload>(envelope.payload.clone())
                    {
                        core.worker_registry
                            .write()
                            .await
                            .mark_ready(payload.agent_id, payload.capabilities);
                    }
                })
            });
            let ready_sub = bus
                .subscribe(RUNTIME_AGENT_READY.to_string(), ready_handler)
                .await
                .map_err(|e| RuntimeError::BusError(format!("subscribe ready: {}", e)))?;
            self.internal_subscriptions.write().await.push(ready_sub);
        }

        // runtime.agent.exit — process only (do NOT re-publish)
        {
            let core = core.clone();
            let b = bus.clone();
            let exit_handler: Handler = Arc::new(move |envelope| {
                let core = core.clone();
                let _b = b.clone();
                Box::pin(async move {
                    if let Ok(payload) =
                        serde_json::from_value::<RuntimeAgentExitPayload>(envelope.payload.clone())
                    {
                        core.worker_registry
                            .write()
                            .await
                            .mark_exited(payload.agent_id, payload.reason);
                    }
                })
            });
            let exit_sub = bus
                .subscribe(RUNTIME_AGENT_EXIT.to_string(), exit_handler)
                .await
                .map_err(|e| RuntimeError::BusError(format!("subscribe exit: {}", e)))?;
            self.internal_subscriptions.write().await.push(exit_sub);
        }

        // runtime.agent.error — process only (do NOT re-publish)
        {
            let core = core.clone();
            let b = bus.clone();
            let error_handler: Handler = Arc::new(move |envelope| {
                let core = core.clone();
                let _b = b.clone();
                Box::pin(async move {
                    if let Ok(payload) =
                        serde_json::from_value::<RuntimeAgentErrorPayload>(envelope.payload.clone())
                    {
                        // Only mark_error if agent_id is present
                        if let Some(agent_id) = payload.agent_id {
                            core.worker_registry.write().await.mark_error(
                                agent_id,
                                payload.task_id,
                                payload.error,
                            );
                        }
                    }
                })
            });
            let error_sub = bus
                .subscribe(RUNTIME_AGENT_ERROR.to_string(), error_handler)
                .await
                .map_err(|e| RuntimeError::BusError(format!("subscribe error: {}", e)))?;
            self.internal_subscriptions.write().await.push(error_sub);
        }

        // runtime.task.assign — process only (do NOT re-publish)
        {
            let core = core.clone();
            let b = bus.clone();
            let task_assign_handler: Handler = Arc::new(move |envelope| {
                let core = core.clone();
                let _b = b.clone();
                Box::pin(async move {
                    if let Ok(payload) =
                        serde_json::from_value::<RuntimeTaskAssignPayload>(envelope.payload.clone())
                    {
                        let task = {
                            let mut states = core.task_states.write().await;
                            states
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
                })
            });
            let task_assign_sub = bus
                .subscribe(RUNTIME_TASK_ASSIGN.to_string(), task_assign_handler)
                .await
                .map_err(|e| RuntimeError::BusError(format!("subscribe task.assign: {}", e)))?;
            self.internal_subscriptions
                .write()
                .await
                .push(task_assign_sub);
        }

        // runtime.task.result — process only (do NOT re-publish)
        {
            let core = core.clone();
            let b = bus.clone();
            let task_result_handler: Handler = Arc::new(move |envelope| {
                let core = core.clone();
                let _b = b.clone();
                Box::pin(async move {
                    if let Ok(payload) =
                        serde_json::from_value::<RuntimeTaskResultPayload>(envelope.payload.clone())
                    {
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
                            let mut states = core.task_states.write().await;
                            states
                                .entry(payload.task_id.clone())
                                .or_insert_with(|| TaskState::new(payload.task_id.clone()))
                                .clone()
                        };
                        *task.result.write().await = payload.result;
                        *task.error.write().await = payload.error;
                        *task.correlation_id.write().await = payload.correlation_id;
                        task.set_task_status(status).await;
                    }
                })
            });
            let task_result_sub = bus
                .subscribe(RUNTIME_TASK_RESULT.to_string(), task_result_handler)
                .await
                .map_err(|e| RuntimeError::BusError(format!("subscribe task.result: {}", e)))?;
            self.internal_subscriptions
                .write()
                .await
                .push(task_result_sub);
        }

        // runtime.llm.request — process, call provider, publish response (do NOT re-publish request)
        {
            let core = core.clone();
            let b = bus.clone();
            let llm_request_handler: Handler = Arc::new(move |envelope| {
                let core = core.clone();
                let b = b.clone();
                Box::pin(async move {
                    // Parse request
                    let request = match serde_json::from_value::<RuntimeLLMRequestPayload>(
                        envelope.payload.clone(),
                    ) {
                        Ok(r) => r,
                        Err(_) => {
                            // Parse error — drop envelope, do not re-publish
                            return;
                        }
                    };

                    // Keep a reference to the original envelope for correlation
                    let request_event = envelope.clone();

                    // Update task state
                    let task = {
                        let mut states = core.task_states.write().await;
                        states
                            .entry(request.task_id.clone())
                            .or_insert_with(|| TaskState::new(request.task_id.clone()))
                            .clone()
                    };
                    *task.correlation_id.write().await = request.correlation_id.clone();
                    task.set_task_status(TaskStatus::WaitingForLlm).await;

                    // Call provider — read fresh provider from core
                    let provider = { core.llm_provider.read().await.clone() };
                    let response_result = provider.complete(request.clone()).await;

                    match response_result {
                        Ok(response) => {
                            let response_event = match core
                                .generate_llm_response_envelope(&request_event, &request, response)
                                .await
                            {
                                Ok(e) => e,
                                Err(_) => {
                                    // Envelope generation failed — drop, do not re-publish
                                    return;
                                }
                            };
                            let _ = b.publish(response_event).await;
                        }
                        Err(err) => {
                            // Provider error — publish error response, do not fail the request
                            let error_response = RuntimeLLMResponsePayload {
                                task_id: request.task_id.clone(),
                                model: request.model.clone(),
                                request_id: request.request_id.clone(),
                                status: "error".to_string(),
                                text: None,
                                message: serde_json::json!({}),
                                usage: serde_json::Value::Null,
                                error: serde_json::json!({
                                    "kind": "provider_error",
                                    "message": err.to_string()
                                }),
                                correlation_id: request.correlation_id.clone(),
                            };
                            let error_event = match core
                                .generate_llm_response_envelope(
                                    &request_event,
                                    &request,
                                    error_response,
                                )
                                .await
                            {
                                Ok(e) => e,
                                Err(_) => {
                                    // Envelope generation failed — drop, do not re-publish
                                    return;
                                }
                            };
                            let _ = b.publish(error_event).await;
                        }
                    }
                })
            });
            let llm_request_sub = bus
                .subscribe(RUNTIME_LLM_REQUEST.to_string(), llm_request_handler)
                .await
                .map_err(|e| RuntimeError::BusError(format!("subscribe llm.request: {}", e)))?;
            self.internal_subscriptions
                .write()
                .await
                .push(llm_request_sub);
        }

        // runtime.tool.request — process, call tool, publish result (do NOT re-publish request)
        {
            let core = core.clone();
            let b = bus.clone();
            let tool_request_handler: Handler = Arc::new(move |envelope| {
                let core = core.clone();
                let b = b.clone();
                Box::pin(async move {
                    let request = match serde_json::from_value::<RuntimeToolRequestPayload>(
                        envelope.payload.clone(),
                    ) {
                        Ok(r) => r,
                        Err(_) => {
                            // Parse error — drop envelope, do not re-publish
                            return;
                        }
                    };

                    // Keep a reference to the original envelope for correlation
                    let request_event = envelope.clone();

                    // Update task state
                    let task = {
                        let mut states = core.task_states.write().await;
                        states
                            .entry(request.task_id.clone())
                            .or_insert_with(|| TaskState::new(request.task_id.clone()))
                            .clone()
                    };
                    *task.correlation_id.write().await = request.correlation_id.clone();
                    task.set_task_status(TaskStatus::WaitingForTool).await;

                    // Look up tool
                    let tool_registry = core.tool_registry.read().await;
                    let tool = tool_registry.get(&request.capability);

                    match tool {
                        Some(t) => {
                            // Release tool_registry lock before invoking
                            drop(tool_registry);
                            let result = t.invoke(request.clone()).await;
                            match result {
                                Ok(result_payload) => {
                                    let result_event = match core
                                        .generate_tool_result_envelope(
                                            &request_event,
                                            &request,
                                            result_payload,
                                        )
                                        .await
                                    {
                                        Ok(e) => e,
                                        Err(_) => {
                                            let _ = b.publish(request_event).await;
                                            return;
                                        }
                                    };
                                    let _ = b.publish(result_event).await;
                                }
                                Err(err) => {
                                    let result_event = match core
                                        .generate_tool_result_envelope_error(
                                            &request_event,
                                            &request,
                                            "tool_error",
                                            &err.to_string(),
                                        )
                                        .await
                                    {
                                        Ok(e) => e,
                                        Err(_) => {
                                            let _ = b.publish(request_event).await;
                                            return;
                                        }
                                    };
                                    let _ = b.publish(result_event).await;
                                }
                            }
                        }
                        None => {
                            // Unknown tool — publish error result with actual capability name
                            drop(tool_registry);
                            let result_event = match core
                                .generate_tool_result_envelope_error(
                                    &request_event,
                                    &request,
                                    "capability_not_found",
                                    &format!("tool not found: {}", request.capability),
                                )
                                .await
                            {
                                Ok(e) => e,
                                Err(_) => {
                                    let _ = b.publish(request_event).await;
                                    return;
                                }
                            };
                            let _ = b.publish(result_event).await;
                        }
                    }
                })
            });
            let tool_request_sub = bus
                .subscribe(RUNTIME_TOOL_REQUEST.to_string(), tool_request_handler)
                .await
                .map_err(|e| RuntimeError::BusError(format!("subscribe tool.request: {}", e)))?;
            self.internal_subscriptions
                .write()
                .await
                .push(tool_request_sub);
        }

        *state = RuntimeState::Running;
        Ok(())
    }

    /// ランタイムを停止する
    ///
    /// Unsubscribes internal handlers, clears subscriptions, closes the bus, and transitions to Stopped.
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

        // Unsubscribe internal handlers and clear the vec
        {
            let mut subs = self.internal_subscriptions.write().await;
            for sub_id in subs.iter() {
                let _ = self.bus.unsubscribe(sub_id.clone()).await;
            }
            subs.clear();
        }

        // Close the bus
        self.bus.close().await;

        *state = RuntimeState::Stopped;
        Ok(())
    }

    /// イベントを公開する
    ///
    /// This is a pure event entrance:
    /// 1. Generate event_id
    /// 2. Validate and normalize payload
    /// 3. Create EventEnvelope
    /// 4. Publish to bus
    /// 5. Return envelope
    ///
    /// It does NOT directly process the event or update state.
    pub async fn publish(
        &self,
        subject: String,
        source: String,
        payload: serde_json::Value,
    ) -> Result<EventEnvelope, RuntimeError> {
        // Generate event_id
        let event_id = Uuid::new_v4().to_string();

        // Validate and normalize payload
        let normalized = thalamus_protocol::validation::validate_and_normalize_payload(
            &subject, &event_id, payload,
        )
        .map_err(|e| RuntimeError::InvalidPayload(format!("{}: {}", e.subject, e.reason)))?;

        let envelope = Self::envelope_with_id(subject.clone(), source, event_id, normalized);

        // Publish to bus — internal handlers and user handlers will process it
        self.bus
            .publish(envelope.clone())
            .await
            .map_err(|e| RuntimeError::BusError(format!("publish failed: {}", e)))?;

        Ok(envelope)
    }

    /// イベントを処理する（非推奨 — publish() + bus handler を使用してください）
    pub async fn handle_event(
        &self,
        subject: String,
        event: EventEnvelope,
    ) -> Result<(), RuntimeError> {
        let _ = subject;
        let _ = event;
        Err(RuntimeError::ScheduleError(
            "use publish() instead".to_string(),
        ))
    }
}

impl<B: MessageBus + 'static> RuntimeCore<B> {
    /// Generate an llm.response envelope with correct correlation/causation semantics.
    ///
    /// envelope.causation_id = request_event.id
    /// envelope.correlation_id = request_event.correlation_id
    ///     .or_else(|| request_payload.correlation_id.clone())
    ///     .or_else(|| Some(request_event.id.clone()))
    async fn generate_llm_response_envelope(
        &self,
        request_event: &EventEnvelope,
        request_payload: &RuntimeLLMRequestPayload,
        response: RuntimeLLMResponsePayload,
    ) -> Result<EventEnvelope, RuntimeError> {
        let payload = serde_json::to_value(&response)
            .map_err(|e| RuntimeError::Internal(format!("serialize llm response: {}", e)))?;
        let mut event = ThalamusRuntime::<B>::envelope(
            RUNTIME_LLM_RESPONSE.to_string(),
            "thalamus-runtime".to_string(),
            payload,
        );
        event.causation_id = Some(request_event.id.clone());
        event.correlation_id = request_event
            .correlation_id
            .clone()
            .or_else(|| request_payload.correlation_id.clone())
            .or_else(|| Some(request_event.id.clone()));
        Ok(event)
    }

    /// Generate a tool.result envelope with correct correlation/causation semantics.
    async fn generate_tool_result_envelope(
        &self,
        request_event: &EventEnvelope,
        request_payload: &RuntimeToolRequestPayload,
        result_payload: RuntimeToolResultPayload,
    ) -> Result<EventEnvelope, RuntimeError> {
        let payload = serde_json::to_value(&result_payload)
            .map_err(|e| RuntimeError::Internal(format!("serialize tool result: {}", e)))?;
        let mut event = ThalamusRuntime::<B>::envelope(
            RUNTIME_TOOL_RESULT.to_string(),
            "thalamus-runtime".to_string(),
            payload,
        );
        event.causation_id = Some(request_event.id.clone());
        event.correlation_id = request_event
            .correlation_id
            .clone()
            .or_else(|| request_payload.correlation_id.clone())
            .or_else(|| Some(request_event.id.clone()));
        Ok(event)
    }

    /// Generate a tool.result error envelope.
    async fn generate_tool_result_envelope_error(
        &self,
        request_event: &EventEnvelope,
        request_payload: &RuntimeToolRequestPayload,
        error_kind: &str,
        error_message: &str,
    ) -> Result<EventEnvelope, RuntimeError> {
        let result_payload = RuntimeToolResultPayload {
            task_id: request_payload.task_id.clone(),
            capability: request_payload.capability.clone(),
            request_id: request_payload.request_id.clone(),
            status: "error".to_string(),
            output: None,
            result: None,
            error: serde_json::json!({
                "kind": error_kind,
                "message": error_message,
            }),
            correlation_id: request_payload.correlation_id.clone(),
        };
        let payload = serde_json::to_value(&result_payload)
            .map_err(|e| RuntimeError::Internal(format!("serialize tool result: {}", e)))?;
        let mut event = ThalamusRuntime::<B>::envelope(
            RUNTIME_TOOL_RESULT.to_string(),
            "thalamus-runtime".to_string(),
            payload,
        );
        event.causation_id = Some(request_event.id.clone());
        event.correlation_id = request_event
            .correlation_id
            .clone()
            .or_else(|| request_payload.correlation_id.clone())
            .or_else(|| Some(request_event.id.clone()));
        Ok(event)
    }
}
