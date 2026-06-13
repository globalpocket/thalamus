import pytest
from types import SimpleNamespace

from runtime.worker.shell_worker import ShellWorker


class _FakePublisher:
    def __init__(self):
        self.published = []

    async def publish(self, **kwargs):
        self.published.append(SimpleNamespace(**kwargs))


class _FakeBus:
    def __init__(self):
        self.close_count = 0

    async def close(self):
        self.close_count += 1


@pytest.mark.asyncio
async def test_contract_missing_tool_shell_capability_publishes_failure_and_exit():
    worker = ShellWorker(
        agent_id="worker-contract-missing-shell",
        capabilities=[],
    )
    publisher = _FakePublisher()
    bus = _FakeBus()
    worker.publisher = publisher
    worker.bus = bus

    event = {
        "payload": {
            "task_id": "task-contract-missing-shell",
            "objective": "echo should not run",
        }
    }

    await worker.handle_task(
        "runtime.task.assign.worker-contract-missing-shell",
        event,
    )

    failure_event = publisher.published[0]
    exit_event = publisher.published[1]

    assert failure_event.payload["status"] == "failure"
    assert (
        failure_event.payload["summary"]
        == "tool.shell capability is not available"
    )
    assert failure_event.payload["result"]["reason"] == "missing_capability"
    assert failure_event.payload["result"]["required"] == "tool.shell"
    assert exit_event.event_type == "runtime.agent.exit"
    assert bus.close_count == 1
    assert worker.running is False
