import asyncio

import pytest

from runtime.runtime import ThalamusRuntime


@pytest.mark.asyncio
async def test_simple_task_flow(nats_container):

    runtime = ThalamusRuntime(
        servers=[nats_container]
    )

    await runtime.start()

    received_result = asyncio.Event()

    result_payload = {}

    async def handle_result(subject, event):

        nonlocal result_payload

        result_payload = event

        received_result.set()

    async def handle_task(subject, event):

        payload = event["payload"]

        await runtime.publisher.publish(
            event_type="runtime.task.result",
            source="test.worker",
            payload={
                "task_id": payload["task_id"],
                "status": "success",
                "summary": "Cognitive pulse confirmed"
            }
        )

    await runtime.bus.subscribe(
        "runtime.task.assign",
        handle_task
    )

    await runtime.bus.subscribe(
        "runtime.task.result",
        handle_result
    )

    await runtime.publisher.publish(
        event_type="runtime.task.assign",
        source="test.publisher",
        payload={
            "task_id": "task-test-001",
            "objective": "Verify runtime propagation"
        }
    )

    await asyncio.wait_for(
        received_result.wait(),
        timeout=5
    )

    assert (
        result_payload["type"]
        == "runtime.task.result"
    )

    assert (
        result_payload["payload"]["status"]
        == "success"
    )

    await runtime.stop()