from runtime.bus.nats_bus import NatsBus
from runtime.events.publisher import EventPublisher
from runtime.registry.registry import WorkerRegistry


class ThalamusRuntime:

    def __init__(self, servers=None):

        self.bus = NatsBus(
            servers=servers or [
                "nats://localhost:4222"
            ]
        )

        self.publisher = EventPublisher(
            self.bus
        )

        self.registry = WorkerRegistry()

    async def start(self):

        await self.bus.connect()

        #
        # registry subscriptions
        #

        await self.bus.subscribe(
            "runtime.agent.register",
            self.handle_register
        )

        await self.bus.subscribe(
            "runtime.agent.unregister",
            self.handle_unregister
        )

    async def handle_register(
        self,
        subject,
        event
    ):

        payload = event["payload"]

        self.registry.register(
            agent_id=payload["agent_id"],
            capabilities=payload["capabilities"]
        )

    async def handle_unregister(
        self,
        subject,
        event
    ):

        payload = event["payload"]

        self.registry.unregister(
            payload["agent_id"]
        )
    
    async def stop(self):

        await self.bus.nc.close()