import asyncio
import os
import signal
import shlex
import uuid

from runtime.bus.nats_bus import NatsBus
from runtime.events.publisher import EventPublisher


class ShellWorker:

    def __init__(
        self,
        nats_url="nats://localhost:4222",
        agent_id=None,
        capabilities=None,
        command_timeout_seconds=15,
    ):

        self.nats_url = nats_url

        self.agent_id = (
            agent_id
            or f"worker-{uuid.uuid4().hex[:8]}"
        )

        self.capabilities = (
            capabilities
            or []
        )

        self.command_timeout_seconds = (
            command_timeout_seconds
        )

        self.bus = NatsBus(
            servers=[self.nats_url]
        )

        self.publisher = EventPublisher(
            self.bus
        )

        self.running = True

    async def start(self):

        await self.bus.connect()

        await self.publisher.publish(
            event_type="runtime.agent.ready",
            source=self.agent_id,
            payload={
                "agent_id": self.agent_id,
                "capabilities": self.capabilities
            }
        )

        subject = (
            f"runtime.task.assign.{self.agent_id}"
        )

        await self.bus.subscribe(
            subject,
            self.handle_task
        )

        while self.running:
            await asyncio.sleep(1)

    async def _run_shell_command(
        self,
        command: str,
    ):

        argv = shlex.split(command)

        process = await asyncio.create_subprocess_exec(
            *argv,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )

        try:
            stdout, stderr = await asyncio.wait_for(
                process.communicate(),
                timeout=self.command_timeout_seconds,
            )
        except asyncio.TimeoutError:
            process.kill()
            await process.wait()
            raise

        return {
            "returncode": process.returncode,
            "stdout": stdout.decode("utf-8", errors="replace"),
            "stderr": stderr.decode("utf-8", errors="replace"),
        }

    async def handle_task(
        self,
        subject,
        event
    ):

        payload = event["payload"]
        task_id = payload["task_id"]

        command = (
            payload.get("command")
            or payload.get("objective", "")
        ).strip()

        status = "success"
        summary = ""
        result = {}

        if "tool.shell" not in self.capabilities:
            status = "failure"
            summary = "tool.shell capability is not available"
            result = {
                "reason": "missing_capability",
                "required": "tool.shell",
            }
        elif not command:
            status = "failure"
            summary = "empty command"
            result = {
                "reason": "invalid_command",
            }
        else:
            try:
                shell_result = await self._run_shell_command(
                    command
                )

                result = {
                    "command": command,
                    **shell_result,
                }

                if shell_result["returncode"] == 0:
                    summary = "shell command completed"
                    status = "success"
                else:
                    summary = "shell command failed"
                    status = "failure"

            except asyncio.TimeoutError:
                status = "failure"
                summary = "shell command timed out"
                result = {
                    "command": command,
                    "reason": "timeout",
                    "timeout_seconds": (
                        self.command_timeout_seconds
                    ),
                }
            except Exception as exc:
                status = "failure"
                summary = "shell command execution error"
                result = {
                    "command": command,
                    "reason": "execution_error",
                    "message": str(exc),
                }

        await self.publisher.publish(
            event_type="runtime.task.result",
            source=self.agent_id,
            payload={
                "task_id": task_id,
                "status": status,
                "summary": summary,
                "result": result,
            }
        )

        await self.publisher.publish(
            event_type="runtime.agent.exit",
            source=self.agent_id,
            payload={
                "agent_id": self.agent_id
            }
        )

        self.running = False
        await self.bus.close()

    async def shutdown(self):

        self.running = False

        try:
            await self.bus.close()
        except Exception:
            pass


async def main():

    nats_url = os.getenv(
        "THALAMUS_NATS_URL",
        "nats://localhost:4222"
    )

    agent_id = os.getenv(
        "THALAMUS_AGENT_ID"
    )

    capabilities_raw = os.getenv(
        "THALAMUS_CAPABILITIES",
        ""
    )

    timeout_raw = os.getenv(
        "THALAMUS_SHELL_TIMEOUT_SECONDS",
        "15",
    )

    capabilities = [
        item.strip()
        for item in capabilities_raw.split(",")
        if item.strip()
    ]

    worker = ShellWorker(
        nats_url=nats_url,
        agent_id=agent_id,
        capabilities=capabilities,
        command_timeout_seconds=int(timeout_raw),
    )

    loop = asyncio.get_running_loop()

    def handle_signal(*_):

        asyncio.create_task(
            worker.shutdown()
        )

    for sig in (
        signal.SIGINT,
        signal.SIGTERM
    ):
        loop.add_signal_handler(
            sig,
            handle_signal
        )

    await worker.start()


if __name__ == "__main__":
    asyncio.run(main())
