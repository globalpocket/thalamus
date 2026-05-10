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

    def unregister(self, agent_id):

        if agent_id in self.workers:
            del self.workers[agent_id]

    def find_by_capability(
        self,
        capability
    ):

        results = []

        for agent_id, meta in self.workers.items():

            if capability in meta["capabilities"]:
                results.append(agent_id)

        return results