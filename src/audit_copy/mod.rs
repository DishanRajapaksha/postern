//! Copy SQLite through its online backup API, including committed WAL contents.
use anyhow::{Result, ensure};
use rusqlite::{
    Connection, OpenFlags,
    backup::{Backup, StepResult},
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    time::{Duration, Instant},
};

fn private_directory(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_dir() && metadata.permissions().mode() & 0o077 == 0,
        "audit and backup parent directories must be private (0700) and not symlinks"
    );
    Ok(())
}

pub fn copy(source: &Path, destination: &Path) -> Result<()> {
    ensure!(
        source.is_absolute() && destination.is_absolute(),
        "use absolute paths"
    );
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow::anyhow!("destination parent"))?;
    private_directory(parent)?;
    private_directory(
        source
            .parent()
            .ok_or_else(|| anyhow::anyhow!("source parent"))?,
    )?;
    ensure!(
        fs::symlink_metadata(source)?.is_file(),
        "source must be a regular file"
    );
    ensure!(!destination.try_exists()?, "destination already exists");
    let input = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    input.busy_timeout(Duration::from_secs(5))?;
    let temporary = tempfile::NamedTempFile::new_in(parent)?;
    let mut output = Connection::open(temporary.path())?;
    {
        let backup = Backup::new(&input, &mut output)?;
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if backup.step(128)? == StepResult::Done {
                break;
            }
            ensure!(
                Instant::now() < deadline,
                "audit backup exceeded 30 seconds"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let integrity: String = output.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    ensure!(integrity == "ok", "snapshot integrity check failed");
    // A snapshot must contain the audit schema, and travel without WAL sidecars.
    output.query_row("SELECT COUNT(*) FROM events", [], |row| {
        row.get::<_, i64>(0)
    })?;
    output.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL;")?;
    output.close().map_err(|(_, error)| error)?;
    temporary.as_file().sync_all()?;
    let snapshot = temporary.persist_noclobber(destination)?;
    snapshot.sync_all()?;
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}
