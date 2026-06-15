use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use thalamus_bus::BasicBus;
use thalamus_protocol::EventEnvelope;
use thalamus_runtime::{RuntimeError, RuntimeState, ThalamusRuntime};

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
    runtime.start().await.expect("runtime should start with registered handler");

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
    let handler: thalamus_runtime::EventHandler = Arc::new(|_subject, _event| {
        Box::pin(async {})
    });

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

    finish_tx.send(()).expect("spawned task should still be awaiting completion signal");
    tokio::task::yield_now().await;

    assert_eq!(runtime.active_task_count().await, 0);
}

#[tokio::test]
async fn behavior_handle_event_without_registered_handler_returns_schedule_error() {
    let runtime = ThalamusRuntime::new(BasicBus::new());
    let subject = "unit.runtime.missing".to_string();
    let envelope = EventEnvelope {
        id: "missing-handler-event".to_string(),
        subject: subject.clone(),
        source: "runtime-basic-test".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        schema: "thalamus.unit.runtime.missing".to_string(),
        payload: serde_json::json!({ "contract": "missing-handler" }),
        correlation_id: None,
        causation_id: None,
        metadata: serde_json::json!({}),
    };

    let error = runtime
        .handle_event(subject.clone(), envelope)
        .await
        .expect_err("missing handler should return a schedule error");

    assert!(matches!(error, RuntimeError::ScheduleError(message) if message == format!("no handler for subject: {subject}")));
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
    assert!(matches!(not_running_error, RuntimeError::LifecycleError(message) if message == "not running, cannot stop"));

    runtime.start().await.expect("runtime should start once");

    let already_running_error = runtime
        .start()
        .await
        .expect_err("running runtime cannot be started again");
    assert!(matches!(already_running_error, RuntimeError::LifecycleError(message) if message == "already running"));

    runtime.stop().await.expect("running runtime should stop");

    let already_stopped_error = runtime
        .stop()
        .await
        .expect_err("stopped runtime cannot be stopped again");
    assert!(matches!(already_stopped_error, RuntimeError::LifecycleError(message) if message == "already stopped"));
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

    assert!(matches!(error, RuntimeError::BusError(message) if message == format!("publish failed: subject not found: {subject}")));
}
