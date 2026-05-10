import asyncio
import json

from runtime.runtime import ThalamusRuntime


runtime = ThalamusRuntime()


async def handle_result(subject, event):

    print("\n=== TASK RESULT ===")
    print(json.dumps(event, indent=2))


async def main():

    await runtime.start()

    await runtime.bus.subscribe(
        "runtime.task.result",
        handle_result
    )

    print("Listening for task results...")

    while True:
        await asyncio.sleep(1)


if __name__ == "__main__":
    asyncio.run(main())