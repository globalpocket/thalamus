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
pub type EventHandler = Arc<
    dyn Fn(String, EventEnvelope) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// RuntimeCore: 共有ランタイム状態
///
/// All internal state is wrapped in `Arc` so that internal handlers
/// (registered in `start()`) can capture a single clone and access
/// worker_registry, task_states, llm_provider, tool_registry, and bus.
#[derive(Clone)]
pub struct RuntimeCore<B: MessageBus> {
    bus: B,
    worker_registry: Arc<RwLock<WorkerRegistry>>,
    task_states: Arc<RwLock<HashMap<String, TaskState>>>,
    llm_provider: Arc<dyn LlmProvider>,
    tool_registry: Arc<RwLock<ToolRegistry>>,
}

/// ThalamusRuntime: メインランタイム構造体
pub struct ThalamusRuntime<B: MessageBus> {
    bus: B,
    state: Arc<RwLock<RuntimeState>>,
    /// User-registered handlers (not auto-registered by start())
    user_handlers: Arc<RwLock<HashMap<String, EventHandler>>>,
    task_handles: Arc<RwLock<Vec<TaskHandle>>>,
    /// Internal subscriptions created by start() — used for cleanup in stop()
    internal_subscriptions: Arc<RwLock<Vec<SubscriptionId>>>,
    /// User-submitted subscriptions — also tracked for potential cleanup
    user_subscriptions: Arc<RwLock<Vec<SubscriptionId>>>,
    llm_provider: Arc<RwLock<Arc<dyn LlmProvider>>>,
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
        registry.register("tool.echo".to_string(), Box::new(crate::tool::EchoTool));
        // Register "echo" alias for backwards compatibility
        registry.register_alias("echo".to_string(), "tool.echo".to_string());

