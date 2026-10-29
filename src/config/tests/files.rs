//! Startup rejects unavailable, malformed and oversized files.
use super::super::Config;
use std::fs;
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
    fs::write(temp.path(), " ".repeat(super::super::MAX_CONFIG_BYTES + 1)).unwrap();
    assert!(Config::load(temp.path()).is_err());
}
