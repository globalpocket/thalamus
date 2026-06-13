import pytest

from runtime.bus.in_memory_bus import InMemoryBus


@pytest.mark.asyncio
async def test_contract_in_memory_bus_records_published_events():
    bus = InMemoryBus()

    event = {
        "type": "runtime.task.assign",
        "payload": {"task_id": "task-unit-3"},
    }

    await bus.connect()
    await bus.publish("runtime.task.assign", event)
    await bus.close()

    assert bus.published == [("runtime.task.assign", event)]


@pytest.mark.asyncio
async def test_contract_in_memory_bus_dispatches_only_matching_subject():
    bus = InMemoryBus()
    received = []

    async def handler(subject, event):
        received.append((subject, event))

    matching_event = {
        "type": "runtime.task.assign",
        "payload": {"task_id": "task-matching"},
    }
    other_event = {
        "type": "runtime.agent.ready",
        "payload": {"agent_id": "agent-ignored"},
    }

    await bus.connect()
    await bus.subscribe("runtime.task.assign", handler)
    await bus.publish("runtime.agent.ready", other_event)
    await bus.publish("runtime.task.assign", matching_event)
    await bus.close()

    assert received == [("runtime.task.assign", matching_event)]
