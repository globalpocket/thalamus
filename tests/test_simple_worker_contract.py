import asyncio

import pytest

from runtime.worker.simple_worker import ShellWorker


class _FakePublisher:
    def __init__(self):
        self.published = []

    async def publish(self, **kwargs):
        self.published.append(kwargs)


class _FakeBus:
    def __init__(self):
        self.close_count = 0

    async def close(self):
        self.close_count += 1


class _NoShellWorker(ShellWorker):
    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.shell_call_count = 0

    async def _run_shell_command(self, command: str):
        self.shell_call_count += 1
        raise AssertionError(
            "shell command must not be invoked without tool.shell capability"
        )


@pytest.mark.asyncio
async def test_contract_simple_worker_missing_tool_shell_capability_publishes_failure_and_exit():
    worker = _NoShellWorker(
        agent_id="worker-simple-missing-shell",
        capabilities=[],
    )
    publisher = _FakePublisher()
    bus = _FakeBus()
    worker.publisher = publisher
    worker.bus = bus

    event = {
        "payload": {
            "task_id": "task-1",
            "command": "echo hi",
        }
    }

    await worker.handle_task(
        "runtime.task.assign.worker-simple-missing-shell",
        event,
    )

    failure_event = publisher.published[0]
    exit_event = publisher.published[1]

    assert len(publisher.published) == 2
    assert failure_event["payload"]["status"] == "failure"
    assert (
        failure_event["payload"]["summary"]
        == "tool.shell capability is not available"
    )
    assert failure_event["payload"]["result"]["reason"] == "missing_capability"
    assert failure_event["payload"]["result"]["required"] == "tool.shell"
    assert exit_event["event_type"] == "runtime.agent.exit"
    assert bus.close_count == 1
    assert worker.running is False
    assert worker.shell_call_count == 0


@pytest.mark.asyncio
async def test_contract_simple_worker_empty_command_publishes_failure_and_exit_without_shell_io():
    worker = _NoShellWorker(
        agent_id="worker-simple-empty-command",
        capabilities=["tool.shell"],
    )
    publisher = _FakePublisher()
    bus = _FakeBus()
    worker.publisher = publisher
    worker.bus = bus

    event = {
        "payload": {
            "task_id": "task-empty-command",
            "command": "   ",
        }
    }

    await worker.handle_task(
        "runtime.task.assign.worker-simple-empty-command",
        event,
    )

    failure_event = publisher.published[0]
    exit_event = publisher.published[1]

    assert len(publisher.published) == 2
    assert failure_event["payload"]["status"] == "failure"
    assert failure_event["payload"]["summary"] == "empty command"
    assert failure_event["payload"]["result"]["reason"] == "invalid_command"
    assert exit_event["event_type"] == "runtime.agent.exit"
    assert bus.close_count == 1
    assert worker.running is False
    assert worker.shell_call_count == 0


@pytest.mark.asyncio
async def test_behavior_simple_worker_valid_shell_command_success_publishes_result_and_exit_without_shell_io():
    class _SuccessfulShellWorker(ShellWorker):
        def __init__(self, **kwargs):
            super().__init__(**kwargs)
            self.shell_call_count = 0

        async def _run_shell_command(self, command: str):
            self.shell_call_count += 1
            return {
                "returncode": 0,
                "stdout": "ok",
                "stderr": "",
            }

    worker = _SuccessfulShellWorker(
        agent_id="worker-simple-success-shell",
        capabilities=["tool.shell"],
    )
    publisher = _FakePublisher()
    bus = _FakeBus()
    worker.publisher = publisher
    worker.bus = bus

    event = {
        "payload": {
            "task_id": "task-success-command",
            "command": "echo ok",
        }
    }

    await worker.handle_task(
        "runtime.task.assign.worker-simple-success-shell",
        event,
    )

    result_event = publisher.published[0]
    exit_event = publisher.published[1]

    assert len(publisher.published) == 2
    assert result_event["payload"]["status"] == "success"
    assert result_event["payload"]["summary"] == "shell command completed"
    assert result_event["payload"]["result"]["command"] == "echo ok"
    assert result_event["payload"]["result"]["stdout"] == "ok"
    assert result_event["payload"]["result"]["stderr"] == ""
    assert result_event["payload"]["result"]["returncode"] == 0
    assert exit_event["event_type"] == "runtime.agent.exit"
    assert bus.close_count == 1
    assert worker.running is False
    assert worker.shell_call_count == 1


