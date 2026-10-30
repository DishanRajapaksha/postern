//! Never leave a fixture mount behind on an assertion failure.
use anyhow::{Result, ensure};
use std::{
    path::{Path, PathBuf},
    process::Command,
};
pub struct Mount(pub PathBuf);
impl Drop for Mount {
    fn drop(&mut self) {
        let _ = Command::new("umount").arg(&self.0).output();
    }
}
pub fn mount(args: &[&str], path: &Path) -> Result<()> {
    ensure!(
        Command::new("mount")
            .args(args)
            .arg(path)
            .status()?
            .success(),
        "fixture mount failed"
    );
    Ok(())
}
