from abc import ABC, abstractmethod
from typing import Callable, Awaitable, Any


MessageHandler = Callable[[str, dict], Awaitable[None]]


class EventBus(ABC):

    @abstractmethod
    async def connect(self) -> None:
        pass

    @abstractmethod
    async def close(self) -> None:
        pass

    @abstractmethod
    async def publish(self, subject: str, payload: dict) -> None:
        pass

    @abstractmethod
    async def subscribe(
        self,
        subject: str,
        handler: MessageHandler
    ) -> None:
        pass

    @abstractmethod
    async def request(
        self,
        subject: str,
        payload: dict,
        timeout: float = 30.0
    ) -> dict:
        pass