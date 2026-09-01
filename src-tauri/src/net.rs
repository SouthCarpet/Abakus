//! The one module allowed to speak HTTP (spec D9/A14b). `update::check` (via
//! `audited_get`) and `update::check_with` (via `fetch_and_log`, its
//! test-injected transport) are the only callers of the request path this
//! module owns; `no_network.rs`'s scanner greps the rest of `src-tauri/src`
//! to keep it that way.
//!
//! `sample_connections` is a separate, lower-level check: it never opens a
//! connection itself, it only reads which sockets this process already has
//! open, so it catches a leak `audited_get` would never see.
use std::future::Future;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use store::Store;

const USER_AGENT: &str = concat!("abakus/", env!("CARGO_PKG_VERSION"));
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// A response larger than this is treated the same as any other transport
/// failure: logged and returned as an error, never buffered without limit.
const MAX_BODY_BYTES: usize = 1024 * 1024;

/// One unauthenticated GET, logged to `net_log` whether it succeeds or
/// fails. Returns the HTTP status and body on success.
///
/// Takes the same `Mutex<Store>` `AppState` holds rather than a `&mut
/// Store`: the store is locked only for the brief, synchronous `net_log`
/// write, never across the `.await` of the request itself, so the guard is
/// never held past a suspend point.
pub async fn audited_get(store: &Mutex<Store>, url: &str) -> Result<(u16, Vec<u8>), String> {
    fetch_and_log(store, url, send_request).await
}

/// The testable half of `audited_get`: `send` stands in for the real
/// network call, so a test can inject a result and assert on the `net_log`
/// row it produced without ever opening a socket.
pub(crate) async fn fetch_and_log<F, Fut>(store: &Mutex<Store>, url: &str, send: F) -> Result<(u16, Vec<u8>), String>
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = Result<(u16, Vec<u8>), String>>,
{
    let started_at = chrono::Utc::now();
    let started = Instant::now();
    let result = send(url.to_string()).await.and_then(|(status, body)| {
        if body.len() > MAX_BODY_BYTES {
            Err(format!("response body of {} bytes exceeds the {MAX_BODY_BYTES} byte cap", body.len()))
        } else {
            Ok((status, body))
        }
    });
    let duration_ms = started.elapsed().as_millis() as i64;
    let (status, bytes_in) = match &result {
        Ok((status, body)) => (status.to_string(), body.len() as i64),
        Err(e) => (format!("error: {e}"), 0),
    };
    log_or_record_failure(store, started_at, url, &status, duration_ms, bytes_in);
    result
}

/// A17/F7: the audit row is written, or the failure to write it is kept. The
/// request result itself is untouched either way, so the caller still learns
/// honestly what the network did; what must never happen is a request that
/// leaves no trace at all and says nothing about it.
fn log_or_record_failure(store: &Mutex<Store>, started_at: chrono::DateTime<chrono::Utc>, url: &str, status: &str, duration_ms: i64, bytes_in: i64) {
    match store.lock() {
        Ok(mut s) => {
            if let Err(e) = s.append_net_log(started_at, url, status, duration_ms, bytes_in) {
                record_audit_failure(url, e.to_string());
            }
        }
        Err(_) => record_audit_failure(url, "store lock poisoned".to_string()),
    }
}

/// An audit-log write that failed. It cannot be recorded in the table that
/// just refused it, so it lands here and the `net_audit_failures` command
/// reads it out for Nastavenia.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AuditFailure {
    pub at: chrono::DateTime<chrono::Utc>,
    pub url: String,
    pub error: String,
}

/// Process-local and bounded. A poisoned sink is recovered rather than
/// panicked on: losing the report of a lost audit row would be the same bug
/// twice.
static AUDIT_FAILURES: Mutex<Vec<AuditFailure>> = Mutex::new(Vec::new());
const MAX_AUDIT_FAILURES: usize = 50;

pub(crate) fn record_audit_failure(url: &str, error: String) {
    let mut list = AUDIT_FAILURES.lock().unwrap_or_else(|e| e.into_inner());
    if list.len() >= MAX_AUDIT_FAILURES {
        list.remove(0);
    }
    list.push(AuditFailure { at: chrono::Utc::now(), url: url.to_string(), error });
}

