import json

from runtime.bus.in_memory_bus import InMemoryBus
from runtime.runtime import ThalamusRuntime


runtime = ThalamusRuntime(
    bus=InMemoryBus()
)


def decode_event(event):

    if isinstance(event, bytes):
        return json.loads(
            event.decode()
        )

    return event


async def handle_task(subject, event):

    event = decode_event(
        event
    )

    payload = event["payload"]

    objective = payload["objective"]

    print(f"Received task: {objective}")

    await runtime.publisher.publish(
        event_type="runtime.task.result",
        source="example.worker",
        scope=event.get("scope", {}),
        payload={
            "task_id": payload["task_id"],
            "status": "success",
            "summary": (
                "reference runtime completed: "
                f"{objective}"
            )
        }
    )

    print("Task completed.")


async def main():

    await runtime.start()

    await runtime.bus.subscribe(
        "runtime.task.assign",
        handle_task
    )

    print("Worker listening...")

    await runtime.publisher.publish(
        event_type="runtime.task.assign",
        source="example.publisher",
        payload={
            "task_id": "task-example-001",
            "objective": "Verify in-memory task propagation"
        }
    )

    print("Published events:")
    for subject, event in runtime.bus.published:
        print(
            subject,
            decode_event(
                event
            )["payload"]
        )


if __name__ == "__main__":
    import asyncio

    asyncio.run(
        main()
    )