        Self {
            bus: bus.clone(),
            state: Arc::new(RwLock::new(RuntimeState::Initialized)),
            user_handlers: Arc::new(RwLock::new(HashMap::new())),
            task_handles: Arc::new(RwLock::new(Vec::new())),
            internal_subscriptions: Arc::new(RwLock::new(Vec::new())),
            user_subscriptions: Arc::new(RwLock::new(Vec::new())),
            llm_provider: Arc::new(RwLock::new(llm_provider)),
            core: RuntimeCore {
                bus,
                worker_registry: Arc::new(RwLock::new(WorkerRegistry::default())),
                task_states: Arc::new(RwLock::new(HashMap::new())),
                llm_provider,
                tool_registry: Arc::new(RwLock::new(registry)),
            },
        }
    }

    /// 現在の状態を取得する
    pub async fn state(&self) -> RuntimeState {
        *self.state.read().await
    }

    /// ユーザーイベントハンドラーを登録する（busにもsubscribe）
    pub async fn register_handler(
        &self,
        subject: String,
        handler: EventHandler,
    ) -> Result<SubscriptionId, RuntimeError> {
        let id = self.bus.subscribe(subject.clone(), handler.clone()).await?;
        let mut handlers = self.user_handlers.write().await;
        handlers.insert(subject, handler);
        let mut subs = self.user_subscriptions.write().await;
        subs.push(id);
        Ok(id)
    }

    /// ユーザーイベントハンドラーを削除する（busからもunsubscribe）
    pub async fn unregister_handler(&self, id: SubscriptionId) -> Result<(), RuntimeError> {
        self.bus.unsubscribe(id.clone()).await?;
        let mut subs = self.user_subscriptions.write().await;
        subs.retain(|s| s != &id);
        // Note: we cannot remove from user_handlers by id since it's keyed by subject
        Ok(())
    }

    /// ユーザーハンドラー数を返す
    pub async fn user_handler_count(&self) -> usize {
        self.user_handlers.read().await.len()
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
    pub async fn register_tool(&self, capability: String, tool: Box<dyn crate::tool::Tool>) {
        let mut registry = self.core.tool_registry.write().await;
        registry.register(capability, tool);
    }

    /// ツールにエイリアスを登録する
    pub async fn register_tool_alias(&self, alias: String, target: String) {
        let mut registry = self.core.tool_registry.write().await;
        registry.register_alias(alias, target);
    }

    /// ツールを削除する
    pub async fn unregister_tool(&self, capability: &str) {
        let mut registry = self.core.tool_registry.write().await;
        registry.unregister(capability);
    }

    /// 登録済みツールの一覧を返す（ソート済み）
    pub async fn list_tool_capabilities(&self) -> Vec<String> {
        let registry = self.core.tool_registry.read().await;
        registry.list_capabilities()
    }

    /// LLMプロバイダーを設定し直す
    pub async fn set_llm_provider(&self, provider: Arc<dyn LlmProvider>) {
        let mut lp = self.llm_provider.write().await;
        *lp = provider;
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

    /// Generate a response envelope for llm.response
    async fn generate_llm_response(
        &self,
        request: RuntimeLLMRequestPayload,
        response: RuntimeLLMResponsePayload,
    ) -> Result<EventEnvelope, RuntimeError> {
        let payload = serde_json::to_value(&response)
            .map_err(|e| RuntimeError::Internal(format!("serialize llm response: {}", e)))?;
        let mut event = Self::envelope(
            RUNTIME_LLM_RESPONSE.to_string(),
            "thalamus-runtime".to_string(),
            payload,
        );
        event.correlation_id = Some(request.task_id.clone());
        event.causation_id = Some(request.request_id.clone());
        Ok(event)
    }

    /// Generate a response envelope for tool.result
    async fn generate_tool_result(
        &self,
        request: RuntimeToolRequestPayload,
        status: &str,
        output: Option<serde_json::Value>,
        error: serde_json::Value,
    ) -> Result<EventEnvelope, RuntimeError> {
        let result = RuntimeToolResultPayload {
            task_id: request.task_id.clone(),
            capability: request.capability.clone(),
            request_id: request.request_id.clone(),
            status: status.to_string(),
            output: output.clone(),
            result: output,
            error,
            correlation_id: request.correlation_id.clone(),
        };
        let payload =
            serde_json::to_value(&result).map_err(|e| RuntimeError::Internal(format!("serialize tool result: {}", e)))?;
        let mut event = Self::envelope(
            RUNTIME_TOOL_RESULT.to_string(),
            "thalamus-runtime".to_string(),
            payload,
        );
        event.correlation_id = Some(request.task_id);
        event.causation_id = Some(request.request_id);
        Ok(event)
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
        let llm_provider_guard = self.llm_provider.read().await;
        let llm_provider = llm_provider_guard.clone();
        drop(llm_provider_guard);

        let internal_bus = bus.clone();
        let internal_subscriptions = self.internal_subscriptions.clone();

        // runtime.agent.spawn — observation only in MVP
        let spawn_handler: Handler = Arc::new(move |_envelope| {
            // Observation only — do nothing
            Box::pin(async move {})
        });
        let spawn_sub = internal_bus
            .subscribe(RUNTIME_AGENT_SPAWN.to_string(), spawn_handler)
            .await
            .map_err(|e| RuntimeError::BusError(format!("subscribe spawn: {}", e)))?;
        self.internal_subscriptions.write().await.push(spawn_sub);

        // runtime.agent.ready
        let ready_core = core.clone();
        let ready_bus = bus.clone();
        let ready_handler: Handler = Arc::new(move |envelope| {
            let core = ready_core.clone();
            let bus = ready_bus.clone();
            Box::pin(async move {
                if let Ok(payload) =
                    serde_json::from_value::<RuntimeAgentReadyPayload>(envelope.payload.clone())
                {
                    core.worker_registry
                        .write()
                        .await
                        .mark_ready(payload.agent_id, payload.capabilities);
                }
                // Forward to bus for user handlers
                let _ = bus.publish(envelope).await;
            })
        });
        let ready_sub = ready_bus
            .subscribe(RUNTIME_AGENT_READY.to_string(), ready_handler)
            .await
            .map_err(|e| RuntimeError::BusError(format!("subscribe ready: {}", e)))?;
        self.internal_subscriptions.write().await.push(ready_sub);

        // runtime.agent.exit
        let exit_core = core.clone();
        let exit_bus = bus.clone();
        let exit_handler: Handler = Arc::new(move |envelope| {
            let core = exit_core.clone();
            let bus = exit_bus.clone();
            Box::pin(async move {
                if let Ok(payload) =
                    serde_json::from_value::<RuntimeAgentExitPayload>(envelope.payload.clone())
                {
                    core.worker_registry
                        .write()
                        .await
                        .mark_exited(payload.agent_id, payload.reason);
                }
                let _ = bus.publish(envelope).await;
            })
        });
        let exit_sub = exit_bus
            .subscribe(RUNTIME_AGENT_EXIT.to_string(), exit_handler)
            .await
            .map_err(|e| RuntimeError::BusError(format!("subscribe exit: {}", e)))?;
        self.internal_subscriptions.write().await.push(exit_sub);

        // runtime.agent.error
        let error_core = core.clone();
        let error_bus = bus.clone();
        let error_handler: Handler = Arc::new(move |envelope| {
            let core = error_core.clone();
            let bus = error_bus.clone();
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
                let _ = bus.publish(envelope).await;
            })
        });
        let error_sub = error_bus
            .subscribe(RUNTIME_AGENT_ERROR.to_string(), error_handler)
            .await
            .map_err(|e| RuntimeError::BusError(format!("subscribe error: {}", e)))?;
        self.internal_subscriptions.write().await.push(error_sub);

        // runtime.task.assign
        let task_core = core.clone();
        let task_assign_bus = bus.clone();
        let task_assign_handler: Handler = Arc::new(move |envelope| {
            let core = task_core.clone();
            let bus = task_assign_bus.clone();
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
                let _ = bus.publish(envelope).await;
            })
        });
        let task_assign_sub = task_assign_bus
            .subscribe(RUNTIME_TASK_ASSIGN.to_string(), task_assign_handler)
            .await
            .map_err(|e| RuntimeError::BusError(format!("subscribe task.assign: {}", e)))?;
        self.internal_subscriptions.write().await.push(task_assign_sub);

        // runtime.task.result
        let task_result_core = core.clone();
        let task_result_bus = bus.clone();
        let task_result_handler: Handler = Arc::new(move |envelope| {
            let core = task_result_core.clone();
            let bus = task_result_bus.clone();
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
                let _ = bus.publish(envelope).await;
            })
        });
        let task_result_sub = task_result_bus
            .subscribe(RUNTIME_TASK_RESULT.to_string(), task_result_handler)
            .await
            .map_err(|e| RuntimeError::BusError(format!("subscribe task.result: {}", e)))?;
        self.internal_subscriptions.write().await.push(task_result_sub);

        // runtime.llm.request
        let llm_core = core.clone();
        let llm_bus = bus.clone();
        let llm_provider_for_handler = llm_provider.clone();
        let llm_request_handler: Handler = Arc::new(move |envelope| {
            let core = llm_core.clone();
            let b = llm_bus.clone();
            let provider = llm_provider_for_handler.clone();
            Box::pin(async move {
                // Parse request
                let request =
                    match serde_json::from_value::<RuntimeLLMRequestPayload>(envelope.payload.clone())
                    {
                        Ok(r) => r,
                        Err(_) => {
                            let _ = b.publish(envelope).await;
                            return;
                        }
                    };

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

                // Call provider exactly once
                let response_result = provider.complete(request.clone()).await;

                match response_result {
                    Ok(response) => {
                        let response_event = match core
                            .generate_llm_response_envelope(request, response)
                            .await
                        {
                            Ok(e) => e,
                            Err(_) => {
                                let _ = b.publish(envelope).await;
                                return;
                            }
                        };
                        let _ = b.publish(response_event).await;
                    }
                    Err(_) => {
                        // Provider error — publish error response, do not fail the request
                        let error_response = RuntimeLLMResponsePayload {
                            task_id: request.task_id.clone(),
                            model: request.model,
                            request_id: request.request_id.clone(),
                            status: "error".to_string(),
                            text: None,
                            message: serde_json::json!({}),
                            usage: serde_json::Value::Null,
                            error: serde_json::json!({
                                "kind": "provider_error",
                                "message": "provider returned error"
                            }),
                            correlation_id: request.correlation_id,
                        };
                        let error_event = match core
                            .generate_llm_response_envelope(request, error_response)
                            .await
                        {
                            Ok(e) => e,
                            Err(_) => {
                                let _ = b.publish(envelope).await;
                                return;
                            }
                        };
                        let _ = b.publish(error_event).await;
                    }
                }
            })
        });
        let llm_request_sub = llm_bus
            .subscribe(RUNTIME_LLM_REQUEST.to_string(), llm_request_handler)
            .await
            .map_err(|e| RuntimeError::BusError(format!("subscribe llm.request: {}", e)))?;
        self.internal_subscriptions.write().await.push(llm_request_sub);

        // runtime.tool.request
        let tool_core = core.clone();
        let tool_bus = bus.clone();
        let tool_request_handler: Handler = Arc::new(move |envelope| {
            let core = tool_core.clone();
            let b = tool_bus.clone();
            Box::pin(async move {
                let request =
                    match serde_json::from_value::<RuntimeToolRequestPayload>(envelope.payload.clone())
                    {
                        Ok(r) => r,
                        Err(_) => {
                            let _ = b.publish(envelope).await;
                            return;
                        }
                    };

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
                        let result = t.invoke(request.input.clone()).await;
                        match result {
                            Ok(output) => {
                                let result_event = match core
                                    .generate_tool_result_envelope(
                                        request,
                                        "completed",
                                        Some(output),
                                        serde_json::json!({}),
                                    )
                                    .await
                                {
                                    Ok(e) => e,
                                    Err(_) => {
                                        let _ = b.publish(envelope).await;
                                        return;
                                    }
                                };
                                let _ = b.publish(result_event).await;
                            }
                            Err(_) => {
                                let result_event = match core
                                    .generate_tool_result_envelope(
                                        request,
                                        "error",
                                        None,
                                        serde_json::json!({
                                            "kind": "tool_error",
                                            "message": "tool invocation failed"
                                        }),
                                    )
                                    .await
                                {
                                    Ok(e) => e,
                                    Err(_) => {
                                        let _ = b.publish(envelope).await;
                                        return;
                                    }
                                };
                                let _ = b.publish(result_event).await;
                            }
                        }
                    }
                    None => {
                        // Unknown tool — publish error result
                        drop(tool_registry);
                        let result_event = match core
                            .generate_tool_result_envelope(
                                request,
                                "error",
                                None,
                                serde_json::json!({
                                    "kind": "unknown_tool",
                                    "message": format!("tool not found: {}", "unknown")
                                }),
                            )
                            .await
                        {
                            Ok(e) => e,
                            Err(_) => {
                                let _ = b.publish(envelope).await;
                                return;
                            }
                        };
                        let _ = b.publish(result_event).await;
                    }
                }
            })
        });
        let tool_request_sub = tool_bus
            .subscribe(RUNTIME_TOOL_REQUEST.to_string(), tool_request_handler)
            .await
            .map_err(|e| RuntimeError::BusError(format!("subscribe tool.request: {}", e)))?;
        self.internal_subscriptions.write().await.push(tool_request_sub);

        *state = RuntimeState::Running;
        Ok(())
    }

    /// ランタイムを停止する
    ///
    /// Unsubscribes internal handlers, closes the bus, and transitions to Stopped.
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

        // Unsubscribe internal handlers
        {
            let subs = self.internal_subscriptions.write().await;
            for sub_id in subs.iter() {
                let _ = self.bus.unsubscribe(sub_id.clone()).await;
            }
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
        let normalized =
            thalamus_protocol::validation::validate_and_normalize_payload(
                &subject,
                &event_id,
                payload,
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
        let handlers = self.user_handlers.read().await;

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

impl<B: MessageBus + 'static> RuntimeCore<B> {
    /// Generate an llm.response envelope
    async fn generate_llm_response_envelope(
        &self,
        request: RuntimeLLMRequestPayload,
        response: RuntimeLLMResponsePayload,
    ) -> Result<EventEnvelope, RuntimeError> {
        let payload = serde_json::to_value(&response)
            .map_err(|e| RuntimeError::Internal(format!("serialize llm response: {}", e)))?;
        let mut event = ThalamusRuntime::<B>::envelope(
            RUNTIME_LLM_RESPONSE.to_string(),
            "thalamus-runtime".to_string(),
            payload,
        );
        event.correlation_id = Some(request.task_id);
        event.causation_id = Some(request.request_id);
        Ok(event)
    }

    /// Generate a tool.result envelope
    async fn generate_tool_result_envelope(
        &self,
        request: RuntimeToolRequestPayload,
        status: &str,
        output: Option<serde_json::Value>,
        error: serde_json::Value,
    ) -> Result<EventEnvelope, RuntimeError> {
        let result = RuntimeToolResultPayload {
            task_id: request.task_id.clone(),
            capability: request.capability.clone(),
            request_id: request.request_id.clone(),
            status: status.to_string(),
            output: output.clone(),
            result: output,
            error,
            correlation_id: request.correlation_id.clone(),
        };
        let payload = serde_json::to_value(&result)
            .map_err(|e| RuntimeError::Internal(format!("serialize tool result: {}", e)))?;
        let mut event = ThalamusRuntime::<B>::envelope(
            RUNTIME_TOOL_RESULT.to_string(),
            "thalamus-runtime".to_string(),
            payload,
        );
        event.correlation_id = Some(request.task_id);
        event.causation_id = Some(request.request_id);
        Ok(event)
    }
}
