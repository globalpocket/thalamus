import subprocess
import sys
import os


class Supervisor:

    def __init__(self, nats_url):

        self.nats_url = nats_url

        self.processes = []

    def spawn_worker(self):

        env = os.environ.copy()

        env["THALAMUS_NATS_URL"] = self.nats_url

        process = subprocess.Popen(
            [
                sys.executable,
                "-m",
                "runtime.worker.simple_worker"
            ],
            env=env
        )

        self.processes.append(process)

        return process

    def stop_all(self):

        for process in self.processes:

            try:
                process.kill()

            except Exception:
                pass

        self.processes.clear()