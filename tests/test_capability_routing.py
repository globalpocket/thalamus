import asyncio
import pytest

from runtime.runtime import ThalamusRuntime
from runtime.supervisor.supervisor import Supervisor


@pytest.mark.asyncio
async def test_capability_routing(
    nats_container
):

    runtime = ThalamusRuntime(
        servers=[nats_container]
    )

    await runtime.start()

    completed = asyncio.Event()

    observed = []

    async def recorder(
        subject,
        event
    ):

        observed.append(
            event["type"]
        )

        print(event)

        if (
            event["type"]
            == "runtime.task.result"
        ):
            completed.set()

    #
    # observe results
    #
    await runtime.bus.subscribe(
        "runtime.task.result",
        recorder
    )

    #
    # spawn shell worker
    #
    supervisor = Supervisor(
        nats_url=nats_container
    )

    process, agent_id = supervisor.spawn_worker(
        capabilities=[
            "tool.shell"
        ]
    )

    #
    # allow boot
    #
    await asyncio.sleep(2)

    #
    # direct task to agent
    #
    task_subject = (
        f"runtime.task.assign.{agent_id}"
    )

    #
    # send shell task
    #
    await runtime.publisher.publish(
        event_type=(
            "runtime.task.assign"
        ),
        subject=task_subject,
        source="test",
        payload={
            "task_id": "capability-001",
            "objective": (
                "execute shell cognition"
            )
        }
    )

    #
    # wait result
    #
    await asyncio.wait_for(
        completed.wait(),
        timeout=10
    )

    process.kill()

    await runtime.stop()

    assert (
        "runtime.task.result"
        in observed
    )