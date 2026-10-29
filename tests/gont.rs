//! Operator-only native tests: run explicitly as root on Gont, never in ordinary CI.
mod live;
mod support;

#[tokio::test]
#[ignore = "provisions the dedicated Postern preview on Gont"]
async fn prepare_gont_preview() -> anyhow::Result<()> {
    live::setup::run().await
}

#[tokio::test]
#[ignore = "tests and cleans diagnostic principals on the installed Gont preview"]
async fn verify_gont_preview() -> anyhow::Result<()> {
    live::accept::run().await
}
