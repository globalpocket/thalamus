from runtime.bus.nats_bus import NatsBus
from runtime.events.publisher import EventPublisher


class ThalamusRuntime:

    def __init__(self):

        self.bus = NatsBus(
            servers=["nats://localhost:4222"]
        )

        self.publisher = EventPublisher(
            self.bus
        )

    async def start(self):
        await self.bus.connect()

    async def stop(self):
        await self.bus.close()