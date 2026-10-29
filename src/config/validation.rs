//! Validate names and grants once at startup; requests use the frozen inventory.
use super::{Config, Host};
use anyhow::{Result, ensure};
use std::collections::HashSet;

impl Config {
    pub(crate) fn validate(mut self) -> Result<Self> {
        ensure!(self.hosts.len() <= 64, "too many host profiles (max 64)");
        let mut seen = HashSet::new();
        for host in &self.hosts {
            host.validate()?;
            ensure!(seen.insert(&host.id), "duplicate host id");
        }
        // Stable ordering makes both responses and access reviews predictable.
        self.hosts.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(self)
    }
}

impl Host {
    fn validate(&self) -> Result<()> {
        let id = self.id.as_bytes();
        ensure!(
            !id.is_empty()
                && id.len() <= 64
                && id[0].is_ascii_lowercase()
                && id[1..]
                    .iter()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"-_".contains(c)),
            "invalid host id"
        );
        ensure!(
            !self.description.trim().is_empty()
                && self.description == self.description.trim()
                && self.description.chars().count() <= 256
                && !self.description.chars().any(char::is_control),
            "invalid description"
        );
        ensure!(
            self.allowed_subjects.len() <= 64,
            "too many allowed subjects"
        );
        let mut subjects = HashSet::new();
        for subject in &self.allowed_subjects {
            ensure!(
                !subject.trim().is_empty()
                    && subject == subject.trim()
                    && subject.len() <= 256
                    && !subject.chars().any(char::is_control),
                "invalid allowed subject"
            );
            ensure!(subjects.insert(subject), "duplicate allowed subject");
        }
        Ok(())
    }
}
