import subprocess
import sys
import os
import uuid


class Supervisor:

    def __init__(
        self,
        nats_url="nats://localhost:4222"
    ):

        self.nats_url = nats_url

    def spawn_worker(
        self,
        capabilities=None
    ):

        capabilities = capabilities or [
            "llm.reasoning"
        ]

        agent_id = (
            f"worker-{uuid.uuid4().hex[:8]}"
        )

        env = os.environ.copy()

        env["THALAMUS_NATS_URL"] = (
            self.nats_url
        )

        env["THALAMUS_AGENT_ID"] = (
            agent_id
        )

        env["THALAMUS_CAPABILITIES"] = (
            ",".join(capabilities)
        )

        process = subprocess.Popen(
            [
                sys.executable,
                "-m",
                "runtime.worker.simple_worker"
            ],
            env=env
        )

        return process, agent_id