import asyncio

from runtime.runtime import ThalamusRuntime
from runtime.cognition.cingulater_client import (
    CingulaterClient
)


runtime = ThalamusRuntime()

llm = CingulaterClient(
    base_url="http://localhost:8000",
    api_key="dummy"
)


async def handle_task(subject, event):

    payload = event["payload"]

    objective = payload["objective"]

    print(f"Received task: {objective}")

    llm_response = await llm.chat(
        model="gpt-4o-mini",
        messages=[
            {
                "role": "user",
                "content": objective
            }
        ]
    )

    content = (
        llm_response["choices"][0]
        ["message"]["content"]
    )

    await runtime.publisher.publish(
        event_type="runtime.task.result",
        source="example.worker",
        scope=event.get("scope", {}),
        payload={
            "task_id": payload["task_id"],
            "status": "success",
            "summary": content
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

    while True:
        await asyncio.sleep(1)


if __name__ == "__main__":
    asyncio.run(main())