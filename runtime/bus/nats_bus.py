import json
from nats.aio.client import Client as NATS

from runtime.bus.interface import EventBus


class NatsBus(EventBus):

    def __init__(self, servers: list[str]):
        self.servers = servers
        self.nc = NATS()

    async def connect(self) -> None:
        await self.nc.connect(servers=self.servers)

    async def close(self) -> None:
        await self.nc.close()

    async def publish(self, subject: str, payload: dict) -> None:
        await self.nc.publish(
            subject,
            payload
        )

    async def subscribe(self, subject: str, handler):

        async def wrapped(msg):
            payload = json.loads(msg.data.decode())
            await handler(msg.subject, payload)

        await self.nc.subscribe(subject, cb=wrapped)

    async def request(
        self,
        subject: str,
        payload: dict,
        timeout: float = 30.0
    ) -> dict:

        response = await self.nc.request(
            subject,
            json.dumps(payload).encode(),
            timeout=timeout
        )

        return json.loads(response.data.decode())