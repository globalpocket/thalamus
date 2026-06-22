use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};
use thalamus_bus::BasicBus;
use thalamus_protocol::{
    payload::{
        RuntimeAgentErrorPayload, RuntimeAgentExitPayload, RuntimeAgentReadyPayload,
        RuntimeLLMRequestPayload, RuntimeTaskAssignPayload, RuntimeTaskResultPayload,
        RuntimeToolRequestPayload,
    },
    subject::{
        RUNTIME_AGENT_ERROR, RUNTIME_AGENT_EXIT, RUNTIME_AGENT_READY, RUNTIME_AGENT_SPAWN,
        RUNTIME_LLM_REQUEST, RUNTIME_LLM_RESPONSE, RUNTIME_TASK_ASSIGN, RUNTIME_TASK_RESULT,
        RUNTIME_TOOL_REQUEST, RUNTIME_TOOL_RESULT,
    },
    EventEnvelope,
};
use thalamus_runtime::llm::LlmProvider;
use thalamus_runtime::Tool;
use thalamus_runtime::{
    EchoTool, MockLlmProvider, RuntimeError, RuntimeState, TaskState, ThalamusRuntime,
    WorkerRegistry,
};

fn runtime_basic_bus(runtime: &ThalamusRuntime<BasicBus>) -> &BasicBus {
    // SAFETY: このテストはThalamusRuntimeが所有するBasicBusの公開済みイベントだけを観測する。
    // ThalamusRuntimeの先頭フィールドはBasicBusであり、可変参照は作らず借用中にruntimeを移動しない。
    unsafe { &*(runtime as *const ThalamusRuntime<BasicBus> as *const BasicBus) }
}

fn new_runtime(bus: BasicBus) -> ThalamusRuntime<BasicBus> {
    ThalamusRuntime::new(bus, Arc::new(MockLlmProvider))
}

#[tokio::test]
async fn contract_new_runtime_starts_in_initialized_state() {
    let runtime = new_runtime(BasicBus::new());

    assert_eq!(runtime.state().await, RuntimeState::Initialized);
}

#[tokio::test]
async fn behavior_start_stop_transitions_runtime_state() {
    let mut runtime = new_runtime(BasicBus::new());

    runtime.start().await.expect("runtime should start");
    assert_eq!(runtime.state().await, RuntimeState::Running);

    runtime.stop().await.expect("runtime should stop");
    assert_eq!(runtime.state().await, RuntimeState::Stopped);
}

