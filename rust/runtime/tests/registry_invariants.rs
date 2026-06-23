use thalamus_runtime::{WorkerRegistry, WorkerState};

#[test]
fn register_creates_ready_worker() {
    let mut registry = WorkerRegistry::default();

    registry.register("agent-1".to_string(), vec!["llm".to_string(), "tool.echo".to_string()]);

    let worker = registry.lookup("agent-1").expect("worker should exist");
    assert_eq!(worker.state, WorkerState::Ready);
    assert_eq!(worker.capabilities, vec!["llm".to_string(), "tool.echo".to_string()]);
}

#[test]
fn mark_ready_updates_state() {
    let mut registry = WorkerRegistry::default();

    registry.mark_ready("agent-1".to_string(), vec!["llm".to_string()]);

    let worker = registry.lookup("agent-1").expect("worker should exist");
    assert_eq!(worker.state, WorkerState::Ready);
}

#[test]
fn mark_exited_updates_state() {
    let mut registry = WorkerRegistry::default();

    registry.mark_ready("agent-1".to_string(), vec!["llm".to_string()]);
    registry.mark_exited("agent-1".to_string(), Some("shutdown".to_string()));

    let worker = registry.lookup("agent-1").expect("worker should exist");
    assert_eq!(worker.state, WorkerState::Exited);
}

#[test]
fn mark_error_updates_state() {
    let mut registry = WorkerRegistry::default();

    registry.mark_ready("agent-1".to_string(), vec!["llm".to_string()]);
    registry.mark_error(
        "agent-1".to_string(),
        Some("task-1".to_string()),
        serde_json::json!({"message": "fail"}),
    );

    let worker = registry.lookup("agent-1").expect("worker should exist");
    assert_eq!(worker.state, WorkerState::Error);
}

#[test]
fn mark_error_without_agent_id_does_not_crash() {
    let mut registry = WorkerRegistry::default();

    // Calling mark_error with a new agent_id should create the worker
    registry.mark_error(
        "agent-new".to_string(),
        None,
        serde_json::json!({"message": "error"}),
    );

    let worker = registry.lookup("agent-new").expect("worker should exist");
    assert_eq!(worker.state, WorkerState::Error);
}

#[test]
fn find_by_capability_filters_by_state() {
    let mut registry = WorkerRegistry::default();

    registry.register("agent-1".to_string(), vec!["llm".to_string()]);
    registry.register("agent-2".to_string(), vec!["llm".to_string()]);
    registry.mark_exited("agent-2".to_string(), None);

    let ready_llm_agents = registry.find_by_capability("llm");
    assert_eq!(ready_llm_agents.len(), 1);
    assert!(ready_llm_agents.contains(&"agent-1".to_string()));
}

#[test]
fn list_capabilities_returns_sorted() {
    let mut registry = WorkerRegistry::default();

    registry.register("agent-1".to_string(), vec!["z-cap".to_string(), "a-cap".to_string()]);
    registry.register("agent-2".to_string(), vec!["m-cap".to_string()]);

    let caps = registry.list_capabilities();
    assert_eq!(caps, vec!["a-cap".to_string(), "m-cap".to_string(), "z-cap".to_string()]);
}

#[test]
fn worker_info_clone_is_independent() {
    let mut registry = WorkerRegistry::default();

    registry.register("agent-1".to_string(), vec!["llm".to_string()]);

    let info1 = registry.lookup("agent-1").cloned();
    registry.mark_exited("agent-1".to_string(), None);

    // Cloned info should not be affected by subsequent changes
    assert!(info1.is_some());
    assert_eq!(info1.unwrap().state, WorkerState::Ready);
}

#[test]
fn registry_clone_is_independent() {
    let mut registry = WorkerRegistry::default();

    registry.register("agent-1".to_string(), vec!["llm".to_string()]);

    let registry2 = registry.clone();

    // registry2 should have the same data
    assert!(registry2.lookup("agent-1").is_some());

    // Changes to registry should not affect registry2
    registry.mark_exited("agent-1".to_string(), None);
    assert_eq!(registry2.lookup("agent-1").unwrap().state, WorkerState::Ready);
}