/// Newest last, as recorded. Empty means every audit row this process wrote
/// reached the database.
pub fn audit_failures() -> Vec<AuditFailure> {
    AUDIT_FAILURES.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

pub(crate) async fn send_request(url: String) -> Result<(u16, Vec<u8>), String> {
    // The one sanctioned call must not silently move: with redirects
    // disabled, a 3xx response comes back as-is and `update::check_with`'s
    // 200-299 range check turns it into a quiet `Status(code)` failure
    // instead of the client following it to an unaudited host.
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    let body = response.bytes().await.map_err(|e| e.to_string())?.to_vec();
    Ok((status, body))
}

/// `true` for loopback addresses (127.0.0.0/8, `::1`) and for an
/// IPv4-mapped IPv6 loopback (`::ffff:127.0.0.1`), which `is_loopback()`
/// alone does not catch. A listening socket's "any" address (`0.0.0.0`,
/// `::`) is neither loopback nor a real remote peer; `sample_connections`
/// drops those by TCP state before this ever runs, rather than folding that
/// case into the classifier.
pub fn is_localhost(addr: IpAddr) -> bool {
    if addr.is_loopback() {
        return true;
    }
    match addr {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().is_some_and(|v4| v4.is_loopback()),
        IpAddr::V4(_) => false,
    }
}

/// Reads this process's own live TCP table and logs every non-localhost
/// endpoint it is connected to (or connecting to) as `observed: <ip>:<port>`.
/// Called once at app start and via the `run_net_audit` command; makes no
/// request of its own, only OS-level socket accounting, so it never has to
/// cross an `.await` and can take the store directly.
pub fn sample_connections(store: &mut Store) -> Result<usize, String> {
    let pid = std::process::id();
    let sockets = netstat2::get_sockets_info(netstat2::AddressFamilyFlags::all(), netstat2::ProtocolFlags::TCP)
        .map_err(|e| e.to_string())?;
    let mut logged = 0;
    for socket in sockets {
        if !socket.associated_pids.contains(&pid) {
            continue;
        }
        let netstat2::ProtocolSocketInfo::Tcp(tcp) = socket.protocol_socket_info else { continue };
        if matches!(tcp.state, netstat2::TcpState::Listen | netstat2::TcpState::Closed) {
            continue;
        }
        if is_localhost(tcp.remote_addr) {
            continue;
        }
        let url = format!("observed: {}:{}", tcp.remote_addr, tcp.remote_port);
        store.append_net_log(chrono::Utc::now(), &url, "observed", 0, 0).map_err(|e| e.to_string())?;
        logged += 1;
    }
    Ok(logged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fetch_and_log_records_a_success_row() {
        let store = Mutex::new(Store::open_in_memory().unwrap());
        let result = fetch_and_log(&store, "https://api.github.com/x", |_| async { Ok((200, b"{}".to_vec())) }).await;
        assert_eq!(result, Ok((200, b"{}".to_vec())));

        let rows = store.lock().unwrap().net_log(10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].url, "https://api.github.com/x");
        assert_eq!(rows[0].status, "200");
        assert_eq!(rows[0].bytes_in, 2);
    }

    #[tokio::test]
    async fn fetch_and_log_records_a_failure_row_and_still_returns_err() {
        let store = Mutex::new(Store::open_in_memory().unwrap());
        let result = fetch_and_log(&store, "https://api.github.com/x", |_| async { Err("offline".to_string()) }).await;
        assert_eq!(result, Err("offline".to_string()));

        let rows = store.lock().unwrap().net_log(10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, "error: offline");
        assert_eq!(rows[0].bytes_in, 0);
    }

    #[test]
    fn is_localhost_classifies_loopback_only() {
        assert!(is_localhost("127.0.0.1".parse().unwrap()));
        assert!(is_localhost("::1".parse().unwrap()));
        assert!(!is_localhost("8.8.8.8".parse().unwrap()));
        assert!(!is_localhost("0.0.0.0".parse().unwrap()), "the any-address is not a real peer, but it is also not loopback");
        assert!(!is_localhost("::".parse().unwrap()));
    }

    #[test]
    fn is_localhost_classifies_ipv4_mapped_loopback_too() {
        assert!(is_localhost("::ffff:127.0.0.1".parse().unwrap()));
        assert!(!is_localhost("::ffff:8.8.8.8".parse().unwrap()));
    }

    /// A17/F7: the audit-log write used to be `let _ = ...`, so a store that
    /// could not take the row left no trace anywhere. A poisoned store lock is
    /// the deterministic version of that failure. This test fails again if
    /// anyone drops the failure on the floor: the request result must still be
    /// honest AND the lost audit row must be reported.
    #[tokio::test]
    async fn a_failed_audit_write_is_recorded_and_the_request_result_is_still_returned() {
        let url = "https://api.github.com/f7-poisoned";
        let store = Mutex::new(Store::open_in_memory().unwrap());
        let _ = std::thread::scope(|s| s.spawn(|| { let _guard = store.lock().unwrap(); panic!("poison the store lock on purpose"); }).join());
        assert!(store.lock().is_err(), "the store lock must be poisoned for this test to mean anything");

        let result = fetch_and_log(&store, url, |_| async { Ok((200, b"{}".to_vec())) }).await;
        assert_eq!(result, Ok((200, b"{}".to_vec())), "the request result stays honest");

        let failure = audit_failures().into_iter().find(|f| f.url == url).expect("the failed audit write must be recorded");
        assert_eq!(failure.error, "store lock poisoned");
    }

    /// The other half: a write that succeeds records nothing, so the failure
    /// list stays meaningful instead of filling up with noise.
    #[tokio::test]
    async fn a_successful_audit_write_records_no_failure() {
        let url = "https://api.github.com/f7-healthy";
        let store = Mutex::new(Store::open_in_memory().unwrap());
        let _ = fetch_and_log(&store, url, |_| async { Ok((200, b"{}".to_vec())) }).await;

        assert_eq!(store.lock().unwrap().net_log(10).unwrap().len(), 1);
        assert!(!audit_failures().iter().any(|f| f.url == url));
    }

    #[tokio::test]
    async fn fetch_and_log_rejects_a_body_over_the_cap_and_logs_it() {
        let store = Mutex::new(Store::open_in_memory().unwrap());
        let oversized = vec![0u8; MAX_BODY_BYTES + 1];
        let result = fetch_and_log(&store, "https://api.github.com/x", move |_| async move { Ok((200, oversized)) }).await;
        assert!(result.is_err(), "a body over the cap must not be returned to the caller");

        let rows = store.lock().unwrap().net_log(10).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].status.starts_with("error:"), "the rejection must still be logged");
        assert_eq!(rows[0].bytes_in, 0);
    }
}
