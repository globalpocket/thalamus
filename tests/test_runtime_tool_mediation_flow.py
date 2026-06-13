import pytest

from runtime.runtime import ThalamusRuntime
from runtime.events.validator import validate_event


class FakeBus:

    def __init__(self):
        self.connected = False
        self.closed = False
        self.published = []
        self.subscriptions = {}

    async def connect(self):
        self.connected = True

    async def close(self):
        self.closed = True

    async def subscribe(self, subject, handler):
        self.subscriptions.setdefault(subject, []).append(handler)

    async def publish(self, subject, payload):
        self.published.append((subject, payload))

        for handler in self.subscriptions.get(subject, []):
            await handler(subject, payload)


@pytest.mark.asyncio
async def test_behavior_runtime_subscribes_to_tool_request():
    bus = FakeBus()
    runtime = ThalamusRuntime(bus=bus)
    runtime.tools = {
        "tool.echo": lambda payload: {"echo": payload["message"]}
    }

    await runtime.start()

    assert "runtime.tool.request" in bus.subscriptions


@pytest.mark.asyncio
async def test_regression_tool_request_publishes_validated_success_result():
    bus = FakeBus()
    runtime = ThalamusRuntime(bus=bus)
    runtime.tools = {
        "tool.echo": lambda payload: {"echo": payload["message"]}
    }
    request_event_id = "event-tool-request-unit-6-success"

    await runtime.start()
    await bus.publish(
        "runtime.task.assign",
        {
            "type": "runtime.task.assign",
            "payload": {
                "task_id": "task-unit-6-success",
                "agent_id": "agent-unit-6",
                "input": {"objective": "mediate tool request state"},
                "capabilities": ["tool.echo"],
                "metadata": {"purpose": "mediate tool request state"},
            },
        },
    )
    await bus.publish(
        "runtime.tool.request",
        {
            "id": request_event_id,
            "type": "runtime.tool.request",
            "payload": {
                "request_id": "tool-request-unit-6-success",
                "task_id": "task-unit-6-success",
                "capability": "tool.echo",
                "input": {"message": "hello"},
                "agent_id": "agent-unit-6",
            },
        },
    )

    assert runtime.task_states["task-unit-6-success"]["status"] == "running"

    result_subject, result_event = bus.published[-1]
    validated_result = validate_event(
        event_type="runtime.tool.result",
        source=result_event["source"],
        payload=result_event["payload"],
    )

    assert result_subject == "runtime.tool.result"
    # contract: runtime.tool.result is published as a full canonical envelope.
    for envelope_key in (
        "id",
        "timestamp",
        "metadata",
        "correlation_id",
        "causation_id",
    ):
        assert envelope_key in result_event
    assert result_event["correlation_id"] == request_event_id
    assert result_event["causation_id"] == request_event_id
    assert validated_result.payload == {
        "request_id": "tool-request-unit-6-success",
        "task_id": "task-unit-6-success",
        "status": "success",
        "output": {"echo": "hello"},
        "error": None,
    }


@pytest.mark.asyncio
async def test_behavior_unregistered_tool_request_publishes_validated_error_result():
    bus = FakeBus()
    runtime = ThalamusRuntime(bus=bus)
    runtime.tools = {}

    await runtime.start()
    await bus.publish(
        "runtime.tool.request",
        {
            "type": "runtime.tool.request",
            "payload": {
                "request_id": "tool-request-unit-6-error",
                "task_id": "task-unit-6-error",
                "capability": "tool.missing",
                "input": {"message": "hello"},
                "agent_id": "agent-unit-6",
            },
        },
    )

    result_subject, result_event = bus.published[-1]
    validated_result = validate_event(
        event_type="runtime.tool.result",
        source=result_event["source"],
        payload=result_event["payload"],
    )

    assert result_subject == "runtime.tool.result"
    assert validated_result.payload["request_id"] == "tool-request-unit-6-error"
    assert validated_result.payload["task_id"] == "task-unit-6-error"
    assert validated_result.payload["status"] == "error"
    assert validated_result.payload["output"] is None
    assert "tool.missing" in validated_result.payload["error"]
