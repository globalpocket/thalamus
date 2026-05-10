import pytest

from pydantic import ValidationError

from runtime.events.validator import (
    validate_event
)


def test_validate_task_assign():

    event = validate_event(
        event_type="runtime.task.assign",
        source="test",
        payload={
            "task_id": "task-001",
            "objective": "hello"
        }
    )

    assert (
        event.payload["task_id"]
        == "task-001"
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