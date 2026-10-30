//! A live WAL writer must produce a standalone, immutable, private snapshot.
use anyhow::{Result, ensure};
use rusqlite::Connection;
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

fn backup(source: &Path, destination: &Path) -> Result<bool> {
    Ok(Command::new(env!("CARGO_BIN_EXE_postern-audit"))
        .arg("backup")
        .arg(source)
        .arg(destination)
        .output()?
        .status
        .success())
}

#[test]
fn snapshots_include_wal_and_refuse_overwrites_or_public_directories() -> Result<()> {
    let root = tempfile::tempdir()?;
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700))?;
    let source = root.path().join("audit.sqlite3");
    let destination = root.path().join("snapshot.sqlite3");
    let writer = Connection::open(&source)?;
    writer.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;
        CREATE TABLE events(seq INTEGER PRIMARY KEY, record TEXT);
        INSERT INTO events(record) VALUES ('accepted'), ('running');",
    )?;
    ensure!(backup(&source, &destination)?);
    ensure!(fs::metadata(&destination)?.permissions().mode() & 0o077 == 0);
    let snapshot = Connection::open(&destination)?;
    ensure!(snapshot.query_row("SELECT COUNT(*) FROM events", [], |r| r.get::<_, i64>(0))? == 2);
    writer.execute("INSERT INTO events(record) VALUES ('completed')", [])?;
    ensure!(
        !backup(&source, &destination)?,
        "overwrote existing snapshot"
    );
    ensure!(snapshot.query_row("SELECT COUNT(*) FROM events", [], |r| r.get::<_, i64>(0))? == 2);
    let public = root.path().join("public");
    fs::create_dir(&public)?;
    fs::set_permissions(&public, fs::Permissions::from_mode(0o755))?;
    ensure!(!backup(&source, &public.join("snapshot.sqlite3"))?);
    let link = root.path().join("link.sqlite3");
    std::os::unix::fs::symlink(&source, &link)?;
    ensure!(!backup(&link, &root.path().join("bad.sqlite3"))?);
    Ok(())
}
