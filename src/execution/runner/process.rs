//! Drain output while independently confirming remote cgroup completion.
use super::supervisor::{self, Observed};
use crate::execution::{output, session::Session};
use anyhow::Result;
use std::time::Duration;

pub(super) async fn run(
    mut child: tokio::process::Child,
    session: &Session,
    id: &str,
    seconds: u64,
) -> Result<(Observed, output::Capture, output::Capture)> {
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
    let observed = tokio::time::timeout(Duration::from_secs(seconds + 6), async {
        let observed = supervisor::observe(session, id, seconds).await?;
        // Observation stops the retained unit and releases systemd-run's pipe waiter.
        child.wait().await?;
        Ok::<_, anyhow::Error>(observed)
    })
    .await;
    let result = match observed {
        Ok(Ok(observed)) => Ok(observed),
        _ => {
            // This is transport cleanup only: callers must see unknown/unconfirmed.
            let _ = child.kill().await;
            Err(anyhow::anyhow!("remote termination unconfirmed"))
        }
    };
    let stdout = out.await??;
    let stderr = err.await??;
    Ok((result?, stdout, stderr))
}
