import json

from runtime.events.validator import (
    validate_event
)


class EventPublisher:

    def __init__(self, bus):

        self.bus = bus

    async def publish(
        self,
        event_type: str,
        source: str,
        payload: dict,
        subject: str = None
    ):

        validated_event = validate_event(
            event_type=event_type,
            source=source,
            payload=payload
        )

        #
        # default subject
        #
        if subject is None:

            subject = event_type

        await self.bus.publish(
            subject,
            json.dumps(
                validated_event.model_dump()
            ).encode()
        )