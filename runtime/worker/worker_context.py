import uuid


class WorkerContext:

    def __init__(
        self,
        agent_id=None,
        capabilities=None
    ):

        self.agent_id = (
            agent_id
            or f"worker-{uuid.uuid4().hex[:8]}"
        )

        self.capabilities = (
            capabilities or []
        )