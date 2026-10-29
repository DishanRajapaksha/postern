//! Build a public response from private configuration; grant lists never serialize.
use crate::config::Host;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct HostsResponse<'a> {
    pub(super) hosts: Vec<PublicHost<'a>>,
}

#[derive(Serialize)]
pub(super) struct PublicHost<'a> {
    pub(super) id: &'a str,
    description: &'a str,
}

pub(super) fn allowed_hosts<'a>(hosts: &'a [Host], subject: &str) -> Vec<PublicHost<'a>> {
    hosts
        .iter()
        .filter(|host| host.allowed_subjects.iter().any(|s| s == subject))
        .map(|h| PublicHost {
            id: &h.id,
            description: &h.description,
        })
        .collect()
}