#[tokio::test]
async fn behavior_publish_and_handle_event_dispatch_registered_handler() {
    let subject = "unit.runtime.event".to_string();
    let counter = Arc::new(AtomicUsize::new(0));
    let handler_counter = Arc::clone(&counter);
    let mut runtime = new_runtime(BasicBus::new());
    let handler: thalamus_runtime::EventHandler = Arc::new(move |_subject, _event| {
        let counter = Arc::clone(&handler_counter);
        Box::pin(async move {
            counter.fetch_add(1, Ordering::SeqCst);
        })
    });

    runtime.register_handler(subject.clone(), handler).await;
    runtime
        .start()
        .await
        .expect("runtime should start with registered handler");

    let envelope = runtime
        .publish(
            subject.clone(),
            "runtime-basic-test".to_string(),
            serde_json::json!({ "contract": "publish" }),
        )
        .await
        .expect("publish should return delivered envelope");

    assert_eq!(envelope.subject, subject);
    assert_eq!(envelope.source, "runtime-basic-test");
    assert_eq!(envelope.schema, "thalamus.unit.runtime.event");
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    runtime
        .handle_event(subject, envelope)
        .await
        .expect("handle_event should dispatch registered handler");
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn contract_register_and_unregister_handler_updates_handler_count() {
    let subject = "unit.runtime.handler-count".to_string();
    let runtime = new_runtime(BasicBus::new());
    let handler: thalamus_runtime::EventHandler = Arc::new(|_subject, _event| Box::pin(async {}));

    assert_eq!(runtime.handler_count().await, 0);

    runtime.register_handler(subject.clone(), handler).await;
    assert_eq!(runtime.handler_count().await, 1);

    runtime.unregister_handler(&subject).await;
    assert_eq!(runtime.handler_count().await, 0);
}

#[tokio::test]
async fn contract_spawn_returns_handle_and_tracks_active_task_count() {
    let runtime = new_runtime(BasicBus::new());
    let (finish_tx, finish_rx) = tokio::sync::oneshot::channel::<()>();

    assert_eq!(runtime.active_task_count().await, 0);

    let handle = runtime
        .spawn(async move {
            let _ = finish_rx.await;
        })
        .await;

    assert_eq!(handle.id.len(), 36);
    assert_eq!(runtime.active_task_count().await, 1);

    finish_tx
        .send(())
        .expect("spawned task should still be awaiting completion signal");
    tokio::task::yield_now().await;

    assert_eq!(runtime.active_task_count().await, 0);
}

#[tokio::test]
async fn behavior_handle_event_without_registered_handler_returns_schedule_error() {
    let runtime = new_runtime(BasicBus::new());
    let subject = "unit.runtime.missing".to_string();
    let envelope = EventEnvelope {
        id: "missing-handler-event".to_string(),
        r#type: subject.clone(),
        subject: subject.clone(),
        source: "runtime-basic-test".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        schema: "thalamus.unit.runtime.missing".to_string(),
        scope: None,
        refs: None,
        payload: serde_json::json!({ "contract": "missing-handler" }),
        correlation_id: None,
        causation_id: None,
        metadata: serde_json::json!({}),
    };

    let error = runtime
        .handle_event(subject.clone(), envelope)
        .await
        .expect_err("missing handler should return a schedule error");

    assert!(
        matches!(error, RuntimeError::ScheduleError(message) if message == format!("no handler for subject: {subject}"))
    );
}

#[tokio::test]
async fn contract_debug_formatters_expose_runtime_and_task_handle_shape() {
    let runtime = new_runtime(BasicBus::default());
    let handle = runtime.spawn(async {}).await;

    let runtime_debug = format!("{:?}", runtime);
    let handle_debug = format!("{:?}", handle);

    assert!(runtime_debug.starts_with("ThalamusRuntime"));
    assert!(runtime_debug.contains("state"));
    assert!(handle_debug.starts_with("TaskHandle"));
    assert!(handle_debug.contains(&handle.id));
}

#[tokio::test]
async fn behavior_lifecycle_errors_preserve_state_transition_contract() {
    let mut runtime = new_runtime(BasicBus::default());

    let not_running_error = runtime
        .stop()
        .await
        .expect_err("initialized runtime cannot be stopped");
    assert!(
        matches!(not_running_error, RuntimeError::LifecycleError(message) if message == "not running, cannot stop")
    );

    runtime.start().await.expect("runtime should start once");

    let already_running_error = runtime
        .start()
        .await
        .expect_err("running runtime cannot be started again");
    assert!(
        matches!(already_running_error, RuntimeError::LifecycleError(message) if message == "already running")
    );

    runtime.stop().await.expect("running runtime should stop");

    let already_stopped_error = runtime
        .stop()
        .await
        .expect_err("stopped runtime cannot be stopped again");
    assert!(
        matches!(already_stopped_error, RuntimeError::LifecycleError(message) if message == "already stopped")
    );
}

#[tokio::test]
async fn runtime_publish_without_handler_is_ok() {
    let runtime = new_runtime(BasicBus::default());
    let subject = "unit.runtime.unregistered-publish".to_string();

    let envelope = runtime
        .publish(
            subject.clone(),
            "runtime-basic-test".to_string(),
            serde_json::json!({ "behavior": "publish-without-handler" }),
        )
        .await
        .expect("publish without handler should be accepted and recorded");

    assert_eq!(envelope.subject, subject);
    assert_eq!(envelope.source, "runtime-basic-test");
    assert_eq!(
        runtime_basic_bus(&runtime).published_events().await,
        vec![envelope]
    );
}

#[tokio::test]
async fn contract_task_state_and_worker_registry_support_runtime_lookup() {
    let mut registry = WorkerRegistry::default();
    let task_state = TaskState::new("task-runtime-1".to_string());

    registry.register(
        "agent-1".to_string(),
        vec!["llm".to_string(), "tool.echo".to_string()],
    );
    task_state.assign_to("agent-1".to_string()).await;

    let worker = registry
        .lookup("agent-1")
        .expect("registered worker should be visible");
    assert_eq!(task_state.id(), "task-runtime-1");
    assert_eq!(
        task_state.assigned_agent().await.as_deref(),
        Some("agent-1")
    );
    assert_eq!(
        worker.capabilities,
        vec!["llm".to_string(), "tool.echo".to_string()]
    );
}

#[tokio::test]
async fn runtime_agent_ready_exit_error_updates_registry() {
    let mut runtime = new_runtime(BasicBus::default());

    runtime
        .start()
        .await
        .expect("runtime should start with agent lifecycle handlers");

    runtime
        .publish(
            RUNTIME_AGENT_READY.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::to_value(RuntimeAgentReadyPayload {
                agent_id: "agent-1".to_string(),
                capabilities: vec!["llm".to_string(), "tool.echo".to_string()],
            })
            .expect("agent ready payload should serialize"),
        )
        .await
        .expect("agent ready publish should update registry");

    let registry = runtime.worker_registry().await;
    let ready_worker = registry
        .lookup("agent-1")
        .expect("ready agent should be registered");
    assert_eq!(ready_worker.state, "ready");
    assert_eq!(
        ready_worker.capabilities,
        vec!["llm".to_string(), "tool.echo".to_string()]
    );

    runtime
        .publish(
            RUNTIME_AGENT_EXIT.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::to_value(RuntimeAgentExitPayload {
                agent_id: "agent-1".to_string(),
                reason: Some("shutdown".to_string()),
            })
            .expect("agent exit payload should serialize"),
        )
        .await
        .expect("agent exit publish should update registry");
    assert_eq!(
        runtime
            .worker_registry()
            .await
            .lookup("agent-1")
            .expect("exited agent should remain observable")
            .state,
        thalamus_runtime::WorkerState::Exited
    );

    runtime
        .publish(
            RUNTIME_AGENT_ERROR.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::to_value(RuntimeAgentErrorPayload {
                agent_id: Some("agent-1".to_string()),
                task_id: Some("task-1".to_string()),
                error: serde_json::json!({ "message": "failed" }),
            })
            .expect("agent error payload should serialize"),
        )
        .await
        .expect("agent error publish should update registry");
    assert_eq!(
        runtime
            .worker_registry()
            .await
            .lookup("agent-1")
            .expect("errored agent should remain observable")
            .state,
        thalamus_runtime::WorkerState::Error
    );
}

#[tokio::test]
async fn runtime_task_assign_result_updates_task_state() {
    let mut runtime = new_runtime(BasicBus::default());

    runtime
        .start()
        .await
        .expect("runtime should start with task state handlers");

    runtime
        .publish(
            RUNTIME_TASK_ASSIGN.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::to_value(RuntimeTaskAssignPayload {
                task_id: "task-runtime-1".to_string(),
                agent_id: Some("agent-1".to_string()),
                parent_task_id: None,
                input: serde_json::json!({ "prompt": "summarize runtime MVP" }),
                capabilities: vec!["llm".to_string()],
                metadata: serde_json::json!({}),
                correlation_id: Some("correlation-1".to_string()),
            })
            .expect("task assign payload should serialize"),
        )
        .await
        .expect("task assign publish should create assigned task state");

    let assigned_task = runtime
        .task_state("task-runtime-1")
        .await
        .expect("assigned task state should be observable");
    assert_eq!(assigned_task.status().await, "assigned");
    assert_eq!(
        assigned_task.assigned_agent().await.as_deref(),
        Some("agent-1")
    );

    runtime
        .publish(
            RUNTIME_TASK_RESULT.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::to_value(RuntimeTaskResultPayload {
                task_id: "task-runtime-1".to_string(),
                status: "completed".to_string(),
                result: Some(serde_json::json!({ "ok": true })),
                error: serde_json::json!({}),
                correlation_id: Some("correlation-1".to_string()),
            })
            .expect("task result payload should serialize"),
        )
        .await
        .expect("task result publish should update task state");

    assert_eq!(
        runtime
            .task_state("task-runtime-1")
            .await
            .expect("completed task state should remain observable")
            .status()
            .await,
        "completed"
    );
}

#[tokio::test]
async fn behavior_runtime_start_registers_default_mvp_subjects() {
    let mut runtime = new_runtime(BasicBus::default());

    runtime
        .start()
        .await
        .expect("runtime should start with default MVP handlers");

    // runtime.agent.spawn has no strict payload validation; accept any JSON
    let envelope = runtime
        .publish(
            RUNTIME_AGENT_SPAWN.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::json!({ "behavior": "default-mvp-subject" }),
        )
        .await
        .expect("runtime.agent.spawn should accept any JSON payload");
    assert_eq!(envelope.subject, RUNTIME_AGENT_SPAWN);
    assert_eq!(envelope.r#type, RUNTIME_AGENT_SPAWN);

    // runtime.task.assign requires task_id field
    let envelope = runtime
        .publish(
            RUNTIME_TASK_ASSIGN.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::json!({ "task_id": "test-task-001" }),
        )
        .await
        .expect("runtime.task.assign should accept valid payload");
    assert_eq!(envelope.subject, RUNTIME_TASK_ASSIGN);
    assert_eq!(envelope.r#type, RUNTIME_TASK_ASSIGN);

    // runtime.llm.request requires task_id field
    let envelope = runtime
        .publish(
            RUNTIME_LLM_REQUEST.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::json!({ "task_id": "test-task-002" }),
        )
        .await
        .expect("runtime.llm.request should accept valid payload");
    assert_eq!(envelope.subject, RUNTIME_LLM_REQUEST);
    assert_eq!(envelope.r#type, RUNTIME_LLM_REQUEST);

    // runtime.tool.request requires capability field
    let envelope = runtime
        .publish(
            RUNTIME_TOOL_REQUEST.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::json!({ "capability": "echo", "task_id": "test-task-003", "input": {} }),
        )
        .await
        .expect("runtime.tool.request should accept valid payload");
    assert_eq!(envelope.subject, RUNTIME_TOOL_REQUEST);
    assert_eq!(envelope.r#type, RUNTIME_TOOL_REQUEST);
}

#[tokio::test]
async fn behavior_mock_llm_provider_and_echo_tool_mediate_runtime_requests() {
    let llm_provider = MockLlmProvider;
    let echo_tool = EchoTool;
    let llm_request = RuntimeLLMRequestPayload {
        task_id: "task-runtime-1".to_string(),
        request_id: None,
        model: Some("mock-model".to_string()),
        prompt: Some("summarize runtime MVP".to_string()),
        messages: Vec::new(),
        options: serde_json::json!({}),
        correlation_id: Some("correlation-llm-1".to_string()),
    };
    let tool_request = RuntimeToolRequestPayload {
        task_id: "task-runtime-1".to_string(),
        request_id: None,
        capability: "echo".to_string(),
        input: serde_json::json!({ "text": "runtime MVP" }),
        timeout_seconds: None,
        correlation_id: Some("correlation-tool-1".to_string()),
    };

    let llm_response = llm_provider
        .complete(llm_request)
        .await
        .expect("mock LLM should answer");
    let tool_result = echo_tool
        .invoke(tool_request.input.clone())
        .await
        .expect("echo tool should answer");
    let tool_result: serde_json::Value =
        serde_json::from_value(tool_result).expect("echo tool result should deserialize as JSON");

    assert_eq!(
        llm_response.message["content"],
        serde_json::json!("Mock response: summarize runtime MVP")
    );
    assert_eq!(tool_result, serde_json::json!({ "text": "runtime MVP" }));
}

#[tokio::test]
async fn behavior_runtime_default_handlers_publish_llm_response_and_tool_result() {
    let mut runtime = new_runtime(BasicBus::default());
    let observed_subjects = Arc::new(RwLock::new(Vec::<String>::new()));
    let llm_observed_subjects = Arc::clone(&observed_subjects);
    let llm_response_handler: thalamus_runtime::EventHandler = Arc::new(move |subject, _event| {
        let observed_subjects = Arc::clone(&llm_observed_subjects);
        Box::pin(async move {
            observed_subjects
                .write()
                .expect("observed subjects lock should not be poisoned")
                .push(subject);
        })
    });
    let tool_observed_subjects = Arc::clone(&observed_subjects);
    let tool_result_handler: thalamus_runtime::EventHandler = Arc::new(move |subject, _event| {
        let observed_subjects = Arc::clone(&tool_observed_subjects);
        Box::pin(async move {
            observed_subjects
                .write()
                .expect("observed subjects lock should not be poisoned")
                .push(subject);
        })
    });

    runtime
        .register_handler(RUNTIME_LLM_RESPONSE.to_string(), llm_response_handler)
        .await;
    runtime
        .register_handler(RUNTIME_TOOL_RESULT.to_string(), tool_result_handler)
        .await;

    runtime
        .start()
        .await
        .expect("runtime should start with default MVP handlers");

    runtime
        .publish(
            RUNTIME_LLM_REQUEST.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::json!({
                "task_id": "task-runtime-1",
                "prompt": "summarize runtime MVP",
                "model": "mock-model"
            }),
        )
        .await
        .expect("runtime should accept LLM request through the bus");

    runtime
        .publish(
            RUNTIME_TOOL_REQUEST.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::json!({
                "task_id": "task-runtime-1",
                "capability": "echo",
                "input": { "text": "runtime MVP" }
            }),
        )
        .await
        .expect("runtime should accept tool request through the bus");

    let published_subjects = observed_subjects
        .read()
        .expect("observed subjects lock should not be poisoned")
        .clone();

    assert!(
        published_subjects
            .iter()
            .any(|subject| subject == RUNTIME_LLM_RESPONSE),
        "expected runtime default LLM handler to publish {RUNTIME_LLM_RESPONSE}, got {published_subjects:?}"
    );
    assert!(
        published_subjects
            .iter()
            .any(|subject| subject == RUNTIME_TOOL_RESULT),
        "expected runtime default tool handler to publish {RUNTIME_TOOL_RESULT}, got {published_subjects:?}"
    );
}

#[tokio::test]
async fn behavior_runtime_llm_result_payload_preserves_request_correlation_id() {
    let mut runtime = new_runtime(BasicBus::default());

    runtime
        .start()
        .await
        .expect("runtime should start with default MVP handlers");

    let llm_request_envelope = runtime
        .publish(
            RUNTIME_LLM_REQUEST.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::json!({
                "task_id": "task-runtime-llm-correlation-1",
                "prompt": "summarize runtime correlation MVP",
                "model": "mock-model",
                "correlation_id": "correlation-llm-result-1"
            }),
        )
        .await
        .expect("runtime should accept LLM correlation request through the bus");

    let published_events = runtime_basic_bus(&runtime).published_events().await;
    let llm_result = published_events
        .iter()
        .find(|event| {
            event.subject == RUNTIME_LLM_RESPONSE
                && event.payload["task_id"] == serde_json::json!("task-runtime-llm-correlation-1")
        })
        .expect("LLM result payload correlation_id should equal request correlation_id");

    assert_eq!(
        llm_result.payload["correlation_id"],
        serde_json::json!("correlation-llm-result-1")
    );
    assert_eq!(
        llm_result.correlation_id.as_deref(),
        Some(llm_request_envelope.id.as_str())
    );
    assert_eq!(
        llm_result.causation_id.as_deref(),
        Some(llm_request_envelope.id.as_str())
    );
}

#[tokio::test]
async fn behavior_runtime_tool_result_payload_preserves_request_correlation_id() {
    let mut runtime = new_runtime(BasicBus::default());

    runtime
        .start()
        .await
        .expect("runtime should start with default MVP handlers");

    let tool_request_envelope = runtime
        .publish(
            RUNTIME_TOOL_REQUEST.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::json!({
                "task_id": "task-runtime-tool-correlation-1",
                "capability": "echo",
                "input": { "text": "runtime correlation MVP" },
                "correlation_id": "correlation-tool-result-1"
            }),
        )
        .await
        .expect("runtime should accept tool correlation request through the bus");

    let published_events = runtime_basic_bus(&runtime).published_events().await;
    let tool_result = published_events
        .iter()
        .find(|event| {
            event.subject == RUNTIME_TOOL_RESULT
                && event.payload["task_id"] == serde_json::json!("task-runtime-tool-correlation-1")
        })
        .expect("Tool result payload correlation_id should equal request correlation_id");

    assert_eq!(
        tool_result.payload["correlation_id"],
        serde_json::json!("correlation-tool-result-1")
    );
    assert_eq!(
        tool_result.correlation_id.as_deref(),
        Some(tool_request_envelope.id.as_str())
    );
    assert_eq!(
        tool_result.causation_id.as_deref(),
        Some(tool_request_envelope.id.as_str())
    );
}

#[tokio::test]
async fn behavior_runtime_llm_request_uses_last_message_content_without_prompt() {
    let mut runtime = new_runtime(BasicBus::default());
    runtime
        .start()
        .await
        .expect("runtime should start with default MVP handlers");

    runtime
        .publish(
            RUNTIME_LLM_REQUEST.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::json!({
                "task_id": "task-runtime-1",
                "messages": [
                    { "role": "system", "content": "ignore setup" },
                    { "role": "user", "content": "summarize messages MVP" }
                ],
                "model": "mock-model"
            }),
        )
        .await
        .expect("runtime should accept messages-only LLM request through the bus");

    let published_llm_responses = runtime_basic_bus(&runtime).published_events().await;
    let llm_response = published_llm_responses
        .iter()
        .find(|event| event.subject == RUNTIME_LLM_RESPONSE)
        .expect("messages-only LLM request should publish runtime LLM response");

    assert_eq!(
        llm_response.payload["message"]["content"],
        serde_json::json!("Mock response: summarize messages MVP")
    );
}

#[tokio::test]
async fn behavior_runtime_agent_lifecycle_events_update_worker_registry_state() {
    let mut runtime = new_runtime(BasicBus::default());

    runtime
        .start()
        .await
        .expect("runtime should start with agent lifecycle handlers");

    runtime
        .publish(
            RUNTIME_AGENT_READY.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::to_value(RuntimeAgentReadyPayload {
                agent_id: "agent-lifecycle-1".to_string(),
                capabilities: vec!["llm".to_string(), "tool.echo".to_string()],
            })
            .expect("agent ready payload should serialize"),
        )
        .await
        .expect("agent.ready should update worker registry");

    let ready_registry = runtime.worker_registry().await;
    let ready_worker = ready_registry
        .lookup("agent-lifecycle-1")
        .expect("agent.ready should register the worker");
    assert_eq!(ready_worker.state, "ready");
    assert_eq!(
        ready_worker.capabilities,
        vec!["llm".to_string(), "tool.echo".to_string()]
    );

    runtime
        .publish(
            RUNTIME_AGENT_EXIT.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::to_value(RuntimeAgentExitPayload {
                agent_id: "agent-lifecycle-1".to_string(),
                reason: Some("completed".to_string()),
            })
            .expect("agent exit payload should serialize"),
        )
        .await
        .expect("agent.exit should update worker registry");
    assert_eq!(
        runtime
            .worker_registry()
            .await
            .lookup("agent-lifecycle-1")
            .expect("agent.exit should preserve worker lookup")
            .state,
        thalamus_runtime::WorkerState::Exited
    );

    runtime
        .publish(
            RUNTIME_AGENT_ERROR.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::to_value(RuntimeAgentErrorPayload {
                agent_id: Some("agent-lifecycle-1".to_string()),
                task_id: Some("task-lifecycle-1".to_string()),
                error: serde_json::json!({ "message": "tool failed" }),
            })
            .expect("agent error payload should serialize"),
        )
        .await
        .expect("agent.error should update worker registry");
    assert_eq!(
        runtime
            .worker_registry()
            .await
            .lookup("agent-lifecycle-1")
            .expect("agent.error should preserve worker lookup")
            .state,
        thalamus_runtime::WorkerState::Error
    );
}

#[tokio::test]
async fn behavior_runtime_llm_response_correlation_and_causation_equal_request_event_id() {
    let mut runtime = new_runtime(BasicBus::default());

    runtime
        .start()
        .await
        .expect("runtime should start with default MVP handlers");

    let request_event = runtime
        .publish(
            RUNTIME_LLM_REQUEST.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::json!({
                "task_id": "task-runtime-llm-causation-1",
                "prompt": "summarize response causation MVP",
                "model": "mock-model"
            }),
        )
        .await
        .expect("runtime should accept LLM request through the bus");

    let published_events = runtime_basic_bus(&runtime).published_events().await;
    let llm_response = published_events
        .iter()
        .find(|event| {
            event.subject == RUNTIME_LLM_RESPONSE
                && event.payload["task_id"] == serde_json::json!("task-runtime-llm-causation-1")
        })
        .expect("runtime.llm.response should be published for the request");

    assert_eq!(
        llm_response.correlation_id.as_deref(),
        Some(request_event.id.as_str())
    );
    assert_eq!(
        llm_response.causation_id.as_deref(),
        Some(request_event.id.as_str())
    );
}

#[tokio::test]
async fn behavior_runtime_tool_result_correlation_and_causation_equal_request_event_id() {
    let mut runtime = new_runtime(BasicBus::default());

    runtime
        .start()
        .await
        .expect("runtime should start with default MVP handlers");

    let request_event = runtime
        .publish(
            RUNTIME_TOOL_REQUEST.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::json!({
                "task_id": "task-runtime-tool-causation-1",
                "capability": "echo",
                "input": { "text": "runtime causation MVP" }
            }),
        )
        .await
        .expect("runtime should accept tool request through the bus");

    let published_events = runtime_basic_bus(&runtime).published_events().await;
    let tool_result = published_events
        .iter()
        .find(|event| {
            event.subject == RUNTIME_TOOL_RESULT
                && event.payload["task_id"] == serde_json::json!("task-runtime-tool-causation-1")
        })
        .expect("runtime.tool.result should be published for the request");

    assert_eq!(
        tool_result.correlation_id.as_deref(),
        Some(request_event.id.as_str())
    );
    assert_eq!(
        tool_result.causation_id.as_deref(),
        Some(request_event.id.as_str())
    );
}

#[tokio::test]
async fn behavior_runtime_agent_error_without_agent_id_does_not_modify_registry() {
    // agent_id: None の場合、worker registry を勝手に変更しないことを確認
    let mut runtime = new_runtime(BasicBus::default());

    runtime.start().await.expect("runtime should start");

    // agent_id: None で agent.error を publish
    runtime
        .publish(
            RUNTIME_AGENT_ERROR.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::to_value(RuntimeAgentErrorPayload {
                agent_id: None,
                task_id: Some("task-startup-failed".to_string()),
                error: serde_json::json!({ "message": "runtime startup failed" }),
            })
            .expect("agent error payload without agent_id should serialize"),
        )
        .await
        .expect("agent error publish without agent_id should succeed");

    // registry に agent が登録されていないことを確認
    let registry = runtime.worker_registry().await;
    assert!(
        registry.lookup("unknown").is_none(),
        "worker registry should not have unknown agents when agent_id is None"
    );
}
