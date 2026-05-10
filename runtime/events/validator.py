from typing import Any, Dict

from pydantic import ValidationError

from runtime.events.types import (
    RuntimeAgentExitPayload,
    RuntimeAgentReadyPayload,
    RuntimeEvent,
    RuntimeTaskAssignPayload,
    RuntimeTaskResultPayload,
)


PAYLOAD_MODELS = {
    "runtime.task.assign": RuntimeTaskAssignPayload,
    "runtime.task.result": RuntimeTaskResultPayload,
    "runtime.agent.ready": RuntimeAgentReadyPayload,
    "runtime.agent.exit": RuntimeAgentExitPayload,
}


def validate_event(
    event_type: str,
    source: str,
    payload: Dict[str, Any]
) -> RuntimeEvent:

    if event_type not in PAYLOAD_MODELS:

        raise ValueError(
            f"unknown event type: {event_type}"
        )

    payload_model = PAYLOAD_MODELS[
        event_type
    ]

    validated_payload = payload_model(
        **payload
    )

    return RuntimeEvent(
        type=event_type,
        source=source,
        payload=validated_payload.model_dump()
    )