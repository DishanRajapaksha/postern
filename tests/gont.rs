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

#[tokio::test]
#[ignore = "reports only security-claim predicates, never tokens or credentials"]
async fn diagnose_gont_claims() -> anyhow::Result<()> {
    live::diagnose::run().await
}

#[tokio::test]
#[ignore = "recreates temporary diagnostics after a failed initial acceptance"]
async fn prepare_gont_diagnostics() -> anyhow::Result<()> {
    live::setup::diagnostics().await
}

#[tokio::test]
#[ignore = "configures only the dedicated Postern OAuth login client on Gont"]
async fn configure_gont_public_client() -> anyhow::Result<()> {
    live::public::configure().await
}

#[tokio::test]
#[ignore = "exercises public audited SSH on Gont and removes diagnostic grants/client"]
async fn verify_gont_public_execution() -> anyhow::Result<()> {
    live::public_accept::run().await
}

#[tokio::test]
#[ignore = "tests target-side deadlines through native MCP with a temporary real issuer principal"]
async fn verify_gont_native_supervision() -> anyhow::Result<()> {
    live::public_accept::run_native().await
}
