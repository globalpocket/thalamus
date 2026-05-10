import asyncio
import os

from runtime.runtime import ThalamusRuntime


AGENT_ID = "worker-001"


async def main():

    nats_url = os.getenv(
        "THALAMUS_NATS_URL",
        "nats://localhost:4222"
    )

    runtime = ThalamusRuntime(
        servers=[nats_url]
    )

    await runtime.start()

    shutdown_event = asyncio.Event()

    #
    # announce ready
    #
    await runtime.publisher.publish(
        event_type="runtime.agent.ready",
        source=AGENT_ID,
        payload={
            "agent_id": AGENT_ID
        }
    )

    async def handle_task(subject, event):

        #
        # simulate task execution
        #
        await runtime.publisher.publish(
            event_type="runtime.task.result",
            source=AGENT_ID,
            payload={
                "task_id": event["payload"].get("task_id"),
                "status": "success"
            }
        )

        #
        # announce exit
        #
        await runtime.publisher.publish(
            event_type="runtime.agent.exit",
            source=AGENT_ID,
            payload={
                "agent_id": AGENT_ID
            }
        )

        #
        # graceful shutdown
        #
        shutdown_event.set()

    #
    # subscribe task channel
    #
    await runtime.bus.subscribe(
        "runtime.task.assign",
        handle_task
    )

    #
    # wait until shutdown requested
    #
    await shutdown_event.wait()

    #
    # flush + disconnect
    #
    await runtime.stop()


if __name__ == "__main__":
    asyncio.run(main())