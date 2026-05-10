import asyncio
import os
import signal
import sys
import uuid

from runtime.bus.nats_bus import NatsBus
from runtime.events.publisher import EventPublisher


class SimpleWorker:

    def __init__(
        self,
        nats_url="nats://localhost:4222",
        agent_id=None,
        capabilities=None
    ):

        self.nats_url = nats_url

        self.agent_id = (
            agent_id
            or f"worker-{uuid.uuid4().hex[:8]}"
        )

        self.capabilities = (
            capabilities
            or []
        )

        self.bus = NatsBus(
            servers=[self.nats_url]
        )

        self.publisher = EventPublisher(
            self.bus
        )

        self.running = True

    async def start(self):

        await self.bus.connect()

        #
        # announce ready
        #
        await self.publisher.publish(
            event_type="runtime.agent.ready",
            source=self.agent_id,
            payload={
                "agent_id": self.agent_id,
                "capabilities": self.capabilities
            }
        )

        #
        # subscribe direct inbox
        #
        subject = (
            f"runtime.task.assign.{self.agent_id}"
        )

        await self.bus.subscribe(
            subject,
            self.handle_task
        )

        #
        # keep alive
        #
        while self.running:
            await asyncio.sleep(1)

    async def handle_task(
        self,
        subject,
        event
    ):

        payload = event["payload"]

        await asyncio.sleep(0.25)

        #
        # publish result
        #
        await self.publisher.publish(
            event_type="runtime.task.result",
            source=self.agent_id,
            payload={
                "task_id": payload["task_id"],
                "result": "cognition complete",
                "worker_id": self.agent_id
            }
        )

        #
        # publish exit
        #
        await self.publisher.publish(
            event_type="runtime.agent.exit",
            source=self.agent_id,
            payload={
                "agent_id": self.agent_id
            }
        )

        self.running = False

        #
        # close transport cleanly
        #
        await self.bus.close()

    async def shutdown(self):

        self.running = False

        try:
            await self.bus.close()
        except Exception:
            pass


async def main():

    nats_url = os.getenv(
        "THALAMUS_NATS_URL",
        "nats://localhost:4222"
    )

    agent_id = os.getenv(
        "THALAMUS_AGENT_ID"
    )

    capabilities_raw = os.getenv(
        "THALAMUS_CAPABILITIES",
        ""
    )

    capabilities = [
        item.strip()
        for item in capabilities_raw.split(",")
        if item.strip()
    ]

    worker = SimpleWorker(
        nats_url=nats_url,
        agent_id=agent_id,
        capabilities=capabilities
    )

    loop = asyncio.get_running_loop()

    def handle_signal(*_):

        asyncio.create_task(
            worker.shutdown()
        )

    for sig in (
        signal.SIGINT,
        signal.SIGTERM
    ):
        loop.add_signal_handler(
            sig,
            handle_signal
        )

    await worker.start()


if __name__ == "__main__":
    asyncio.run(main())