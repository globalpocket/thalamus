from runtime.events.validator import validate_event


def test_contract_validate_runtime_tool_request_payload():

    event = validate_event(
        event_type="runtime.tool.request",
        source="test",
        payload={
            "request_id": "request-001",
            "task_id": "task-001",
            "capability": "tool.shell",
            "input": {
                "command": "echo hello"
            },
            "agent_id": "agent-001"
        }
    )

    assert (
        event.type
        == "runtime.tool.request"
    )

    assert (
        event.payload["request_id"]
        == "request-001"
    )

    assert (
        event.payload["capability"]
        == "tool.shell"
    )

    assert (
        event.payload["input"]
        == {
            "command": "echo hello"
        }
    )


def test_contract_validate_runtime_llm_request_payload():

    event = validate_event(
        event_type="runtime.llm.request",
        source="test",
        payload={
            "request_id": "llm-request-001",
            "task_id": "task-001",
            "prompt": "Summarize the task.",
            "model": "test-model",
            "agent_id": "agent-001"
        }
    )

    assert (
        event.type
        == "runtime.llm.request"
    )

    assert (
        event.payload["request_id"]
        == "llm-request-001"
    )

    assert (
        event.payload["prompt"]
        == "Summarize the task."
    )

    assert (
        event.payload["model"]
        == "test-model"
    )


def test_contract_validate_runtime_agent_error_payload():

    event = validate_event(
        event_type="runtime.agent.error",
        source="test",
        payload={
            "agent_id": "agent-001",
            "error": "worker failed",
            "task_id": "task-001"
        }
    )

    assert (
        event.type
        == "runtime.agent.error"
    )

    assert (
        event.payload["agent_id"]
        == "agent-001"
    )

    assert (
        event.payload["error"]
        == "worker failed"
    )

    assert (
        event.payload["task_id"]
        == "task-001"
    )
