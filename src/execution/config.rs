//! Operator-only SSH destinations; no MCP argument can select credentials or flags.
pub(super) use super::directory::private_dir;
use crate::config::read_bounded;
use anyhow::{Result, ensure};
use serde::Deserialize;
use std::{
    collections::HashSet,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Profile {
    pub host_id: String,
    pub hostname: String,
    pub username: String,
    pub port: u16,
    pub identity_file: PathBuf,
    pub known_hosts_file: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExecutionConfig {
    pub profiles: Vec<Profile>,
    pub audit_dir: PathBuf,
    pub runtime_dir: PathBuf,
}

impl ExecutionConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let config: Self = serde_json::from_slice(&read_bounded(path, 65536)?)?;
        ensure!(config.profiles.len() <= 64, "too many SSH profiles");
        let mut seen = HashSet::new();
        for profile in &config.profiles {
            ensure!(seen.insert(&profile.host_id), "duplicate SSH profile");
            profile.validate()?;
        }
        private_dir(&config.audit_dir)?;
        private_dir(&config.runtime_dir)?;
        Ok(config)
    }
}

impl Profile {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.host_id.is_empty() && self.host_id.len() <= 64,
            "invalid host id"
        );
        for value in [&self.hostname, &self.username] {
            ensure!(
                !value.is_empty()
                    && value.len() <= 253
                    && !value.starts_with('-')
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b)),
                "invalid SSH destination"
            );
        }
        ensure!(self.port > 0, "invalid SSH port");
        for path in [&self.identity_file, &self.known_hosts_file] {
            ensure!(
                path.is_absolute() && path.is_file(),
                "SSH files must exist at absolute paths"
            );
            let text = path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("invalid SSH file path"))?;
            ensure!(
                !text.contains(['%', '$', '\n', '\r', ' ', '\t', '"', '\'']),
                "unsafe SSH file path"
            );
        }
        ensure!(
            std::fs::metadata(&self.identity_file)?.permissions().mode() & 0o077 == 0,
            "SSH identity must be private"
        );
        Ok(())
    }
}
