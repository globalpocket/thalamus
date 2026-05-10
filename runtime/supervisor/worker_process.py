import subprocess
import sys


class WorkerProcess:

    def __init__(
        self,
        module="runtime.worker.simple_worker"
    ):

        self.module = module
        self.process = None

    def start(self):

        self.process = subprocess.Popen(
            [
                sys.executable,
                "-m",
                self.module
            ]
        )

        return self.process

    def stop(self):

        if self.process is None:
            return

        try:
            self.process.kill()

        except Exception:
            pass

    def is_running(self):

        if self.process is None:
            return False

        return self.process.poll() is None

    @property
    def pid(self):

        if self.process is None:
            return None

        return self.process.pid