import pytest

import runtime.worker.simple_worker as simple_worker


@pytest.mark.asyncio
async def test_behavior_main_bootstrap_uses_env_registers_signals_and_starts_worker(monkeypatch):
    worker_inits = []
    started_workers = []
    created_tasks = []
    signal_handlers = []

    class FakeWorker:
        def __init__(
            self,
            *,
            nats_url,
            agent_id,
            capabilities,
            command_timeout_seconds,
        ):
            self.shutdown_calls = 0
            worker_inits.append(
                {
                    "nats_url": nats_url,
                    "agent_id": agent_id,
                    "capabilities": capabilities,
                    "command_timeout_seconds": command_timeout_seconds,
                }
            )

        async def start(self):
            started_workers.append(self)

        async def shutdown(self):
            self.shutdown_calls += 1

    class FakeLoop:
        def add_signal_handler(self, sig, handler):
            signal_handlers.append((sig, handler))

    def fake_create_task(coro):
        created_tasks.append(coro)
        return object()

    monkeypatch.setenv("THALAMUS_NATS_URL", "nats://example.test:4222")
    monkeypatch.setenv("THALAMUS_AGENT_ID", "agent-main")
    monkeypatch.setenv("THALAMUS_CAPABILITIES", "shell, docker , ,python")
    monkeypatch.setenv("THALAMUS_SHELL_TIMEOUT_SECONDS", "42")
    monkeypatch.setattr(simple_worker, "ShellWorker", FakeWorker)
    monkeypatch.setattr(
        simple_worker.asyncio,
        "get_running_loop",
        lambda: FakeLoop(),
    )
    monkeypatch.setattr(
        simple_worker.asyncio,
        "create_task",
        fake_create_task,
    )

    await simple_worker.main()

    assert worker_inits == [
        {
            "nats_url": "nats://example.test:4222",
            "agent_id": "agent-main",
            "capabilities": ["shell", "docker", "python"],
            "command_timeout_seconds": 42,
        }
    ]
    assert len(started_workers) == 1
    assert [sig for sig, _ in signal_handlers] == [
        simple_worker.signal.SIGINT,
        simple_worker.signal.SIGTERM,
    ]

    signal_handlers[0][1]()

    assert len(created_tasks) == 1
    assert created_tasks[0].cr_code.co_name == "shutdown"
    created_tasks[0].close()
