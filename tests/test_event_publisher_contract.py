import asyncio
import json

from runtime.events.publisher import EventPublisher


class FakeBus:

    def __init__(self):
        self.calls = []

    async def publish(self, subject, payload):
        self.calls.append(
            (subject, payload)
        )


def test_publish_uses_event_type_as_default_subject_and_json_bytes():
    bus = FakeBus()
    publisher = EventPublisher(bus)

    async def publish_event():
        await publisher.publish(
            event_type="runtime.task.assign",
            source="test-suite",
            payload={
                "task_id": "task-1",
                "agent_id": "agent-1",
                "input": {"objective": "verify default subject"},
                "capabilities": ["contract-test"],
                "metadata": {"origin": "event-publisher-contract"},
            },
        )

    asyncio.run(publish_event())

    assert len(bus.calls) == 1

    subject, payload = bus.calls[0]

    assert subject == "runtime.task.assign"
    assert isinstance(payload, bytes)

    event = json.loads(
        payload.decode()
    )
    assert event["type"] == "runtime.task.assign"
    assert event["subject"] == "runtime.task.assign"
    assert event["source"] == "test-suite"
    assert event["schema"] == "runtime.event.v1"
    assert event["payload"] == {
        "task_id": "task-1",
        "agent_id": "agent-1",
        "input": {"objective": "verify default subject"},
        "capabilities": ["contract-test"],
        "metadata": {"origin": "event-publisher-contract"},
    }
