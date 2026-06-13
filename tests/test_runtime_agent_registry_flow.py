import pytest

from runtime.bus.in_memory_bus import InMemoryBus
from runtime.runtime import ThalamusRuntime


@pytest.mark.asyncio
async def test_behavior_agent_ready_publishes_capabilities_into_registry():
    bus = InMemoryBus()
    runtime = ThalamusRuntime(bus=bus)

    await runtime.start()

    ready_event = {
        "type": "runtime.agent.ready",
        "payload": {
            "agent_id": "agent-unit-4-ready",
            "capabilities": ["tool.shell", "llm.mock"],
        },
    }

    await bus.publish("runtime.agent.ready", ready_event)

    assert runtime.registry.find_by_capability("tool.shell") == [
        "agent-unit-4-ready"
    ]
    assert runtime.registry.workers["agent-unit-4-ready"] == {
        "status": "ready",
        "capabilities": ["tool.shell", "llm.mock"],
    }


@pytest.mark.asyncio
async def test_behavior_agent_exit_removes_or_marks_registry_agent_inactive():
    bus = InMemoryBus()
    runtime = ThalamusRuntime(bus=bus)

    await runtime.start()

    ready_event = {
        "type": "runtime.agent.ready",
        "payload": {
            "agent_id": "agent-unit-4-exit",
            "capabilities": ["tool.shell"],
        },
    }
    exit_event = {
        "type": "runtime.agent.exit",
        "payload": {
            "agent_id": "agent-unit-4-exit",
            "reason": "completed",
        },
    }

    await bus.publish("runtime.agent.ready", ready_event)
    await bus.publish("runtime.agent.exit", exit_event)

    agent_state = runtime.registry.workers.get("agent-unit-4-exit")

    if agent_state is None:
        assert "agent-unit-4-exit" not in runtime.registry.find_by_capability(
            "tool.shell"
        )
    else:
        assert agent_state == {
            "status": "exited",
            "capabilities": ["tool.shell"],
            "reason": "completed",
        }


@pytest.mark.asyncio
async def test_behavior_agent_error_records_registry_error_for_agent_task():
    bus = InMemoryBus()
    runtime = ThalamusRuntime(bus=bus)

    await runtime.start()

    ready_event = {
        "type": "runtime.agent.ready",
        "payload": {
            "agent_id": "agent-unit-4-error",
            "capabilities": ["tool.shell"],
        },
    }
    error_event = {
        "type": "runtime.agent.error",
        "payload": {
            "agent_id": "agent-unit-4-error",
            "task_id": "task-unit-4-error",
            "error": "worker failed",
        },
    }

    await bus.publish("runtime.agent.ready", ready_event)
    await bus.publish("runtime.agent.error", error_event)

    assert runtime.registry.workers["agent-unit-4-error"] == {
        "status": "error",
        "capabilities": ["tool.shell"],
        "task_id": "task-unit-4-error",
        "error": "worker failed",
    }
