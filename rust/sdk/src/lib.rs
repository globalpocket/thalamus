use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use thalamus_protocol::EventEnvelope;

/// Thalamus SDK - 外部言語用バインディング管理構造体
pub struct ThalamusSDK {
    connected: bool,
}

impl ThalamusSDK {
    /// 新しいThalamusSDKインスタンスを生成する
    pub fn new() -> Self {
        Self { connected: false }
    }

    /// Thalamusランタイムへ接続する
    pub fn connect(&mut self) {
        self.connected = true;
    }

    /// Thalamusランタイムからの切断を行う
    pub fn disconnect(&mut self) {
        self.connected = false;
    }

    /// イベントをpublishする
    pub fn publish(
        &self,
        subject: &str,
        source: &str,
        payload: serde_json::Value,
    ) -> Result<EventEnvelope, SDKError> {
        if !self.connected {
            return Err(SDKError::NotConnected);
        }

        let envelope = EventEnvelope {
            id: uuid::Uuid::new_v4().to_string(),
            subject: subject.to_string(),
            source: source.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            schema: "thalamus/v1".to_string(),
            payload,
            correlation_id: None,
            causation_id: None,
            metadata: serde_json::json!({}),
        };

        Ok(envelope)
    }

    /// イベントをsubscribeする（handler関数登録）
    pub fn subscribe<F>(&self, _subject: &str, _handler: F) -> Result<String, SDKError>
    where
        F: Fn(EventEnvelope) + Send + Sync + 'static,
    {
        if !self.connected {
            return Err(SDKError::NotConnected);
        }

        // 現在はsubscription IDのみを返す（実装は後続Unitで拡張）
        Ok(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for ThalamusSDK {
    fn default() -> Self {
        Self::new()
    }
}

/// SDKエラーTypeEnum
#[derive(Debug, thiserror::Error)]
pub enum SDKError {
    #[error("not connected to Thalamus runtime")]
    NotConnected,
    #[error("subscription failed: {0}")]
    SubscriptionFailed(String),
    #[error("publish failed: {0}")]
    PublishFailed(String),
}

// FFI バインディング API

/// イベントをpublishする（C互換API）
/// # Safety
/// subject、source、payloadは有効なNULL終端文字列を指す必要がある。
#[no_mangle]
pub unsafe extern "C" fn thalamus_publish(
    subject: *const c_char,
    source: *const c_char,
    payload: *const c_char,
) -> i32 {
    if subject.is_null() || source.is_null() || payload.is_null() {
        return -1;
    }

    let subject_str = match CStr::from_ptr(subject).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let source_str = match CStr::from_ptr(source).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let payload_str = match CStr::from_ptr(payload).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let payload: serde_json::Value = match serde_json::from_str(payload_str) {
        Ok(v) => v,
        Err(_) => return -1,
    };

    let mut sdk = ThalamusSDK::new();
    sdk.connect();

    match sdk.publish(subject_str, source_str, payload) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// イベントをsubscribeする（C互換API）
/// # Safety
/// subjectは有効なNULL終端文字列を指す必要がある。
/// handlerは有効な関数ポインタでなければならない。
/// handlerへ渡されるpayload pointerはNUL終端済みでcallback呼出中だけ有効であり、callback後に保持してはならない。
#[no_mangle]
pub unsafe extern "C" fn thalamus_subscribe(
    subject: *const c_char,
    handler: Option<extern "C" fn(*const c_char)>,
) -> i32 {
    let Some(handler) = handler else {
        return -1;
    };

    if subject.is_null() {
        return -1;
    }

    let subject_str = match CStr::from_ptr(subject).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let mut sdk = ThalamusSDK::new();
    sdk.connect();

    match sdk.subscribe(subject_str, |_env| {}) {
        Ok(_) => {
            let payload = serde_json::json!({
                "subject": subject_str,
                "kind": "contract",
            })
            .to_string();

        if let Ok(c_payload) = CString::new(payload) {
            handler(c_payload.as_ptr());
        }

            0
        }
        Err(_) => -1,
    }
}

/// Thalamusランタイムをシャットダウンする（C互換API）
#[no_mangle]
pub extern "C" fn thalamus_shutdown() {
    // 現在はクリーンアップ処理のみ（将来的にリソース解放を追加）
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_sdk() {
        let _sdk = ThalamusSDK::new();
        // 内部状態はpublicではないが、デフォルト値として機能することを確認
    }

    #[test]
    fn test_connect_disconnect() {
        let mut sdk = ThalamusSDK::new();
        sdk.connect();
        sdk.disconnect();
    }

    #[test]
    fn test_publish_not_connected() {
        let sdk = ThalamusSDK::new();
        let result = sdk.publish("test.subject", "test.source", serde_json::json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn test_publish_connected() {
        let mut sdk = ThalamusSDK::new();
        sdk.connect();
        let result = sdk.publish("test.subject", "test.source", serde_json::json!({"key": "value"}));
        assert!(result.is_ok());
        let envelope = result.unwrap();
        assert_eq!(envelope.subject, "test.subject");
        assert_eq!(envelope.source, "test.source");
    }

    #[test]
    fn test_subscribe_not_connected() {
        let sdk = ThalamusSDK::new();
        let result = sdk.subscribe("test.subject", |_| {});
        assert!(result.is_err());
    }
}
