from runtime.worker.worker_context import WorkerContext


def test_contract_worker_context_preserves_explicit_agent_id_and_capabilities():
    capabilities = ["tool.shell", "runtime.task.execute"]

    context = WorkerContext(
        agent_id="worker-explicit",
        capabilities=capabilities,
    )

    assert context.agent_id == "worker-explicit"
    assert context.capabilities == capabilities


def test_contract_worker_context_generates_default_worker_id_and_empty_capabilities():
    context = WorkerContext()

    assert context.agent_id.startswith("worker-")
    assert context.capabilities == []
