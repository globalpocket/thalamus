import pytest

from runtime.bus.in_memory_bus import InMemoryBus
from runtime.runtime import ThalamusRuntime


@pytest.mark.asyncio
async def test_regression_task_assign_records_runtime_task_state_from_canonical_payload():
    bus = InMemoryBus()
    runtime = ThalamusRuntime(bus=bus)

    await runtime.start()

    await bus.publish(
        "runtime.task.assign",
        {
            "type": "runtime.task.assign",
            "payload": {
                "task_id": "task-unit-5-assign",
                "agent_id": "agent-unit-A2",
                "input": {
                    "objective": "track assigned task state",
                },
                "capabilities": ["state-tracking"],
                "metadata": {
                    "source": "regression",
                },
            },
        },
    )

    task_state = runtime.task_states["task-unit-5-assign"]

    assert task_state["status"] == "assigned"
    assert task_state["agent_id"] == "agent-unit-A2"
    assert task_state["input"] == {"objective": "track assigned task state"}


@pytest.mark.asyncio
async def test_regression_task_result_completed_updates_runtime_task_state():
    bus = InMemoryBus()
    runtime = ThalamusRuntime(bus=bus)

    await runtime.start()

    await bus.publish(
        "runtime.task.assign",
        {
            "type": "runtime.task.assign",
            "payload": {
                "task_id": "task-unit-5-completed",
                "agent_id": "agent-unit-A2",
                "input": {
                    "objective": "track completed task state",
                },
                "capabilities": ["state-tracking"],
                "metadata": {
                    "source": "regression",
                },
            },
        },
    )
    await bus.publish(
        "runtime.task.result",
        {
            "type": "runtime.task.result",
            "payload": {
                "task_id": "task-unit-5-completed",
                "status": "completed",
                "summary": "completed by worker",
            },
        },
    )

    task_state = runtime.task_states["task-unit-5-completed"]

    assert task_state["status"] == "completed"
    assert task_state["summary"] == "completed by worker"


@pytest.mark.asyncio
async def test_regression_task_result_failed_records_failure_summary_and_result():
    bus = InMemoryBus()
    runtime = ThalamusRuntime(bus=bus)

    await runtime.start()

    await bus.publish(
        "runtime.task.assign",
        {
            "type": "runtime.task.assign",
            "payload": {
                "task_id": "task-unit-5-failed",
                "agent_id": "agent-unit-A2",
                "input": {
                    "objective": "track failed task state",
                },
                "capabilities": ["state-tracking"],
                "metadata": {
                    "source": "regression",
                },
            },
        },
    )
    await bus.publish(
        "runtime.task.result",
        {
            "type": "runtime.task.result",
            "payload": {
                "task_id": "task-unit-5-failed",
                "status": "failed",
                "summary": "worker reported failure",
                "result": {
                    "error": "capability failed",
                },
            },
        },
    )

    task_state = runtime.task_states["task-unit-5-failed"]

    assert task_state["status"] == "failed"
    assert task_state["summary"] == "worker reported failure"
    assert task_state["result"] == {"error": "capability failed"}
