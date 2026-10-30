//! Bound the process lifetime and drain both pipes concurrently.
use crate::execution::output;
use anyhow::Result;
use std::time::Duration;

pub(super) async fn run(
    mut child: tokio::process::Child,
    seconds: u64,
) -> Result<(Option<i32>, bool, output::Capture, output::Capture)> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("missing stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("missing stderr"))?;
    let out = tokio::spawn(output::drain(stdout));
    let err = tokio::spawn(output::drain(stderr));
    let (status, timed_out) =
        match tokio::time::timeout(Duration::from_secs(seconds), child.wait()).await {
            Ok(status) => (status?.code(), false),
            Err(_) => {
                child.kill().await?;
                (None, true)
            }
        };
    // The local ssh process owns these pipes; killing it releases the readers.
    Ok((status, timed_out, out.await??, err.await??))
}
