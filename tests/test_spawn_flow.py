import asyncio

import pytest

from runtime.runtime import ThalamusRuntime
from runtime.supervisor.supervisor import Supervisor


@pytest.mark.asyncio
async def test_spawn_flow(nats_container):

    runtime = ThalamusRuntime(
        servers=[nats_container]
    )

    await runtime.start()

    observed = []

    completed = asyncio.Event()

    async def recorder(subject, event):

        event_type = event["type"]

        observed.append(event_type)

        print(f"observed: {event_type}")

        #
        # worker finished lifecycle
        #
        if event_type == "runtime.agent.exit":
            completed.set()

    #
    # subscribe lifecycle events
    #
    await runtime.bus.subscribe(
        "runtime.agent.ready",
        recorder
    )

    await runtime.bus.subscribe(
        "runtime.task.result",
        recorder
    )

    await runtime.bus.subscribe(
        "runtime.agent.exit",
        recorder
    )

    #
    # spawn worker
    #
    supervisor = Supervisor(
        nats_url=nats_container
    )

    process = supervisor.spawn_worker()

    #
    # allow worker boot
    #
    await asyncio.sleep(2)

    #
    # assign task
    #
    await runtime.publisher.publish(
        event_type="runtime.task.assign",
        source="test.supervisor",
        payload={
            "task_id": "task-spawn-001",
            "objective": "verify spawned worker"
        }
    )

    #
    # wait for worker exit
    #
    await asyncio.wait_for(
        completed.wait(),
        timeout=10
    )

    #
    # verify lifecycle
    #
    assert observed == [
        "runtime.agent.ready",
        "runtime.task.result",
        "runtime.agent.exit"
    ]

    #
    # cleanup
    #
    process.kill()

    await runtime.stop()