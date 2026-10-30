//! A malicious website must not reach this server through a rebinding hostname.
use axum::http::{
    HeaderMap,
    header::{HOST, ORIGIN},
};

#[cfg(test)]
pub(super) fn allowed(headers: &HeaderMap) -> bool {
    allowed_public(headers, None)
}

pub(super) fn allowed_public(headers: &HeaderMap, public: Option<&str>) -> bool {
    let mut hosts = headers.get_all(HOST).iter();
    let Some(host) = hosts.next().and_then(|h| h.to_str().ok()) else {
        return false;
    };
    let public_host = public.and_then(|v| v.strip_prefix("https://"));
    if hosts.next().is_some()
        || !(matches!(host, "localhost:8765" | "127.0.0.1:8765") || public_host == Some(host))
    {
        return false;
    }
    let mut origins = headers.get_all(ORIGIN).iter();
    // CLI clients normally omit Origin. Browsers must use this exact local origin.
    match origins.next() {
        None => true,
        Some(origin) => {
            origins.next().is_none()
                && origin.to_str().is_ok_and(|v| {
                    if public_host == Some(host) {
                        Some(v) == public
                    } else {
                        v == format!("http://{host}")
                    }
                })
        }
    }
}
