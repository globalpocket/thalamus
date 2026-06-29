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


@pytest.mark.asyncio
async def test_contract_nats_bus_round_trip_publish_to_subscribe(monkeypatch):
    """NATS 経由でのメッセージ送信→受信が確認できる round-trip テスト。"""
    class FakeNATS:
        def __init__(self):
            self.connected_servers = None
            self.closed = False
            self.published = []
            self.subscriptions = []

        async def connect(self, servers):
            self.connected_servers = servers

        async def close(self):
            self.closed = True

        async def publish(self, subject, payload):
            self.published.append((subject, payload))

        async def subscribe(self, subject, cb):
            self.subscriptions.append((subject, cb))

    class FakeMessage:
        def __init__(self, subject, data):
            self.subject = subject
            self.data = data

    monkeypatch.setattr(nats_bus_module, "NATS", FakeNATS)

    bus = nats_bus_module.NatsBus(["nats://localhost:4222"])
    fake_client = bus.nc

    await bus.connect()

    received_messages = []

    async def handler(subject, payload):
        received_messages.append((subject, payload))

    await bus.subscribe("events.created", handler)
    await bus.publish("events.created", {"id": "evt-1", "type": "user.signup"})

    # publish が fake_client.published に記録されていることを確認
    assert len(fake_client.published) == 1
    assert fake_client.published[0][0] == "events.created"
    assert json.loads(fake_client.published[0][1]) == {"id": "evt-1", "type": "user.signup"}

    # subscribe が fake_client.subscriptions に記録されていることを確認
    assert len(fake_client.subscriptions) == 1
    subscription_subject, wrapped_handler = fake_client.subscriptions[0]
    assert subscription_subject == "events.created"

    # フロー: publish されたメッセージを subscribe ハンドラーが受信
    await wrapped_handler(FakeMessage("events.created", b'{"id": "evt-1", "type": "user.signup"}'))

    # ハンドラーが呼び出されたことを確認
    assert len(received_messages) == 1
    assert received_messages[0][0] == "events.created"
    assert received_messages[0][1] == {"id": "evt-1", "type": "user.signup"}

    await bus.close()
    assert fake_client.closed is True


@pytest.mark.asyncio
async def test_contract_nats_bus_round_trip_multiple_subjects(monkeypatch):
    """複数の subject 間で publish → subscribe が正しく動作することを確認する round-trip テスト。"""
    class FakeNATS:
        def __init__(self):
            self.connected_servers = None
            self.closed = False
            self.published = []
            self.subscriptions = []

        async def connect(self, servers):
            self.connected_servers = servers

        async def close(self):
            self.closed = True

        async def publish(self, subject, payload):
            self.published.append((subject, payload))

        async def subscribe(self, subject, cb):
            self.subscriptions.append((subject, cb))

    class FakeMessage:
        def __init__(self, subject, data):
            self.subject = subject
            self.data = data

    monkeypatch.setattr(nats_bus_module, "NATS", FakeNATS)

    bus = nats_bus_module.NatsBus(["nats://localhost:4222"])
    fake_client = bus.nc

    await bus.connect()

    received = {"tasks.created": [], "tasks.completed": []}

    async def tasks_created_handler(subject, payload):
        received["tasks.created"].append((subject, payload))

    async def tasks_completed_handler(subject, payload):
        received["tasks.completed"].append((subject, payload))

    await bus.subscribe("tasks.created", tasks_created_handler)
    await bus.subscribe("tasks.completed", tasks_completed_handler)

    await bus.publish("tasks.created", {"task_id": "t-1", "status": "pending"})
    await bus.publish("tasks.completed", {"task_id": "t-2", "status": "done"})

    # 2つの publish が記録されている
    assert len(fake_client.published) == 2

    # 2つの subscription が記録されている
    assert len(fake_client.subscriptions) == 2

    # 各 subscription の wrapped handler を呼び出してメッセージを受信
    for subject, wrapped_handler in fake_client.subscriptions:
        sample_payload = {"task_id": "test", "status": "test"}
        if subject == "tasks.created":
            sample_payload = {"task_id": "t-1", "status": "pending"}
        elif subject == "tasks.completed":
            sample_payload = {"task_id": "t-2", "status": "done"}
        await wrapped_handler(FakeMessage(subject, json.dumps(sample_payload).encode()))

    assert len(received["tasks.created"]) == 1
    assert received["tasks.created"][0][1] == {"task_id": "t-1", "status": "pending"}
    assert len(received["tasks.completed"]) == 1
    assert received["tasks.completed"][0][1] == {"task_id": "t-2", "status": "done"}

    await bus.close()
    assert fake_client.closed is True
