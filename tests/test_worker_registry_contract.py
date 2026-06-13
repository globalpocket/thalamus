from runtime.registry.registry import WorkerRegistry


def test_behavior_register_then_unregister_removes_worker():
    registry = WorkerRegistry()

    registry.register(
        "agent-unit-register-unregister",
        ["tool.shell", "llm.mock"],
    )

    assert registry.workers["agent-unit-register-unregister"] == {
        "capabilities": ["tool.shell", "llm.mock"],
    }

    registry.unregister("agent-unit-register-unregister")

    assert "agent-unit-register-unregister" not in registry.workers
