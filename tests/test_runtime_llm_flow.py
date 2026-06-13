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


class MockLLMProvider:

    async def complete(self, prompt, model=None):
        return f"mock completion for {model}: {prompt}"


class FailingMockLLMProvider:

    async def complete(self, prompt, model=None):
        raise RuntimeError("mock llm unavailable")


@pytest.mark.asyncio
async def test_behavior_runtime_subscribes_to_llm_request():
    bus = FakeBus()
    runtime = ThalamusRuntime(bus=bus)
    runtime.llm = MockLLMProvider()

    await runtime.start()

    assert "runtime.llm.request" in bus.subscriptions


@pytest.mark.asyncio
async def test_behavior_llm_request_publishes_validated_success_response():
    bus = FakeBus()
    runtime = ThalamusRuntime(bus=bus)
    runtime.llm = MockLLMProvider()

    await runtime.start()
    await bus.publish(
        "runtime.llm.request",
        {
            "id": "runtime-llm-request-event-unit-d-success",
            "type": "runtime.llm.request",
            "payload": {
                "request_id": "llm-request-unit-7a-success",
                "task_id": "task-unit-7a-success",
                "prompt": "summarize unit 7a",
                "model": "mock-model",
                "agent_id": "agent-unit-7a",
            },
        },
    )

    response_subject, response_event = bus.published[-1]
    # contract: runtime.llm.response must be published as a full canonical envelope.
    for envelope_key in (
        "id",
        "timestamp",
        "metadata",
        "correlation_id",
        "causation_id",
    ):
        assert envelope_key in response_event

    assert response_event["correlation_id"] == "runtime-llm-request-event-unit-d-success"
    assert response_event["causation_id"] == "runtime-llm-request-event-unit-d-success"

    validated_response = validate_event(
        event_type="runtime.llm.response",
        source=response_event["source"],
        payload=response_event["payload"],
    )

    assert response_subject == "runtime.llm.response"
    assert validated_response.payload == {
        "request_id": "llm-request-unit-7a-success",
        "task_id": "task-unit-7a-success",
        "status": "success",
        "text": "mock completion for mock-model: summarize unit 7a",
        "model": "mock-model",
        "error": None,
    }


@pytest.mark.asyncio
async def test_behavior_llm_request_publishes_validated_error_response():
    bus = FakeBus()
    runtime = ThalamusRuntime(bus=bus)
    runtime.llm = FailingMockLLMProvider()

    await runtime.start()
    await bus.publish(
        "runtime.llm.request",
        {
            "type": "runtime.llm.request",
            "payload": {
                "request_id": "llm-request-unit-7a-error",
                "task_id": "task-unit-7a-error",
                "prompt": "summarize failing unit 7a",
                "model": "mock-model",
                "agent_id": "agent-unit-7a",
            },
        },
    )

    response_subject, response_event = bus.published[-1]
    validated_response = validate_event(
        event_type="runtime.llm.response",
        source=response_event["source"],
        payload=response_event["payload"],
    )

    assert response_subject == "runtime.llm.response"
    assert validated_response.payload["request_id"] == "llm-request-unit-7a-error"
    assert validated_response.payload["task_id"] == "task-unit-7a-error"
    assert validated_response.payload["status"] == "error"
    assert validated_response.payload["text"] is None
    assert validated_response.payload["model"] == "mock-model"
    assert "mock llm unavailable" in validated_response.payload["error"]
