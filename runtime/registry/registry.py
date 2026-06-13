class WorkerRegistry:

    def __init__(self):

        self.workers = {}

    def register(
        self,
        agent_id,
        capabilities
    ):

        self.workers[agent_id] = {
            "capabilities": capabilities
        }

    def mark_ready(
        self,
        agent_id,
        capabilities
    ):

        self.workers[agent_id] = {
            "status": "ready",
            "capabilities": capabilities
        }

    def unregister(self, agent_id):

        if agent_id in self.workers:
            del self.workers[agent_id]

    def mark_exited(
        self,
        agent_id,
        reason=None
    ):

        worker = self.workers.get(agent_id)

        if worker is None:
            return

        self.workers[agent_id] = {
            "status": "exited",
            "capabilities": worker.get("capabilities", []),
            "reason": reason
        }

    def mark_error(
        self,
        agent_id,
        task_id,
        error
    ):

        worker = self.workers.get(agent_id, {})

        self.workers[agent_id] = {
            "status": "error",
            "capabilities": worker.get("capabilities", []),
            "task_id": task_id,
            "error": error
        }

    def find_by_capability(
        self,
        capability
    ):

        results = []

        for agent_id, meta in self.workers.items():

            if meta.get("status") in {"exited", "error"}:
                continue

            if capability in meta["capabilities"]:
                results.append(agent_id)

        return results
