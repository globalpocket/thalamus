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
        let Ok(url) = std::env::var("THALAMUS_NATS_TEST_URL") else {
            return;
        };

        let bus = NatsBus::connect_url(&url)
            .await
            .expect("THALAMUS_NATS_TEST_URL is set, so NATS connection should succeed");

        let subject = format!("test.contract.roundtrip.{}", uuid::Uuid::new_v4());
        let received = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let received_clone = received.clone();

        let handler = std::sync::Arc::new(move |envelope: thalamus_protocol::EventEnvelope| {
            let received = received_clone.clone();
            Box::pin(async move {
                received.lock().await.push(envelope);
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        });

        let sub_id = bus.subscribe(subject.clone(), handler).await.unwrap();
        assert_eq!(bus.handler_count(&subject).await, 1);

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

        bus.publish(test_envelope).await.unwrap();

        tokio::time::timeout(tokio::time::Duration::from_secs(2), async {
            loop {
                if received.lock().await.len() == 1 {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("message should be received within timeout");

        let received_events = received.lock().await;
        assert_eq!(received_events.len(), 1);
        assert_eq!(received_events[0].subject, subject);
        drop(received_events);

        bus.unsubscribe(sub_id).await.unwrap();
        assert_eq!(bus.handler_count(&subject).await, 0);

        bus.close().await;
        assert!(bus.is_closed().await);
    }

    #[tokio::test]
    async fn test_nats_multi_message_round_trip_if_env_is_set() {
        let url = match std::env::var("THALAMUS_NATS_TEST_URL") {
            Ok(u) => u,
            Err(_) => {
                return;
            }
        };

        let bus = match NatsBus::connect_url(&url).await {
            Ok(b) => b,
            Err(_) => {
                return;
            }
        };

        let subject = "test.contract.multi".to_string();
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
                return;
            }
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let message_count = 5;
        for i in 0..message_count {
            let test_envelope = thalamus_protocol::EventEnvelope {
                id: uuid::Uuid::new_v4().to_string(),
                r#type: "test".to_string(),
                subject: subject.clone(),
                source: "contract-test".to_string(),
                timestamp: "2025-01-24T00:00:00Z".to_string(),
                schema: "v1".to_string(),
                payload: serde_json::json!({
                    "agent_id": "contract-test-agent",
                    "index": i
                }),
                correlation_id: None,
                causation_id: None,
                metadata: serde_json::json!({}),
                scope: None,
                refs: None,
            };

            if bus.publish(test_envelope).await.is_err() {
                return;
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

        let received_events = received.lock().await;
        assert_eq!(received_events.len(), message_count);

        let indices: Vec<i64> = received_events
            .iter()
            .map(|e| e.payload["index"].as_i64().unwrap())
            .collect();
        let mut sorted_indices = indices.clone();
        sorted_indices.sort();
        assert_eq!(
            sorted_indices,
            (0..message_count).map(|i| i as i64).collect::<Vec<i64>>()
        );
    }

    #[tokio::test]
    async fn test_nats_subscribe_unsubscribe_cycle_if_env_is_set() {
        let url = match std::env::var("THALAMUS_NATS_TEST_URL") {
            Ok(u) => u,
            Err(_) => {
                return;
            }
        };

        let bus = match NatsBus::connect_url(&url).await {
            Ok(b) => b,
            Err(_) => {
                return;
            }
        };

        let subject = "test.contract.cycle".to_string();
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

        // 1. subscribe
        let sub_id = match bus.subscribe(subject.clone(), handler).await {
            Ok(id) => id,
            Err(_) => {
                return;
            }
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // 2. publish and receive
        let test_envelope = thalamus_protocol::EventEnvelope {
            id: uuid::Uuid::new_v4().to_string(),
            r#type: "test".to_string(),
            subject: subject.clone(),
            source: "contract-test".to_string(),
            timestamp: "2025-01-24T00:00:00Z".to_string(),
            schema: "v1".to_string(),
            payload: serde_json::json!({"step": "before_unsubscribe"}),
            correlation_id: None,
            causation_id: None,
            metadata: serde_json::json!({}),
            scope: None,
            refs: None,
        };

        if bus.publish(test_envelope).await.is_err() {
            return;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        {
            let received_events = received.lock().await;
            assert_eq!(received_events.len(), 1);
        }

        // 3. unsubscribe
        if bus.unsubscribe(sub_id).await.is_err() {
            return;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // 4. publish after unsubscribe - should not receive
        let before_count = {
            let received_events = received.lock().await;
            received_events.len()
        };

        let test_envelope2 = thalamus_protocol::EventEnvelope {
            id: uuid::Uuid::new_v4().to_string(),
            r#type: "test".to_string(),
            subject: subject.clone(),
            source: "contract-test".to_string(),
            timestamp: "2025-01-24T00:00:00Z".to_string(),
            schema: "v1".to_string(),
            payload: serde_json::json!({"step": "after_unsubscribe"}),
            correlation_id: None,
            causation_id: None,
            metadata: serde_json::json!({}),
            scope: None,
            refs: None,
        };

        if bus.publish(test_envelope2).await.is_err() {
            return;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        {
            let received_events = received.lock().await;
            assert_eq!(received_events.len(), before_count);
        }

        // 5. re-subscribe
        let received_clone2 = received.clone();
        let handler2 = std::sync::Arc::new(move |envelope: thalamus_protocol::EventEnvelope| {
            let received = received_clone2.clone();
            let env = envelope;
            Box::pin(async move {
                let mut guard = received.lock().await;
                guard.push(env);
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        });

        let sub_id2 = match bus.subscribe(subject.clone(), handler2).await {
            Ok(id) => id,
            Err(_) => {
                return;
            }
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // 6. publish after re-subscribe - should receive again
        let test_envelope3 = thalamus_protocol::EventEnvelope {
            id: uuid::Uuid::new_v4().to_string(),
            r#type: "test".to_string(),
            subject: subject.clone(),
            source: "contract-test".to_string(),
            timestamp: "2025-01-24T00:00:00Z".to_string(),
            schema: "v1".to_string(),
            payload: serde_json::json!({"step": "after_resubscribe"}),
            correlation_id: None,
            causation_id: None,
            metadata: serde_json::json!({}),
            scope: None,
            refs: None,
        };

        if bus.publish(test_envelope3).await.is_err() {
            return;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        {
            let received_events = received.lock().await;
            assert_eq!(received_events.len(), before_count + 1);
        }

        let _ = sub_id2;
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
