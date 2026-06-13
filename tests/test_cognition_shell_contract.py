import pytest

from runtime.cognition.shell import CingulaterClient


@pytest.mark.asyncio
async def test_contract_cingulater_client_chat_uses_http_boundary(monkeypatch):
    recorded_calls = []

    class FakeResponse:
        def __init__(self):
            self.raise_for_status_called = False

        def raise_for_status(self):
            self.raise_for_status_called = True

        def json(self):
            return {
                "id": "chatcmpl-contract",
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": "contract response",
                        },
                    }
                ],
            }

    class FakeAsyncClient:
        async def __aenter__(self):
            return self

        async def __aexit__(self, exc_type, exc, traceback):
            return None

        async def post(self, url, headers, json):
            response = FakeResponse()
            recorded_calls.append(
                {
                    "url": url,
                    "headers": headers,
                    "json": json,
                    "response": response,
                }
            )
            return response

    monkeypatch.setattr("runtime.cognition.shell.httpx.AsyncClient", FakeAsyncClient)

    messages = [{"role": "user", "content": "hello"}]
    client = CingulaterClient(
        base_url="https://cingulater.example.test",
        api_key="test-api-key",
    )

    result = await client.chat(model="mock-model", messages=messages)

    assert result == {
        "id": "chatcmpl-contract",
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": "contract response",
                },
            }
        ],
    }
    assert recorded_calls == [
        {
            "url": "https://cingulater.example.test/v1/chat/completions",
            "headers": {"Authorization": "Bearer test-api-key"},
            "json": {
                "model": "mock-model",
                "messages": messages,
            },
            "response": recorded_calls[0]["response"],
        }
    ]
    assert recorded_calls[0]["response"].raise_for_status_called is True
