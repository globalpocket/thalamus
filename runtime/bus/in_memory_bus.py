from collections import defaultdict

from runtime.bus.interface import EventBus, MessageHandler


class InMemoryBus(EventBus):

    def __init__(self):
        self.connected = False
        self.published = []
        self.subscriptions = defaultdict(list)

    async def connect(self) -> None:
        self.connected = True

    async def close(self) -> None:
        self.connected = False

    async def publish(self, subject: str, payload: dict) -> None:
        self.published.append((subject, payload))

        for handler in self.subscriptions[subject]:
            await handler(subject, payload)

    async def subscribe(
        self,
        subject: str,
        handler: MessageHandler
    ) -> None:
        self.subscriptions[subject].append(handler)

    async def request(
        self,
        subject: str,
        payload: dict,
        timeout: float = 30.0
    ) -> dict:
        await self.publish(subject, payload)
        raise NotImplementedError(
            "InMemoryBus request/response is not implemented"
        )
