import asyncio

from runtime.runtime import ThalamusRuntime


async def main():

    runtime = ThalamusRuntime()

    await runtime.start()

    await runtime.publisher.publish(
        event_type="runtime.task.assign",
        source="example.publisher",
        payload={
            "task_id": "task-001",
            "objective": "Explain what Thalamus is."
        }
    )

    print("Task published.")

    await asyncio.sleep(1)

    await runtime.stop()


if __name__ == "__main__":
    asyncio.run(main())