//! Live non-root target acceptance through the authenticated native MCP boundary.
use super::tool::call;
use crate::support::client::Mcp;
use anyhow::{Context, Result, ensure};
use serde_json::json;
use std::time::{Duration, Instant};

pub(super) async fn run(mcp: &Mcp, token: &str, session: &str) -> Result<()> {
    let marker = format!("/tmp/postern-deadline-{}", uuid::Uuid::new_v4());
    for script in [
        format!("sleep 5; touch {marker}"),
        format!("trap '' TERM; sleep 5; touch {marker}"),
        format!("setsid sh -c 'trap \"\" TERM; sleep 5; touch {marker}' & wait"),
        format!("(sleep 5; touch {marker}) >/dev/null 2>&1 &"),
    ] {
        let result = call(
            mcp,
            token,
            "exec",
            json!({
                "session_id":session,"command":script,"timeout_seconds":1
            }),
        )
        .await?;
        ensure!(result["record"]["outcome"] == "timed_out");
        ensure!(result["record"]["termination_confirmed"] == true);
        ensure!(result["record"]["supervisor_result"] == "timeout");
        tokio::time::sleep(Duration::from_secs(5)).await;
        let check = call(
            mcp,
            token,
            "exec",
            json!({
                "session_id":session,"command":format!("test ! -e {marker}")
            }),
        )
        .await?;
        ensure!(
            check["record"]["exit_code"] == 0,
            "delayed side effect survived deadline"
        );
    }
    for status in [0, 23, 255] {
        let result = call(
            mcp,
            token,
            "exec",
            json!({"session_id":session,"command":format!("exit {status}")}),
        )
        .await?;
        ensure!(
            result["record"]["outcome"] == "completed" && result["record"]["exit_code"] == status
        );
    }
    let second = call(mcp, token, "open_session", json!({"host_id":"gont"})).await?;
    let other = second["session_id"].as_str().context("second session")?;
    let command = |session| json!({"session_id":session,"command":"sleep 3"});
    let started = Instant::now();
    let (a, b) = tokio::join!(
        call(mcp, token, "exec", command(session)),
        call(mcp, token, "exec", command(other))
    );
    ensure!(a?["record"]["exit_code"] == 0 && b?["record"]["exit_code"] == 0);
    let elapsed = started.elapsed();
    ensure!(
        elapsed < Duration::from_secs(5),
        "native backend serialized independent sessions"
    );
    call(mcp, token, "close_session", json!({"session_id":other})).await?;
    println!(
        "PASS: four confirmed cgroup deadlines; no delayed markers; exit codes 0/23/255; two sleep-3 sessions in {:.2}s",
        elapsed.as_secs_f64()
    );
    Ok(())
}
