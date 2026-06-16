use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use serde_json::json;
use thalamus_sdk::{thalamus_publish, thalamus_subscribe, ThalamusSDK};

static CALLBACK_CALLS: AtomicUsize = AtomicUsize::new(0);
static CALLBACK_PAYLOAD_MATCHED: AtomicBool = AtomicBool::new(false);

#[test]
fn contract_connected_subscribe_succeeds() {
    let mut sdk = ThalamusSDK::new();
    sdk.connect();

    let subscription_id = sdk
        .subscribe("contract.subject", |_event| {})
        .expect("connected SDK should accept a subscription handler");

    assert!(!subscription_id.is_empty());
}

#[test]
fn contract_ffi_publish_rejects_null_and_accepts_valid_json() {
    let subject = CString::new("contract.subject").expect("subject has no interior NUL");
    let source = CString::new("contract.source").expect("source has no interior NUL");
    let payload =
        CString::new(json!({ "ok": true }).to_string()).expect("payload JSON has no interior NUL");

    unsafe {
        assert_eq!(
            thalamus_publish(std::ptr::null(), source.as_ptr(), payload.as_ptr()),
            -1
        );
        assert_eq!(
            thalamus_publish(subject.as_ptr(), std::ptr::null(), payload.as_ptr()),
            -1
        );
        assert_eq!(
            thalamus_publish(subject.as_ptr(), source.as_ptr(), std::ptr::null()),
            -1
        );
        assert_eq!(
            thalamus_publish(subject.as_ptr(), source.as_ptr(), payload.as_ptr()),
            0
        );
    }
}

#[test]
fn regression_ffi_subscribe_rejects_null_callback() {
    let subject = CString::new("contract.subject").expect("subject has no interior NUL");

    unsafe {
        assert_eq!(thalamus_subscribe(subject.as_ptr(), None), -1);
    }
}

#[test]
fn contract_ffi_subscribe_invokes_callback_with_nul_terminated_payload() {
    extern "C" fn callback(payload: *const c_char) {
        assert!(
            !payload.is_null(),
            "callback payload pointer must be non-null"
        );

        let payload = unsafe { std::ffi::CStr::from_ptr(payload) }
            .to_str()
            .expect("callback payload must be valid UTF-8 JSON");

        CALLBACK_CALLS.fetch_add(1, Ordering::SeqCst);
        CALLBACK_PAYLOAD_MATCHED.store(
            payload.contains("contract.subject") && payload.contains("contract"),
            Ordering::SeqCst,
        );
    }

    let subject = CString::new("contract.subject").expect("subject has no interior NUL");
    CALLBACK_CALLS.store(0, Ordering::SeqCst);
    CALLBACK_PAYLOAD_MATCHED.store(false, Ordering::SeqCst);

    unsafe {
        assert_eq!(thalamus_subscribe(std::ptr::null(), Some(callback)), -1);
        assert_eq!(thalamus_subscribe(subject.as_ptr(), Some(callback)), 0);
    }

    assert_eq!(CALLBACK_CALLS.load(Ordering::SeqCst), 1);
    assert!(CALLBACK_PAYLOAD_MATCHED.load(Ordering::SeqCst));
}
