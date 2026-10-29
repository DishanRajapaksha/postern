//! A malicious website must not reach this server through a rebinding hostname.
use axum::http::{
    HeaderMap,
    header::{HOST, ORIGIN},
};

pub(super) fn allowed(headers: &HeaderMap) -> bool {
    let mut hosts = headers.get_all(HOST).iter();
    let Some(host) = hosts.next().and_then(|h| h.to_str().ok()) else {
        return false;
    };
    if hosts.next().is_some() || !matches!(host, "localhost:8765" | "127.0.0.1:8765") {
        return false;
    }
    let mut origins = headers.get_all(ORIGIN).iter();
    // CLI clients normally omit Origin. Browsers must use this exact local origin.
    match origins.next() {
        None => true,
        Some(origin) => {
            origins.next().is_none() && origin.to_str().is_ok_and(|v| v == format!("http://{host}"))
        }
    }
}
