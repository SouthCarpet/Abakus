//! Grok review t15b: `check_update_now` must gate on the persisted
//! `check_updates` setting itself, not trust the UI to have checked it
//! first. Proved here at the `check_update_now_inner` seam with a recording
//! fake transport, so the assertion is "the network was never touched", not
//! just "the result looked right".
use abakus_lib::commands::check_update_now_inner;
use std::cell::RefCell;
use std::sync::Mutex;
use store::Store;

fn releases_body(tag: &str) -> Vec<u8> {
    format!(r#"{{"tag_name": "{tag}", "html_url": "https://example.invalid/r", "body": "notes"}}"#).into_bytes()
}

#[tokio::test]
async fn transport_is_never_called_when_check_updates_is_off() {
    let store = Mutex::new(Store::open_in_memory().unwrap());
    // default is off; do not call set_check_updates(true)

    let calls: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let send = |url: String| {
        calls.borrow_mut().push(url);
        async move { Ok((200, releases_body("v9.9.9"))) }
    };

    let result = check_update_now_inner(&store, "1.0.0", send).await;

    assert_eq!(result, Ok(None));
    assert!(calls.borrow().is_empty(), "the transport must not be called while the setting is off");
    assert!(store.lock().unwrap().net_log(10).unwrap().is_empty(), "an untaken request must not appear in net_log either");
}

#[tokio::test]
async fn transport_is_called_when_check_updates_is_on() {
    let store = Mutex::new(Store::open_in_memory().unwrap());
    store.lock().unwrap().set_check_updates(true).unwrap();

    let calls: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let send = |url: String| {
        calls.borrow_mut().push(url.clone());
        async move { Ok((200, releases_body("v9.9.9"))) }
    };

    let result = check_update_now_inner(&store, "1.0.0", send).await;

    assert_eq!(result.unwrap().map(|r| r.tag), Some("v9.9.9".to_string()));
    assert_eq!(calls.borrow().len(), 1, "the transport must run exactly once while the setting is on");
}

/// Plan 091 Kaliber r3 (Michal 2026-10-12): a connection error must not look like "you have
/// the latest version". The command reports it as an error; only a completed check with
/// nothing newer is `Ok(None)`.
#[tokio::test]
async fn a_failed_request_is_an_error_not_the_latest_version() {
    let store = Mutex::new(Store::open_in_memory().unwrap());
    store.lock().unwrap().set_check_updates(true).unwrap();

    let send = |_: String| async move { Err::<(u16, Vec<u8>), String>("offline".to_string()) };
    let result = check_update_now_inner(&store, "1.0.0", send).await;

    let error = result.expect_err("a request that never completed must be an error");
    assert!(error.starts_with("update check request failed"), "unexpected error text: {error}");
}

#[tokio::test]
async fn an_http_error_is_an_error_and_nothing_newer_is_none() {
    let store = Mutex::new(Store::open_in_memory().unwrap());
    store.lock().unwrap().set_check_updates(true).unwrap();

    let refused = |_: String| async move { Ok((403, Vec::new())) };
    assert_eq!(check_update_now_inner(&store, "1.0.0", refused).await, Err("update check got HTTP 403".to_string()));

    let same = |_: String| async move { Ok((200, releases_body("v1.0.0"))) };
    assert_eq!(check_update_now_inner(&store, "1.0.0", same).await, Ok(None));
}
