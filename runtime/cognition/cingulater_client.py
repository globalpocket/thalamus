import httpx


class CingulaterClient:

    def __init__(
        self,
        base_url: str,
        api_key: str
    ):
        self.base_url = base_url
        self.api_key = api_key

    async def chat(
        self,
        model: str,
        messages: list[dict]
    ) -> dict:

        async with httpx.AsyncClient() as client:

            response = await client.post(
                f"{self.base_url}/v1/chat/completions",
                headers={
                    "Authorization":
                    f"Bearer {self.api_key}"
                },
                json={
                    "model": model,
                    "messages": messages
                }
            )

            response.raise_for_status()

            return response.json()