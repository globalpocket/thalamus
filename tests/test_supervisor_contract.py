import subprocess

from runtime.supervisor.supervisor import Supervisor


class _FakeUuid:
    hex = "abcdef1234567890"


def test_contract_supervisor_spawn_worker_sets_command_env_and_returns_process(monkeypatch):
    calls = []
    fake_process = object()

    def fake_popen(command, env):
        calls.append((command, env))
        return fake_process

    monkeypatch.setattr(subprocess, "Popen", fake_popen)
    monkeypatch.setattr("runtime.supervisor.supervisor.uuid.uuid4", lambda: _FakeUuid())

    supervisor = Supervisor(nats_url="nats://contract.example:4222")

    process, agent_id = supervisor.spawn_worker(capabilities=["llm.reasoning", "tools.search"])

    assert process is fake_process
    assert agent_id == "worker-abcdef12"
    assert calls[0][0] == [subprocess.sys.executable, "-m", "runtime.worker.shell_worker"]
    assert calls[0][1]["THALAMUS_NATS_URL"] == "nats://contract.example:4222"
    assert calls[0][1]["THALAMUS_AGENT_ID"] == "worker-abcdef12"
    assert calls[0][1]["THALAMUS_CAPABILITIES"] == "llm.reasoning,tools.search"
