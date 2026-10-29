//! Read-only host inventory for the first MCP feature.
//! No SSH connection details or credentials are accepted in this schema yet.

use std::{collections::HashSet, fs, path::Path};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

const MAX_CONFIG_BYTES: usize = 64 * 1024;
const MAX_HOSTS: usize = 64;
const MAX_ID_BYTES: usize = 64;
const MAX_DESCRIPTION_CHARS: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Host {
    pub id: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub hosts: Vec<Host>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes =
            fs::read(path).with_context(|| format!("cannot read config: {}", path.display()))?;
        ensure!(
            bytes.len() <= MAX_CONFIG_BYTES,
            "config exceeds {} bytes",
            MAX_CONFIG_BYTES
        );
        let config: Self =
            serde_json::from_slice(&bytes).context("invalid host configuration JSON")?;
        config.validate()
    }

    pub fn validate(mut self) -> Result<Self> {
        ensure!(
            self.hosts.len() <= MAX_HOSTS,
            "too many host profiles (max {MAX_HOSTS})"
        );
        let mut seen = HashSet::new();
        for host in &self.hosts {
            let id = host.id.as_bytes();
            let valid = !id.is_empty()
                && id.len() <= MAX_ID_BYTES
                && id[0].is_ascii_lowercase()
                && id[1..].iter().all(|c| {
                    c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-' || *c == b'_'
                });
            if !valid {
                bail!(
                    "invalid host id: use lowercase a-z first, then a-z, 0-9, - or _, max {MAX_ID_BYTES} bytes"
                );
            }
            ensure!(
                seen.insert(host.id.as_str()),
                "duplicate host id: {}",
                host.id
            );
            let desc = host.description.as_str();
            ensure!(
                !desc.trim().is_empty()
                    && desc == desc.trim()
                    && desc.chars().count() <= MAX_DESCRIPTION_CHARS
                    && !desc.chars().any(char::is_control),
                "invalid description for host {}",
                host.id
            );
        }
        self.hosts.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(value: &str) -> Result<Config> {
        let c: Config = serde_json::from_str(value)?;
        c.validate()
    }

    #[test]
    fn accepts_and_sorts_explicit_host_ids() {
        let c = config(r#"{"hosts":[{"id":"z_lab","description":"Secondary"},{"id":"a-lab","description":"Primary"}]}"#).unwrap();
        assert_eq!(
            c.hosts.iter().map(|h| h.id.as_str()).collect::<Vec<_>>(),
            vec!["a-lab", "z_lab"]
        );
    }

    #[test]
    fn empty_inventory_is_safe() {
        assert!(config(r#"{"hosts":[]}"#).unwrap().hosts.is_empty());
    }

    #[test]
    fn rejects_duplicate_host_ids() {
        assert!(
            config(
                r#"{"hosts":[{"id":"lab","description":"One"},{"id":"lab","description":"Two"}]}"#
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_host_option_injection_and_unknown_fields() {
        for id in [
            "-oProxyCommand",
            "evil.host",
            "../root",
            "bad host",
            "",
            "UPPER",
            "a/b",
        ] {
            let s = serde_json::json!({"hosts":[{"id":id,"description":"Test"}]}).to_string();
            assert!(config(&s).is_err(), "accepted invalid id: {id}");
        }
        assert!(
            config(r#"{"hosts":[{"id":"test","description":"Test","private_key":"/tmp/id"}]}"#)
                .is_err()
        );
        assert!(config(r#"{"hosts":[],"ssh_password":"oops"}"#).is_err());
    }

    #[test]
    fn rejects_unbounded_or_control_character_descriptions() {
        let long = "x".repeat(257);
        for description in [long.as_str(), "\n", "bad\rtext", " spaces "] {
            let s =
                serde_json::json!({"hosts":[{"id":"test","description":description}]}).to_string();
            assert!(config(&s).is_err());
        }
    }

    #[test]
    fn rejects_more_than_64_hosts() {
        let hosts = (0..65)
            .map(|n| Host {
                id: format!("h{n}"),
                description: "Test".into(),
            })
            .collect();
        assert!(Config { hosts }.validate().is_err());
    }

    #[test]
    fn refuses_missing_file_and_invalid_json() {
        let path = std::path::Path::new("/this-path-must-not-exist/postern.json");
        assert!(Config::load(path).is_err());
        let temp = tempfile::NamedTempFile::new().unwrap();
        fs::write(temp.path(), "{broken").unwrap();
        assert!(Config::load(temp.path()).is_err());
    }

    #[test]
    fn refuses_large_file() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        fs::write(temp.path(), " ".repeat(MAX_CONFIG_BYTES + 1)).unwrap();
        assert!(Config::load(temp.path()).is_err());
    }
}
