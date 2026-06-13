import json

import pytest

import runtime.bus.nats_bus as nats_bus_module


@pytest.mark.asyncio
async def test_contract_nats_bus_delegates_adapter_operations_with_json_boundaries(monkeypatch):
    class FakeNATS:
        instances = []

        def __init__(self):
            self.connected_servers = None
            self.closed = False
            self.published = []
            self.subscriptions = []
            self.requests = []
            FakeNATS.instances.append(self)

        async def connect(self, servers):
            self.connected_servers = servers

        async def close(self):
            self.closed = True

        async def publish(self, subject, payload):
            self.published.append((subject, payload))

        async def subscribe(self, subject, cb):
            self.subscriptions.append((subject, cb))

        async def request(self, subject, payload, timeout):
            self.requests.append((subject, payload, timeout))

            class FakeResponse:
                data = b'{"accepted": true}'

            return FakeResponse()

    class FakeMessage:
        subject = "events.created"
        data = b'{"id": "evt-1"}'

    monkeypatch.setattr(nats_bus_module, "NATS", FakeNATS)

    bus = nats_bus_module.NatsBus(["nats://localhost:4222"])
    fake_client = FakeNATS.instances[0]

    await bus.connect()
    await bus.publish("events.created", {"id": "evt-1"})


    handled = []

    async def handler(subject, payload):
        handled.append((subject, payload))

    await bus.subscribe("events.created", handler)
    subscription_subject, wrapped_handler = fake_client.subscriptions[0]
    await wrapped_handler(FakeMessage())
    response = await bus.request(
        "events.lookup",
        {"id": "evt-1"},
        timeout=1.5,
    )
    await bus.close()

    assert fake_client.connected_servers == ["nats://localhost:4222"]
    assert fake_client.published == [
        ("events.created", json.dumps({"id": "evt-1"}).encode()),
    ]
    assert subscription_subject == "events.created"
    assert handled == [("events.created", {"id": "evt-1"})]
    assert fake_client.requests == [
        ("events.lookup", b'{"id": "evt-1"}', 1.5),
    ]
    assert response == {"accepted": True}
    assert fake_client.closed is True
