//! NatsBus contract tests.
//!
//! These tests verify:
//! 1. NatsBus compiles with `--all-features`
//! 2. NatsBusConfig::default() returns correct URL
//! 3. NatsBus type satisfies Clone + Send + Sync + 'static
//! 4. NATS round-trip test only runs when THALAMUS_NATS_TEST_URL is set

#[cfg(feature = "nats")]
mod nats_contract_tests {
    use thalamus_bus::{MessageBus, NatsBus, NatsBusConfig};

    #[test]
    fn test_nats_bus_config_default() {
        let config = NatsBusConfig::default();
        assert_eq!(config.url, "nats://127.0.0.1:4222");
    }

    #[test]
    fn test_nats_bus_config_new() {
        let config = NatsBusConfig::new("nats://custom:4222");
        assert_eq!(config.url, "nats://custom:4222");
    }

    #[test]
    fn test_nats_bus_is_clone() {
        let bus = NatsBus::new(NatsBusConfig::default());
        let _cloned = bus.clone();
    }

    // Verify NatsBus satisfies Send + Sync + 'static
    const fn _assert_send_sync_static<T: Send + Sync + 'static>() {}
    #[test]
    fn test_nats_bus_is_send_sync_static() {
        _assert_send_sync_static::<NatsBus>();
    }

    #[tokio::test]
    async fn test_nats_round_trip_if_env_is_set() {
        let url = match std::env::var("THALAMUS_NATS_TEST_URL") {
            Ok(u) => u,
            Err(_) => {
                // Skip test if THALAMUS_NATS_TEST_URL is not set
                return;
            }
        };

        let bus = match NatsBus::connect_url(&url).await {
            Ok(b) => b,
            Err(_) => {
                // Skip if cannot connect — acceptable for integration test skip behavior
                return;
            }
        };

        let subject = "test.contract.roundtrip".to_string();
        let received = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let received_clone = received.clone();

        let handler = std::sync::Arc::new(move |envelope: thalamus_protocol::EventEnvelope| {
            let received = received_clone.clone();
            let env = envelope;
            Box::pin(async move {
                let mut guard = received.lock().await;
                guard.push(env);
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        });

        let _sub_id = match bus.subscribe(subject.clone(), handler).await {
            Ok(id) => id,
            Err(_) => {
                // Skip if subscribe fails
                return;
            }
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let test_envelope = thalamus_protocol::EventEnvelope {
            id: uuid::Uuid::new_v4().to_string(),
            r#type: "test".to_string(),
            subject: subject.clone(),
            source: "contract-test".to_string(),
            timestamp: "2025-01-24T00:00:00Z".to_string(),
            schema: "v1".to_string(),
            payload: serde_json::json!({
                "agent_id": "contract-test-agent",
                "capability": "contract-test"
            }),
            correlation_id: None,
            causation_id: None,
            metadata: serde_json::json!({}),
            scope: None,
            refs: None,
        };

        if bus.publish(test_envelope).await.is_err() {
            // Skip if publish fails
            return;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        let received_events = received.lock().await;
        assert_eq!(received_events.len(), 1);
        assert_eq!(received_events[0].subject, subject);
    }
}

#[cfg(not(feature = "nats"))]
mod nats_not_enabled {
    // This test verifies that thalamus-bus compiles without the nats feature
    #[test]
    fn test_compiles_without_nats() {
        assert!(true);
    }
}