@pytest.mark.asyncio
async def test_regression_simple_worker_run_shell_command_success_uses_exec_argv_without_shell_boundary(monkeypatch):
    worker = ShellWorker(
        agent_id="worker-simple-run-shell-success",
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
        "runtime.worker.simple_worker.asyncio.create_subprocess_exec",
        fake_create_subprocess_exec,
    )
    monkeypatch.setattr(
        "runtime.worker.simple_worker.asyncio.create_subprocess_shell",
        fail_if_shell_used,
    )
    monkeypatch.setattr(
        "runtime.worker.simple_worker.asyncio.wait_for",
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


@pytest.mark.asyncio
async def test_behavior_simple_worker_nonzero_shell_returncode_publishes_failure_and_exit_without_shell_io():
    class _FailedShellWorker(ShellWorker):
        def __init__(self, **kwargs):
            super().__init__(**kwargs)
            self.shell_call_count = 0

        async def _run_shell_command(self, command: str):
            self.shell_call_count += 1
            return {
                "returncode": 2,
                "stdout": "partial output",
                "stderr": "command failed",
            }

    worker = _FailedShellWorker(
        agent_id="worker-simple-failed-shell",
        capabilities=["tool.shell"],
    )
    publisher = _FakePublisher()
    bus = _FakeBus()
    worker.publisher = publisher
    worker.bus = bus

    event = {
        "payload": {
            "task_id": "task-failed-command",
            "command": "false",
        }
    }

    await worker.handle_task(
        "runtime.task.assign.worker-simple-failed-shell",
        event,
    )

    failure_event = publisher.published[0]
    exit_event = publisher.published[1]

    assert len(publisher.published) == 2
    assert failure_event["payload"]["status"] == "failure"
    assert failure_event["payload"]["summary"] == "shell command failed"
    assert failure_event["payload"]["result"]["command"] == "false"
    assert failure_event["payload"]["result"]["stdout"] == "partial output"
    assert failure_event["payload"]["result"]["stderr"] == "command failed"
    assert failure_event["payload"]["result"]["returncode"] == 2
    assert exit_event["event_type"] == "runtime.agent.exit"
    assert bus.close_count == 1
    assert worker.running is False
    assert worker.shell_call_count == 1


@pytest.mark.asyncio
async def test_behavior_simple_worker_shell_timeout_publishes_failure_and_exit_without_shell_io():
    class _TimeoutShellWorker(ShellWorker):
        def __init__(self, **kwargs):
            super().__init__(**kwargs)
            self.shell_call_count = 0

        async def _run_shell_command(self, command: str):
            self.shell_call_count += 1
            raise asyncio.TimeoutError

    worker = _TimeoutShellWorker(
        agent_id="worker-simple-timeout-shell",
        capabilities=["tool.shell"],
        command_timeout_seconds=7,
    )
    publisher = _FakePublisher()
    bus = _FakeBus()
    worker.publisher = publisher
    worker.bus = bus

    event = {
        "payload": {
            "task_id": "task-timeout-command",
            "command": "sleep 30",
        }
    }

    await worker.handle_task(
        "runtime.task.assign.worker-simple-timeout-shell",
        event,
    )

    failure_event = publisher.published[0]
    exit_event = publisher.published[1]

    assert len(publisher.published) == 2
    assert failure_event["payload"]["status"] == "failure"
    assert failure_event["payload"]["summary"] == "shell command timed out"
    assert failure_event["payload"]["result"]["command"] == "sleep 30"
    assert failure_event["payload"]["result"]["reason"] == "timeout"
    assert failure_event["payload"]["result"]["timeout_seconds"] == 7
    assert exit_event["event_type"] == "runtime.agent.exit"
    assert bus.close_count == 1
    assert worker.running is False
    assert worker.shell_call_count == 1


@pytest.mark.asyncio
async def test_behavior_simple_worker_shell_execution_exception_publishes_failure_and_exit_without_shell_io():
    class _ExecutionErrorShellWorker(ShellWorker):
        def __init__(self, **kwargs):
            super().__init__(**kwargs)
            self.shell_call_count = 0

        async def _run_shell_command(self, command: str):
            self.shell_call_count += 1
            raise RuntimeError("subprocess transport failed")

    worker = _ExecutionErrorShellWorker(
        agent_id="worker-simple-execution-error-shell",
        capabilities=["tool.shell"],
    )
    publisher = _FakePublisher()
    bus = _FakeBus()
    worker.publisher = publisher
    worker.bus = bus

    event = {
        "payload": {
            "task_id": "task-execution-error-command",
            "command": "echo boom",
        }
    }

    await worker.handle_task(
        "runtime.task.assign.worker-simple-execution-error-shell",
        event,
    )

    failure_event = publisher.published[0]
    exit_event = publisher.published[1]

    assert len(publisher.published) == 2
    assert failure_event["payload"]["status"] == "failure"
    assert failure_event["payload"]["summary"] == "shell command execution error"
    assert failure_event["payload"]["result"]["command"] == "echo boom"
    assert failure_event["payload"]["result"]["reason"] == "execution_error"
    assert failure_event["payload"]["result"]["message"] == "subprocess transport failed"
    assert exit_event["event_type"] == "runtime.agent.exit"
    assert bus.close_count == 1
    assert worker.running is False
    assert worker.shell_call_count == 1
