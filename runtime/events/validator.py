from datetime import datetime, timezone
from typing import Any, Dict
from uuid import uuid4

from pydantic import ValidationError

from runtime.events.types import (
    RuntimeAgentExitPayload,
    RuntimeAgentErrorPayload,
    RuntimeAgentReadyPayload,
    RuntimeEvent,
    RuntimeLLMRequestPayload,
    RuntimeLLMResponsePayload,
    RuntimeTaskAssignPayload,
    RuntimeTaskResultPayload,
    RuntimeToolRequestPayload,
    RuntimeToolResultPayload,
)


PAYLOAD_MODELS = {
    "runtime.task.assign": RuntimeTaskAssignPayload,
    "runtime.task.result": RuntimeTaskResultPayload,
    "runtime.agent.ready": RuntimeAgentReadyPayload,
    "runtime.agent.exit": RuntimeAgentExitPayload,
    "runtime.tool.request": RuntimeToolRequestPayload,
    "runtime.tool.result": RuntimeToolResultPayload,
    "runtime.llm.request": RuntimeLLMRequestPayload,
    "runtime.llm.response": RuntimeLLMResponsePayload,
    "runtime.agent.error": RuntimeAgentErrorPayload,
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
        id=str(
            uuid4()
        ),
        type=event_type,
        subject=event_type,
        source=source,
        timestamp=datetime.now(
            timezone.utc
        ).isoformat(),
        schema="runtime.event.v1",
        payload=validated_payload.model_dump()
    )
