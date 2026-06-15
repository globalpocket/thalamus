use serde_json::json;
use thalamus_protocol::{deserialize, serialize, EventEnvelope};

#[test]
fn contract_event_envelope_new_preserves_public_fields() {
    let envelope = EventEnvelope::new(
        "event-1".to_string(),
        "subject.created".to_string(),
        "protocol-contract".to_string(),
        "2026-01-01T00:00:00Z".to_string(),
        "subject.v1".to_string(),
        json!({ "name": "example" }),
        Some("correlation-1".to_string()),
        Some("causation-1".to_string()),
        json!({ "tenant": "test" }),
    );

    assert_eq!(envelope.id, "event-1");
    assert_eq!(envelope.subject, "subject.created");
    assert_eq!(envelope.source, "protocol-contract");
    assert_eq!(envelope.timestamp, "2026-01-01T00:00:00Z");
    assert_eq!(envelope.schema, "subject.v1");
    assert_eq!(envelope.payload, json!({ "name": "example" }));
    assert_eq!(envelope.correlation_id, Some("correlation-1".to_string()));
    assert_eq!(envelope.causation_id, Some("causation-1".to_string()));
    assert_eq!(envelope.metadata, json!({ "tenant": "test" }));
}

#[test]
fn contract_serialize_deserialize_round_trips_json_envelope() {
    let envelope = EventEnvelope::new(
        "event-2".to_string(),
        "subject.updated".to_string(),
        "protocol-contract".to_string(),
        "2026-01-01T00:00:01Z".to_string(),
        "subject.v1".to_string(),
        json!({ "count": 2, "active": true }),
        Some("correlation-2".to_string()),
        None,
        json!({ "trace": "trace-2" }),
    );

    let serialized = serialize(&envelope).expect("envelope serializes to JSON");
    let deserialized = deserialize(&serialized).expect("serialized JSON deserializes to envelope");

    assert_eq!(deserialized, envelope);
}
