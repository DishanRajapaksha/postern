//! Load an explicit host inventory. Unknown fields fail instead of being ignored.
#[cfg(test)]
mod tests;
mod validation;

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{fs::File, io::Read, path::Path};

const MAX_CONFIG_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Host {
    pub(crate) id: String,
    pub(crate) description: String,
    /// Exact Keycloak subjects allowed here. An empty list means nobody.
    pub(crate) allowed_subjects: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    pub(crate) hosts: Vec<Host>,
}

/// Read at most limit + 1 bytes, so an oversized file cannot exhaust memory.
pub(crate) fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let file = File::open(path).context("cannot open configuration file")?;
    ensure!(
        file.metadata()?.is_file(),
        "configuration must be a regular file"
    );
    let mut bytes = Vec::new();
    file.take((limit + 1) as u64).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= limit, "configuration exceeds {limit} bytes");
    Ok(bytes)
}

impl Config {
    pub(crate) fn load(path: &Path) -> Result<Self> {
        let bytes = read_bounded(path, MAX_CONFIG_BYTES)?;
        let config: Self =
            serde_json::from_slice(&bytes).context("invalid host configuration JSON")?;
        config.validate()
    }
}
