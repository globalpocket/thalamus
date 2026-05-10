import uuid
from datetime import datetime, timezone


class EventPublisher:

    def __init__(self, bus):
        self.bus = bus

    async def publish(
        self,
        event_type: str,
        source: str,
        payload: dict,
        scope: dict | None = None,
        refs: dict | None = None
    ):

        event = {
            "id": f"evt_{uuid.uuid4().hex}",
            "type": event_type,
            "timestamp": datetime.now(
                timezone.utc
            ).isoformat(),
            "source": source,
            "scope": scope or {},
            "refs": refs or {},
            "payload": payload
        }

        await self.bus.publish(
            event_type,
            event
        )