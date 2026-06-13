from runtime.events.types import RuntimeAgentErrorPayload, RuntimeLLMRequestPayload, RuntimeTaskAssignPayload, RuntimeToolRequestPayload


def test_contract_task_assign_payload_preserves_required_assignment_fields():

    payload = RuntimeTaskAssignPayload(
        task_id="task-payload-contract-001",
        agent_id="agent-payload-contract-001",
        input={
            "prompt": "summarize runtime contract"
        },
        capabilities=[
            "llm.complete",
            "tool.search"
        ],
        metadata={
            "tenant": "contract-test"
        }
    )

    dumped = payload.model_dump()

    assert dumped["task_id"] == "task-payload-contract-001"
    assert dumped["agent_id"] == "agent-payload-contract-001"
    assert dumped["input"] == {
        "prompt": "summarize runtime contract"
    }
    assert dumped["capabilities"] == [
        "llm.complete",
        "tool.search"
    ]
    assert dumped["metadata"] == {
        "tenant": "contract-test"
    }


def test_contract_tool_and_llm_request_payloads_accept_minimal_optional_fields():

    tool_payload = RuntimeToolRequestPayload(
        request_id="tool-request-contract-001",
        capability="tool.search",
        input={
            "query": "runtime payload optional contract"
        }
    )
    llm_payload = RuntimeLLMRequestPayload(
        request_id="llm-request-contract-001",
        prompt="Explain runtime payload optional contract"
    )

    assert tool_payload.model_dump()["task_id"] is None
    assert tool_payload.model_dump()["agent_id"] is None
    assert tool_payload.model_dump()["input"] == {
        "query": "runtime payload optional contract"
    }
    assert llm_payload.model_dump()["task_id"] is None
    assert llm_payload.model_dump()["model"] is None
    assert llm_payload.model_dump()["agent_id"] is None
    assert llm_payload.model_dump()["prompt"] == "Explain runtime payload optional contract"


def test_contract_agent_error_payload_accepts_minimal_optional_fields():

    payload = RuntimeAgentErrorPayload(
        error="agent failed before task assignment"
    )

    dumped = payload.model_dump()

    assert dumped["error"] == "agent failed before task assignment"
    assert dumped["agent_id"] is None
    assert dumped["task_id"] is None
