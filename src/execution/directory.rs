//! Private non-symlink runtime and audit directories.
use anyhow::{Result, ensure};
use std::{os::unix::fs::PermissionsExt, path::Path};
pub(super) fn private_dir(path: &Path) -> Result<()> {
    ensure!(path.is_absolute(), "directory must be absolute");
    // Do not silently change permissions on existing operator directories.
    if !path.exists() {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new().mode(0o700).create(path)?;
    }
    let meta = std::fs::symlink_metadata(path)?;
    ensure!(
        meta.is_dir() && meta.permissions().mode() & 0o077 == 0,
        "directory must be private and cannot be a symlink"
    );
    ensure!(
        !path.to_string_lossy().contains(['%', '$']),
        "unsafe directory path"
    );
    Ok(())
}
