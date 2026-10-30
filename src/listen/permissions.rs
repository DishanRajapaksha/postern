//! An explicit local broker group can connect without access to audit or SSH keys.
use anyhow::{Result, ensure};
use std::{os::unix::fs::PermissionsExt, path::Path};

pub(super) fn apply(path: &Path, group: Option<&str>) -> Result<()> {
    let mode = if let Some(group) = group {
        let gid: u32 = group.parse()?;
        ensure!(gid > 0, "proxy socket group must be a non-root numeric gid");
        std::os::unix::fs::chown(path, None, Some(gid))?;
        0o660
    } else {
        0o600
    };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::MetadataExt;
    #[test]
    fn default_is_private_and_explicit_group_is_bounded() -> Result<()> {
        let file = tempfile::NamedTempFile::new()?;
        apply(file.path(), None)?;
        ensure!(file.as_file().metadata()?.mode() & 0o777 == 0o600);
        let gid = file.as_file().metadata()?.gid();
        if gid != 0 {
            apply(file.path(), Some(&gid.to_string()))?;
            let meta = file.as_file().metadata()?;
            ensure!(meta.mode() & 0o777 == 0o660 && meta.gid() == gid);
        }
        for value in ["0", "root", "", "-1", "4294967296"] {
            ensure!(apply(file.path(), Some(value)).is_err());
        }
        Ok(())
    }
}
