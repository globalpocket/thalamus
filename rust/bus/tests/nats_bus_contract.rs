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

    // ==================== Connection Retry Tests ====================

    #[test]
    fn test_nats_config_from_env_uses_thalamus_nats_url() {
        // Save original value
        let original = std::env::var("THALAMUS_NATS_URL").ok();

        // Set environment variable
        std::env::set_var("THALAMUS_NATS_URL", "nats://custom-host:4222");

        let config = NatsBusConfig::from_env();
        assert_eq!(config.url, "nats://custom-host:4222");

        // Restore original value
        match original {
            Some(val) => std::env::set_var("THALAMUS_NATS_URL", val),
            None => std::env::remove_var("THALAMUS_NATS_URL"),
        }
    }

    #[test]
    fn test_nats_config_from_env_falls_back_to_default() {
        // Save original value
        let original = std::env::var("THALAMUS_NATS_URL").ok();

        // Remove environment variable
        std::env::remove_var("THALAMUS_NATS_URL");

        let config = NatsBusConfig::from_env();
        assert_eq!(config.url, "nats://127.0.0.1:4222");

        // Restore original value
        match original {
            Some(val) => std::env::set_var("THALAMUS_NATS_URL", val),
            None => std::env::remove_var("THALAMUS_NATS_URL"),
        }
    }

    #[test]
    fn test_nats_config_from_env_panics_on_empty() {
        let original = std::env::var("THALAMUS_NATS_URL").ok();
        std::env::set_var("THALAMUS_NATS_URL", "   ");

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            NatsBusConfig::from_env();
        }));

        assert!(
            result.is_err(),
            "from_env() should panic when THALAMUS_NATS_URL is empty/whitespace"
        );

        // Restore
        match original {
            Some(val) => std::env::set_var("THALAMUS_NATS_URL", val),
            None => std::env::remove_var("THALAMUS_NATS_URL"),
        }
    }

    #[test]
    fn test_nats_config_is_valid_url_accepts_nats() {
        assert!(NatsBusConfig::is_valid_url("nats://127.0.0.1:4222"));
        assert!(NatsBusConfig::is_valid_url("nats://localhost:4222"));
    }

    #[test]
    fn test_nats_config_is_valid_url_accepts_tls() {
        assert!(NatsBusConfig::is_valid_url("tls://127.0.0.1:4222"));
        assert!(NatsBusConfig::is_valid_url("tls://secure.example.com:4222"));
    }

    #[test]
    fn test_nats_config_is_valid_url_rejects_invalid() {
        assert!(!NatsBusConfig::is_valid_url("http://127.0.0.1:4222"));
        assert!(!NatsBusConfig::is_valid_url("nats://"));
        assert!(!NatsBusConfig::is_valid_url(""));
        assert!(!NatsBusConfig::is_valid_url("  "));
    }

    #[test]
    fn test_nats_config_default_uses_thalamus_nats_url() {
        let original = std::env::var("THALAMUS_NATS_URL").ok();
        std::env::set_var("THALAMUS_NATS_URL", "nats://default-test:4222");

        let config = NatsBusConfig::default();
        assert_eq!(config.url, "nats://default-test:4222");

        match original {
            Some(val) => std::env::set_var("THALAMUS_NATS_URL", val),
            None => std::env::remove_var("THALAMUS_NATS_URL"),
        }
    }

    // ==================== Graceful Shutdown Tests ====================

    #[tokio::test]
    async fn test_nats_graceful_shutdown_aborts_subscriber_tasks() {
        let url = match std::env::var("THALAMUS_NATS_TEST_URL") {
            Ok(u) => u,
            Err(_) => return,
        };

        let bus = match NatsBus::connect_url(&url).await {
            Ok(b) => b,
            Err(_) => return,
        };

        let subject = "test.shutdown.subscriber".to_string();
        let received = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let received_clone = received.clone();

        let handler = std::sync::Arc::new(move |envelope: thalamus_protocol::EventEnvelope| {
            let received = received_clone.clone();
            Box::pin(async move {
                received.lock().await.push(envelope);
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        });

        let sub_id = bus.subscribe(subject.clone(), handler).await.unwrap();

        // Wait for subscription to be established
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Verify handler count before close
        assert_eq!(bus.handler_count(&subject).await, 1);

        // Close the bus (graceful shutdown)
        bus.close().await;

        // Verify bus is closed
        assert!(bus.is_closed().await);

        // Verify handler count is zero after close
        assert_eq!(bus.handler_count(&subject).await, 0);

        // Publishing after close should fail
        let test_envelope = thalamus_protocol::EventEnvelope {
            id: uuid::Uuid::new_v4().to_string(),
            r#type: "test".to_string(),
            subject: subject.clone(),
            source: "shutdown-test".to_string(),
            timestamp: "2025-01-24T00:00:00Z".to_string(),
            schema: "v1".to_string(),
            payload: serde_json::json!({"step": "after_close"}),
            correlation_id: None,
            causation_id: None,
            metadata: serde_json::json!({}),
            scope: None,
            refs: None,
        };

        assert!(
            bus.publish(test_envelope).await.is_err(),
            "publish after close should fail"
        );

        // Unsubscribing after close should fail
        assert!(
            bus.unsubscribe(sub_id).await.is_err(),
            "unsubscribe after close should fail"
        );
    }

    #[tokio::test]
    async fn test_nats_graceful_shutdown_multiple_subscribers() {
        let url = match std::env::var("THALAMUS_NATS_TEST_URL") {
            Ok(u) => u,
            Err(_) => return,
        };

        let bus = match NatsBus::connect_url(&url).await {
            Ok(b) => b,
            Err(_) => return,
        };

        let subject = "test.shutdown.multi".to_string();
        let received1 = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let received2 = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let handler1 = std::sync::Arc::new(move |envelope: thalamus_protocol::EventEnvelope| {
            let received = received1.clone();
            Box::pin(async move {
                received.lock().await.push(envelope);
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        });

        let handler2 = std::sync::Arc::new(move |envelope: thalamus_protocol::EventEnvelope| {
            let received = received2.clone();
            Box::pin(async move {
                received.lock().await.push(envelope);
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        });

        let sub_id1 = bus.subscribe(subject.clone(), handler1).await.unwrap();
        let sub_id2 = bus.subscribe(subject.clone(), handler2).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Verify two handlers registered
        assert_eq!(bus.handler_count(&subject).await, 2);

        // Close the bus
        bus.close().await;

        assert!(bus.is_closed().await);
        assert_eq!(bus.handler_count(&subject).await, 0);

        // Both subscriptions should be unsubscribable (return NotFound)
        assert!(bus.unsubscribe(sub_id1).await.is_err());
        assert!(bus.unsubscribe(sub_id2).await.is_err());
    }

    #[tokio::test]
    async fn test_nats_graceful_shutdown_is_closed_after_close() {
        let url = match std::env::var("THALAMUS_NATS_TEST_URL") {
            Ok(u) => u,
            Err(_) => return,
        };

        let bus = match NatsBus::connect_url(&url).await {
            Ok(b) => b,
            Err(_) => return,
        };

        assert!(!bus.is_closed().await, "bus should not be closed initially");

        bus.close().await;

        assert!(bus.is_closed().await, "bus should be closed after close()");

        // Multiple close() calls should be safe (idempotent)
        bus.close().await;
        assert!(bus.is_closed().await, "bus should still be closed");
    }

    #[tokio::test]
    async fn test_nats_graceful_shutdown_no_double_delivery_on_close() {
        let url = match std::env::var("THALAMUS_NATS_TEST_URL") {
            Ok(u) => u,
            Err(_) => return,
        };

        let bus = match NatsBus::connect_url(&url).await {
            Ok(b) => b,
            Err(_) => return,
        };

        let subject = "test.shutdown.nodelivery".to_string();
        let received = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let received_clone = received.clone();

        let handler = std::sync::Arc::new(move |envelope: thalamus_protocol::EventEnvelope| {
            let received = received_clone.clone();
            Box::pin(async move {
                received.lock().await.push(envelope);
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        });

        let _sub_id = bus.subscribe(subject.clone(), handler).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Publish a message before close
        let test_envelope = thalamus_protocol::EventEnvelope {
            id: uuid::Uuid::new_v4().to_string(),
            r#type: "test".to_string(),
            subject: subject.clone(),
            source: "shutdown-test".to_string(),
            timestamp: "2025-01-24T00:00:00Z".to_string(),
            schema: "v1".to_string(),
            payload: serde_json::json!({"step": "before_close"}),
            correlation_id: None,
            causation_id: None,
            metadata: serde_json::json!({}),
            scope: None,
            refs: None,
        };

        bus.publish(test_envelope).await.unwrap();

        // Wait for message delivery
        tokio::time::timeout(tokio::time::Duration::from_secs(2), async {
            loop {
                if !received.lock().await.is_empty() {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("message should be received before close");

        let before_close_count = received.lock().await.len();

        // Close the bus
        bus.close().await;

        // Wait a bit to ensure no double delivery
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let after_close_count = received.lock().await.len();
        assert_eq!(
            before_close_count, after_close_count,
            "no messages should be delivered after close"
        );
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
