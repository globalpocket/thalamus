import asyncio

import pytest

from runtime.worker.shell_worker import ShellWorker


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


def _worker_with_fakes(**kwargs):
    worker = ShellWorker(**kwargs)
    publisher = _FakePublisher()
    bus = _FakeBus()
    worker.publisher = publisher
    worker.bus = bus
    return worker, publisher, bus


@pytest.mark.asyncio
async def test_contract_empty_command_publishes_invalid_command_without_shell_io():
    worker, publisher, bus = _worker_with_fakes(
        agent_id="worker-handle-empty-command",
        capabilities=["tool.shell"],
    )
    shell_calls = []

    async def fake_run_shell_command(command: str):
        shell_calls.append(command)
        raise AssertionError("empty command must not invoke shell execution")

    worker._run_shell_command = fake_run_shell_command

    await worker.handle_task(
        "runtime.task.assign.worker-handle-empty-command",
        {
            "payload": {
                "task_id": "task-empty-command",
                "command": "   ",
            }
        },
    )

    failure_event = publisher.published[0]
    exit_event = publisher.published[1]

    assert len(publisher.published) == 2
    assert failure_event["event_type"] == "runtime.task.result"
    assert failure_event["payload"]["task_id"] == "task-empty-command"
    assert failure_event["payload"]["status"] == "failure"
    assert failure_event["payload"]["summary"] == "empty command"
    assert failure_event["payload"]["result"] == {
        "reason": "invalid_command",
    }
    assert exit_event["event_type"] == "runtime.agent.exit"
    assert shell_calls == []
    assert bus.close_count == 1
    assert worker.running is False


@pytest.mark.asyncio
async def test_behavior_shell_returncode_controls_success_and_failure_results_without_shell_io():
    cases = [
        (
            "success",
            0,
            "shell command completed",
        ),
        (
            "failure",
            2,
            "shell command failed",
        ),
    ]

    for status, returncode, summary in cases:
        worker, publisher, bus = _worker_with_fakes(
            agent_id=f"worker-handle-{status}-command",
            capabilities=["tool.shell"],
        )
        shell_calls = []

        async def fake_run_shell_command(command: str):
            shell_calls.append(command)
            return {
                "returncode": returncode,
                "stdout": f"stdout-{status}",
                "stderr": f"stderr-{status}",
            }

        worker._run_shell_command = fake_run_shell_command

        await worker.handle_task(
            f"runtime.task.assign.worker-handle-{status}-command",
            {
                "payload": {
                    "task_id": f"task-{status}-command",
                    "objective": f"echo {status}",
                }
            },
        )

        result_event = publisher.published[0]
        exit_event = publisher.published[1]

        assert len(publisher.published) == 2
        assert result_event["event_type"] == "runtime.task.result"
        assert result_event["payload"]["task_id"] == f"task-{status}-command"
        assert result_event["payload"]["status"] == status
        assert result_event["payload"]["summary"] == summary
        assert result_event["payload"]["result"] == {
            "command": f"echo {status}",
            "returncode": returncode,
            "stdout": f"stdout-{status}",
            "stderr": f"stderr-{status}",
        }
        assert exit_event["event_type"] == "runtime.agent.exit"
        assert shell_calls == [f"echo {status}"]
        assert bus.close_count == 1
        assert worker.running is False


@pytest.mark.asyncio
async def test_behavior_shell_timeout_and_exception_publish_failure_reasons_without_shell_io():
    cases = [
        (
            "timeout",
            asyncio.TimeoutError(),
            "shell command timed out",
            {
                "command": "sleep 30",
                "reason": "timeout",
                "timeout_seconds": 3,
            },
        ),
        (
            "execution-error",
            RuntimeError("boom"),
            "shell command execution error",
            {
                "command": "sleep 30",
                "reason": "execution_error",
                "message": "boom",
            },
        ),
    ]

    for case_name, error, summary, expected_result in cases:
        worker, publisher, bus = _worker_with_fakes(
            agent_id=f"worker-handle-{case_name}",
            capabilities=["tool.shell"],
            command_timeout_seconds=3,
        )
        shell_calls = []

        async def fake_run_shell_command(command: str):
            shell_calls.append(command)
            raise error

        worker._run_shell_command = fake_run_shell_command

        await worker.handle_task(
            f"runtime.task.assign.worker-handle-{case_name}",
            {
                "payload": {
                    "task_id": f"task-{case_name}",
                    "command": "sleep 30",
                }
            },
        )

        failure_event = publisher.published[0]
        exit_event = publisher.published[1]

        assert len(publisher.published) == 2
        assert failure_event["event_type"] == "runtime.task.result"
        assert failure_event["payload"]["task_id"] == f"task-{case_name}"
        assert failure_event["payload"]["status"] == "failure"
        assert failure_event["payload"]["summary"] == summary
        assert failure_event["payload"]["result"] == expected_result
        assert exit_event["event_type"] == "runtime.agent.exit"
        assert shell_calls == ["sleep 30"]
        assert bus.close_count == 1
        assert worker.running is False
