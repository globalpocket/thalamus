import pytest

from runtime.worker.shell_worker import ShellWorker


@pytest.mark.asyncio
async def test_regression_run_shell_command_success_uses_exec_argv_without_shell_boundary(monkeypatch):
    worker = ShellWorker(
        agent_id="worker-run-shell-success",
        capabilities=["tool.shell"],
        command_timeout_seconds=7,
    )
    captured = {}

    class FakeProcess:
        returncode = 0

        async def communicate(self):
            captured["communicate_called"] = True
            return b"ok\n", b""

    async def fake_create_subprocess_exec(*argv, stdout, stderr):
        captured["argv"] = argv
        captured["stdout_pipe"] = stdout
        captured["stderr_pipe"] = stderr
        return FakeProcess()

    async def fail_if_shell_used(*args, **kwargs):
        raise AssertionError("unsafe shell boundary must not be used")

    async def fake_wait_for(awaitable, timeout):
        captured["timeout"] = timeout
        return await awaitable

    monkeypatch.setattr(
        "runtime.worker.shell_worker.asyncio.create_subprocess_exec",
        fake_create_subprocess_exec,
    )
    monkeypatch.setattr(
        "runtime.worker.shell_worker.asyncio.create_subprocess_shell",
        fail_if_shell_used,
    )
    monkeypatch.setattr(
        "runtime.worker.shell_worker.asyncio.wait_for",
        fake_wait_for,
    )

    result = await worker._run_shell_command("printf ok")

    assert captured["argv"] == ("printf", "ok")
    assert captured["timeout"] == 7
    assert captured["communicate_called"] is True
    assert result == {
        "returncode": 0,
        "stdout": "ok\n",
        "stderr": "",
    }
