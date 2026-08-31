//! The one module allowed to speak HTTP (spec D9/A14b). `update::check` is
//! the only caller of `audited_get`; `no_network.rs`'s `no_reqwest_outside_net`
//! test greps the rest of `src-tauri/src` to keep it that way.
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
async fn fetch_and_log<F, Fut>(store: &Mutex<Store>, url: &str, send: F) -> Result<(u16, Vec<u8>), String>
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = Result<(u16, Vec<u8>), String>>,
{
    let started_at = chrono::Utc::now();
    let started = Instant::now();
    let result = send(url.to_string()).await;
    let duration_ms = started.elapsed().as_millis() as i64;
    let (status, bytes_in) = match &result {
        Ok((status, body)) => (status.to_string(), body.len() as i64),
        Err(e) => (format!("error: {e}"), 0),
    };
    if let Ok(mut s) = store.lock() {
        let _ = s.append_net_log(started_at, url, &status, duration_ms, bytes_in);
    }
    result
}

async fn send_request(url: String) -> Result<(u16, Vec<u8>), String> {
    let response = reqwest::Client::new()
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

/// `true` only for loopback addresses (127.0.0.0/8, `::1`). A listening
/// socket's "any" address (`0.0.0.0`, `::`) is neither loopback nor a real
/// remote peer; `sample_connections` drops those by TCP state before this
/// ever runs, rather than folding that case into the classifier.
pub fn is_localhost(addr: IpAddr) -> bool {
    addr.is_loopback()
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
}
