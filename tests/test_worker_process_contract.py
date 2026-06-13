import subprocess

from runtime.supervisor.worker_process import WorkerProcess


class _FakeProcess:
    def __init__(self, pid=12345, poll_result=None, raises_on_kill=False):
        self.pid = pid
        self.poll_result = poll_result
        self.raises_on_kill = raises_on_kill
        self.kill_calls = 0

    def poll(self):
        return self.poll_result

    def kill(self):
        self.kill_calls += 1
        if self.raises_on_kill:
            raise subprocess.SubprocessError("fake kill failure")


def test_contract_worker_process_is_inert_before_start():
    worker = WorkerProcess(module="runtime.worker.fake_worker")

    assert worker.is_running() is False
    assert worker.pid is None
    assert worker.stop() is None


def test_contract_worker_process_start_delegates_module_command(monkeypatch):
    calls = []
    fake_process = _FakeProcess(pid=24680, poll_result=None)

    def fake_popen(command):
        calls.append(command)
        return fake_process

    monkeypatch.setattr(subprocess, "Popen", fake_popen)

    worker = WorkerProcess(module="runtime.worker.fake_worker")

    process = worker.start()

    assert process is fake_process
    assert calls == [[subprocess.sys.executable, "-m", "runtime.worker.fake_worker"]]
    assert worker.is_running() is True
    assert worker.pid == 24680


def test_contract_worker_process_stop_suppresses_kill_errors():
    fake_process = _FakeProcess(raises_on_kill=True)
    worker = WorkerProcess(module="runtime.worker.fake_worker")
    worker.process = fake_process

    assert worker.stop() is None
    assert fake_process.kill_calls == 1
