from runtime.bus.nats_bus import NatsBus
from runtime.events.publisher import EventPublisher
import json

from runtime.registry.registry import WorkerRegistry
from inspect import isawaitable


class ThalamusRuntime:

    def __init__(self, servers=None, bus=None):

        self.bus = bus or NatsBus(
            servers=servers or [
                "nats://localhost:4222"
            ]
        )

        self.publisher = EventPublisher(
            self.bus
        )

        self.registry = WorkerRegistry()

        self.task_states = {}

        self.tools = {}

        self.llm = None

    async def start(self):

        await self.bus.connect()

        #
        # registry subscriptions
        #

        await self.bus.subscribe(
            "runtime.agent.register",
            self.handle_register
        )

        await self.bus.subscribe(
            "runtime.agent.unregister",
            self.handle_unregister
        )

        await self.bus.subscribe(
            "runtime.agent.ready",
            self.handle_agent_ready
        )

        await self.bus.subscribe(
            "runtime.agent.exit",
            self.handle_agent_exit
        )

        await self.bus.subscribe(
            "runtime.agent.error",
            self.handle_agent_error
        )

        await self.bus.subscribe(
            "runtime.task.assign",
            self.handle_task_assign
        )

        await self.bus.subscribe(
            "runtime.task.result",
            self.handle_task_result
        )

        await self.bus.subscribe(
            "runtime.tool.request",
            self.handle_tool_request
        )

        await self.bus.subscribe(
            "runtime.llm.request",
            self.handle_llm_request
        )

    async def handle_task_assign(
        self,
        subject,
        event
    ):

        payload = event["payload"]
        task_id = payload["task_id"]

        self.task_states[task_id] = {
            "task_id": task_id,
            "agent_id": payload["agent_id"],
            "status": "assigned",
            "input": payload["input"],
            "capabilities": payload["capabilities"],
            "metadata": payload["metadata"]
        }

    async def handle_task_result(
        self,
        subject,
        event
    ):

        payload = event["payload"]
        task_id = payload["task_id"]
        task_state = self.task_states.setdefault(
            task_id,
            {
                "task_id": task_id
            }
        )

        task_state["status"] = payload["status"]

        for key in (
            "summary",
            "result",
            "error"
        ):
            if key in payload:
                task_state[key] = payload[key]

    async def handle_register(
        self,
        subject,
        event
    ):

        payload = event["payload"]

        self.registry.register(
            agent_id=payload["agent_id"],
            capabilities=payload["capabilities"]
        )

    async def handle_unregister(
        self,
        subject,
        event
    ):

        payload = event["payload"]

        self.registry.unregister(
            payload["agent_id"]
        )

    async def handle_agent_ready(
        self,
        subject,
        event
    ):

        payload = event["payload"]

        self.registry.mark_ready(
            agent_id=payload["agent_id"],
            capabilities=payload["capabilities"]
        )

    async def handle_agent_exit(
        self,
        subject,
        event
    ):

        payload = event["payload"]

        self.registry.mark_exited(
            agent_id=payload["agent_id"],
            reason=payload.get("reason")
        )

    async def handle_agent_error(
        self,
        subject,
        event
    ):

        payload = event["payload"]

        self.registry.mark_error(
            agent_id=payload["agent_id"],
            task_id=payload.get("task_id"),
            error=payload["error"]
        )

    async def handle_tool_request(
        self,
        subject,
        event
    ):

        payload = event["payload"]
        task_id = payload["task_id"]
        task_state = self.task_states.get(
            task_id
        )
        if task_state is not None:
            task_state["status"] = "running"

        capability = payload["capability"]
        tool = self.tools.get(
            capability
        )

        if tool is None:
            result_payload = {
                "request_id": payload["request_id"],
                "task_id": task_id,
                "status": "error",
                "output": None,
                "error": f"tool not registered: {capability}"
            }
        else:
            try:
                output = tool(
                    payload["input"]
                )
                if isawaitable(
                    output
                ):
                    output = await output
                result_payload = {
                    "request_id": payload["request_id"],
                    "task_id": task_id,
                    "status": "success",
                    "output": output,
                    "error": None
                }
            except Exception as exc:
                result_payload = {
                    "request_id": payload["request_id"],
                    "task_id": task_id,
                    "status": "error",
                    "output": None,
                    "error": str(
                        exc
                    )
                }

        request_event_id = event.get(
            "id"
        )

        class CapturingBus:

            def __init__(self):
                self.event = None

            async def publish(self, subject, payload):
                self.event = json.loads(
                    payload.decode()
                )

        capturing_bus = CapturingBus()
        publisher = EventPublisher(
            capturing_bus
        )
        await publisher.publish(
            event_type="runtime.tool.result",
            source="runtime",
            payload=result_payload
        )

        result_event = capturing_bus.event

        result_event["correlation_id"] = request_event_id
        result_event["causation_id"] = request_event_id

        await self.bus.publish(
            "runtime.tool.result",
            result_event
        )

    async def handle_llm_request(
        self,
        subject,
        event
    ):

        payload = event["payload"]
        provider = self.llm
        model = payload["model"]

        if provider is None:
            response_payload = {
                "request_id": payload["request_id"],
                "task_id": payload["task_id"],
                "status": "error",
                "text": None,
                "model": model,
                "error": "llm provider not registered"
            }
        else:
            try:
                result = provider.complete(
                    payload["prompt"],
                    model=model
                )
                if isawaitable(
                    result
                ):
                    result = await result
                response_payload = {
                    "request_id": payload["request_id"],
                    "task_id": payload["task_id"],
                    "status": "success",
                    "text": str(
                        result
                    ),
                    "model": model,
                    "error": None
                }
            except Exception as exc:
                response_payload = {
                    "request_id": payload["request_id"],
                    "task_id": payload["task_id"],
                    "status": "error",
                    "text": None,
                    "model": model,
                    "error": str(
                        exc
                    )
                }

        request_event_id = event.get(
            "id"
        )

        class CapturingBus:

            def __init__(self):
                self.event = None

            async def publish(self, subject, payload):
                self.event = json.loads(
                    payload.decode()
                )

        capturing_bus = CapturingBus()
        publisher = EventPublisher(
            capturing_bus
        )
        await publisher.publish(
            event_type="runtime.llm.response",
            source="runtime",
            payload=response_payload
        )

        response_event = capturing_bus.event

        response_event["correlation_id"] = request_event_id
        response_event["causation_id"] = request_event_id

        await self.bus.publish(
            "runtime.llm.response",
            response_event
        )
     
    async def stop(self):

        await self.bus.close()
