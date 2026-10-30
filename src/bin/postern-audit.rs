//! Offline operator tool; never expose this interface through MCP.
#[path = "../audit_copy/mod.rs"]
mod audit_copy;

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    anyhow::ensure!(
        args.len() == 3 && args[0] == "backup",
        "usage: postern-audit backup SOURCE.sqlite3 NEW-DESTINATION.sqlite3"
    );
    audit_copy::copy(args[1].as_ref(), args[2].as_ref())?;
    println!("Verified audit snapshot created; source unchanged.");
    Ok(())
}
