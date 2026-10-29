//! Count physical lines, so comments and blank lines also count toward the limit.
use anyhow::{Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn collect(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect(&entry.path(), files)?;
        } else if kind.is_file() {
            let path = entry.path();
            let extension = path.extension().and_then(|v| v.to_str()).unwrap_or("");
            if matches!(
                extension,
                "rs" | "py" | "sh" | "yml" | "yaml" | "json" | "toml"
            ) || path.file_name().is_some_and(|v| v == "Dockerfile")
            {
                files.push(path);
            }
        }
    }
    Ok(())
}

#[test]
fn handwritten_files_do_not_exceed_one_hundred_lines() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![root.join("Cargo.toml")];
    for directory in ["src", "scripts", "tests", ".github", "config"] {
        collect(&root.join(directory), &mut files)?;
    }
    for path in &files {
        let count = fs::read_to_string(path)?.lines().count();
        ensure!(
            count <= 100,
            "{}: {count} lines (maximum 100)",
            path.display()
        );
    }
    println!(
        "Checked {} handwritten code/configuration files",
        files.len()
    );
    Ok(())
}
