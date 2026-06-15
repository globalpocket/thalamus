use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use thalamus_bus::{BasicBus, BusError, Handler, MessageBus, Subscription, SubscriptionId};
use thalamus_protocol::EventEnvelope;
use uuid::Uuid;

fn envelope_for(subject: &str) -> EventEnvelope {
    EventEnvelope {
        id: Uuid::new_v4().to_string(),
        subject: subject.to_string(),
        source: "behavior-test".to_string(),
        timestamp: Uuid::new_v4().to_string(),
        schema: "1.0".to_string(),
        payload: serde_json::json!({}),
        correlation_id: None,
        causation_id: None,
        metadata: serde_json::json!({}),
    }
}

#[tokio::test]
async fn behavior_public_formatters_expose_stable_display_and_debug_contract() {
    let handler: Handler = Arc::new(|_| Box::pin(async {}));
    let subscription = Subscription {
        id: "sub-1".to_string(),
        subject: "test.subject".to_string(),
        handler,
    };

    assert_eq!(SubscriptionId("sub-1".to_string()).to_string(), "SubscriptionId(sub-1)");
    assert_eq!(
        format!("{subscription:?}"),
        "Subscription { id: \"sub-1\", subject: \"test.subject\", .. }"
    );
}

#[tokio::test]
async fn behavior_default_bus_publish_delivers_envelope_to_matching_handler() {
    let mut bus = BasicBus::default();
    let delivered = Arc::new(AtomicUsize::new(0));
    let delivered_in_handler = Arc::clone(&delivered);
    let handler: Handler = Arc::new(move |envelope| {
        let delivered_in_handler = Arc::clone(&delivered_in_handler);
        Box::pin(async move {
            assert_eq!(envelope.subject, "test.subject");
            delivered_in_handler.fetch_add(1, Ordering::SeqCst);
        })
    });

    bus.subscribe("test.subject".to_string(), handler).await.unwrap();
    bus.publish(envelope_for("test.subject")).await.unwrap();

    assert_eq!(delivered.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn behavior_publish_maps_panicking_handler_to_protocol_error() {
    let mut bus = BasicBus::default();
    let handler: Handler = Arc::new(|_| {
        Box::pin(async {
            panic!("handler panic maps to protocol error");
        })
    });

    bus.subscribe("test.subject".to_string(), handler).await.unwrap();
    let result = bus.publish(envelope_for("test.subject")).await;

    assert!(matches!(result, Err(BusError::ProtocolError)));
}
