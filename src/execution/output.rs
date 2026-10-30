//! Drain both pipes to EOF while retaining only a bounded prefix.
use anyhow::Result;
use tokio::io::{AsyncRead, AsyncReadExt};

pub(super) struct Capture {
    pub text: String,
    pub bytes: u64,
    pub truncated: bool,
}

pub(super) async fn drain(mut pipe: impl AsyncRead + Unpin) -> Result<Capture> {
    let mut kept = Vec::new();
    let mut bytes = 0;
    let mut buffer = [0; 8192];
    loop {
        let read = pipe.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        bytes += read as u64;
        let remaining = (65536_usize).saturating_sub(kept.len());
        kept.extend_from_slice(&buffer[..read.min(remaining)]);
    }
    Ok(Capture {
        truncated: bytes > kept.len() as u64,
        text: String::from_utf8_lossy(&kept).into_owned(),
        bytes,
    })
}
