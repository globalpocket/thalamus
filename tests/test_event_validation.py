import json
from unittest.mock import AsyncMock

import pytest

from pydantic import ValidationError

from runtime.bus.nats_bus import NatsBus
from runtime.events.validator import (
    validate_event
)


@pytest.mark.asyncio
async def test_contract_nats_bus_publish_encodes_payload_before_publishing():

    bus = NatsBus(["nats://example:4222"])
    bus.nc.publish = AsyncMock()

    await bus.publish(
        "tasks.created",
        {
            "task_id": "t-1"
        }
    )

    bus.nc.publish.assert_awaited_once_with(
        "tasks.created",
        json.dumps({
            "task_id": "t-1"
        }).encode()
    )


def test_validate_task_assign():

    event = validate_event(
        event_type="runtime.task.assign",
        source="test",
        payload={
            "task_id": "task-001",
            "agent_id": "agent-001",
            "input": {
                "objective": "hello"
            },
            "capabilities": [
                "contract"
            ],
            "metadata": {
                "classification": "contract"
            }
        }
    )

    assert (
        event.payload["task_id"]
        == "task-001"
    )
    assert (
        event.payload["agent_id"]
        == "agent-001"
    )


def test_invalid_task_assign():

    with pytest.raises(
        ValidationError
    ):

        validate_event(
            event_type="runtime.task.assign",
            source="test",
            payload={
                "objective": "missing id"
            }
        )


def test_unknown_event():

    with pytest.raises(
        ValueError
    ):

        validate_event(
            event_type="unknown.event",
            source="test",
            payload={}
        )
