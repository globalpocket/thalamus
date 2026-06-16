use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};
use thalamus_bus::BasicBus;
use thalamus_protocol::{
    payload::{RuntimeLLMRequestPayload, RuntimeToolRequestPayload},
    subject::{
        RUNTIME_AGENT_SPAWN, RUNTIME_LLM_REQUEST, RUNTIME_LLM_RESPONSE, RUNTIME_TASK_ASSIGN,
        RUNTIME_TOOL_REQUEST, RUNTIME_TOOL_RESULT,
    },
    EventEnvelope,
};
use thalamus_runtime::{
    EchoTool, MockLlmProvider, RuntimeError, RuntimeState, TaskState, ThalamusRuntime,
    WorkerRegistry,
};

fn runtime_basic_bus(runtime: &ThalamusRuntime<BasicBus>) -> &BasicBus {
    // SAFETY: このテストはThalamusRuntimeが所有するBasicBusの公開済みイベントだけを観測する。
    // ThalamusRuntimeの先頭フィールドはBasicBusであり、可変参照は作らず借用中にruntimeを移動しない。
    unsafe { &*(runtime as *const ThalamusRuntime<BasicBus> as *const BasicBus) }
}

#[tokio::test]
async fn contract_new_runtime_starts_in_initialized_state() {
    let runtime = ThalamusRuntime::new(BasicBus::new());

    assert_eq!(runtime.state().await, RuntimeState::Initialized);
}

#[tokio::test]
async fn behavior_start_stop_transitions_runtime_state() {
    let mut runtime = ThalamusRuntime::new(BasicBus::new());

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
    let mut runtime = ThalamusRuntime::new(BasicBus::new());
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
    let runtime = ThalamusRuntime::new(BasicBus::new());
    let handler: thalamus_runtime::EventHandler = Arc::new(|_subject, _event| Box::pin(async {}));

    assert_eq!(runtime.handler_count().await, 0);

    runtime.register_handler(subject.clone(), handler).await;
    assert_eq!(runtime.handler_count().await, 1);

    runtime.unregister_handler(&subject).await;
    assert_eq!(runtime.handler_count().await, 0);
}

#[tokio::test]
async fn contract_spawn_returns_handle_and_tracks_active_task_count() {
    let runtime = ThalamusRuntime::new(BasicBus::new());
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
    let runtime = ThalamusRuntime::new(BasicBus::new());
    let subject = "unit.runtime.missing".to_string();
    let envelope = EventEnvelope {
        id: "missing-handler-event".to_string(),
        r#type: subject.clone(),
        subject: subject.clone(),
        source: "runtime-basic-test".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        schema: "thalamus.unit.runtime.missing".to_string(),
        scope: None,
        refs: Vec::new(),
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
    let runtime = ThalamusRuntime::new(BasicBus::default());
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
    let mut runtime = ThalamusRuntime::new(BasicBus::default());

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
async fn behavior_publish_without_registered_subject_maps_bus_error() {
    let runtime = ThalamusRuntime::new(BasicBus::default());
    let subject = "unit.runtime.unregistered-publish".to_string();

    let error = runtime
        .publish(
            subject.clone(),
            "runtime-basic-test".to_string(),
            serde_json::json!({ "behavior": "publish-error-mapping" }),
        )
        .await
        .expect_err("unregistered subject should map bus error");

    assert!(
        matches!(error, RuntimeError::BusError(message) if message == format!("publish failed: subject not found: {subject}"))
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
async fn behavior_runtime_start_registers_default_mvp_subjects() {
    let mut runtime = ThalamusRuntime::new(BasicBus::default());

    runtime
        .start()
        .await
        .expect("runtime should start with default MVP handlers");

    for subject in [
        RUNTIME_AGENT_SPAWN,
        RUNTIME_TASK_ASSIGN,
        RUNTIME_LLM_REQUEST,
        RUNTIME_TOOL_REQUEST,
    ] {
        let envelope = runtime
            .publish(
                subject.to_string(),
                "runtime-basic-test".to_string(),
                serde_json::json!({ "behavior": "default-mvp-subject" }),
            )
            .await
            .expect("default MVP subject should be registered on runtime start");

        assert_eq!(envelope.subject, subject);
        assert_eq!(envelope.r#type, subject);
    }
}

#[tokio::test]
async fn behavior_mock_llm_provider_and_echo_tool_mediate_runtime_requests() {
    let llm_provider = MockLlmProvider;
    let echo_tool = EchoTool;
    let llm_request = RuntimeLLMRequestPayload {
        request_id: "llm-request-1".to_string(),
        task_id: Some("task-runtime-1".to_string()),
        prompt: "summarize runtime MVP".to_string(),
        model: Some("mock-model".to_string()),
        agent_id: Some("agent-1".to_string()),
    };
    let tool_request = RuntimeToolRequestPayload {
        request_id: "tool-request-1".to_string(),
        task_id: Some("task-runtime-1".to_string()),
        capability: "echo".to_string(),
        input: serde_json::json!({ "text": "runtime MVP" }),
        agent_id: Some("agent-1".to_string()),
    };

    let llm_response = llm_provider
        .complete(llm_request)
        .await
        .expect("mock LLM should answer");
    let tool_result = echo_tool
        .invoke(tool_request)
        .await
        .expect("echo tool should answer");

    assert_eq!(llm_response.status, "completed");
    assert_eq!(
        llm_response.text.as_deref(),
        Some("Mock response: summarize runtime MVP")
    );
    assert_eq!(tool_result.status, "completed");
    assert_eq!(
        tool_result.output,
        Some(serde_json::json!({ "text": "runtime MVP" }))
    );
}

#[tokio::test]
async fn behavior_runtime_default_handlers_publish_llm_response_and_tool_result() {
    let mut runtime = ThalamusRuntime::new(BasicBus::default());
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
                "request_id": "llm-request-1",
                "task_id": "task-runtime-1",
                "prompt": "summarize runtime MVP",
                "model": "mock-model",
                "agent_id": "agent-1"
            }),
        )
        .await
        .expect("runtime should accept LLM request through the bus");

    runtime
        .publish(
            RUNTIME_TOOL_REQUEST.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::json!({
                "request_id": "tool-request-1",
                "task_id": "task-runtime-1",
                "capability": "echo",
                "input": { "text": "runtime MVP" },
                "agent_id": "agent-1"
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
async fn behavior_runtime_llm_request_uses_last_message_content_without_prompt() {
    let mut runtime = ThalamusRuntime::new(BasicBus::default());
    runtime
        .start()
        .await
        .expect("runtime should start with default MVP handlers");

    runtime
        .publish(
            RUNTIME_LLM_REQUEST.to_string(),
            "runtime-basic-test".to_string(),
            serde_json::json!({
                "request_id": "llm-messages-request-1",
                "task_id": "task-runtime-1",
                "messages": [
                    { "role": "system", "content": "ignore setup" },
                    { "role": "user", "content": "summarize messages MVP" }
                ],
                "model": "mock-model",
                "agent_id": "agent-1"
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
        llm_response.payload["text"],
        serde_json::json!("Mock response: summarize messages MVP")
    );
}
