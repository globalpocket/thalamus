import uuid
from datetime import datetime


class EventPublisher:

    def __init__(self, bus):

        self.bus = bus

    async def publish(
        self,
        event_type,
        source,
        payload,
        subject=None,
        scope=None,
        correlation_id=None
    ):

        envelope = {
            "id": str(uuid.uuid4()),
            "type": event_type,
            "source": source,
            "timestamp": datetime.utcnow().isoformat(),
            "payload": payload,
            "scope": scope or {},
            "correlation_id": correlation_id
        }

        publish_subject = subject or event_type

        await self.bus.publish(
            publish_subject,
            envelope
        )