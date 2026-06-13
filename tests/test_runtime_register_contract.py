import asyncio

from runtime.runtime import ThalamusRuntime


class FakeRegistry:

    def __init__(self):

        self.calls = []

    def register(
        self,
        agent_id,
        capabilities
    ):

        self.calls.append(
            (agent_id, capabilities)
        )


def test_handle_register_delegates_payload_to_registry():

    runtime = ThalamusRuntime()
    fake_registry = FakeRegistry()
    runtime.registry = fake_registry

    capabilities = [
        "shell.exec",
        "llm.reasoning"
    ]

    event = {
        "payload": {
            "agent_id": "worker-contract-1",
            "capabilities": capabilities
        }
    }

    asyncio.run(
        runtime.handle_register(
            "runtime.agent.register",
            event
        )
    )

    assert fake_registry.calls == [
        (
            "worker-contract-1",
            capabilities
        )
    ]
