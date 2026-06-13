from runtime.events.types import RuntimeEvent


def test_contract_runtime_event_accepts_canonical_envelope_fields():

    event = RuntimeEvent(
        id="evt-contract-001",
        type="runtime.task.assign",
        subject="runtime.task.assign",
        source="test.contract",
        timestamp="2025-01-01T00:00:00Z",
        schema="https://thalamus.dev/schemas/runtime/event-envelope.json",
        payload={
            "task_id": "task-contract-001",
            "objective": "Verify canonical event envelope"
        },
        correlation_id="corr-contract-001",
        causation_id="cause-contract-001",
        metadata={
            "tenant": "contract-test"
        }
    )

    assert event.subject == "runtime.task.assign"
    assert event.type == "runtime.task.assign"


def test_contract_runtime_event_exposes_required_canonical_envelope_fields():

    required_fields = {
        "id",
        "type",
        "subject",
        "source",
        "timestamp",
        "schema",
        "payload",
        "correlation_id",
        "causation_id",
        "metadata",
    }

    assert required_fields <= set(
        RuntimeEvent.model_fields
    )
