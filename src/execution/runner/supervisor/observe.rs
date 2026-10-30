//! A separate SSH channel carries manager status, never user command output.
use super::Observed;
use anyhow::{Result, bail, ensure};

pub(super) fn script(id: &str, seconds: u64) -> String {
    // Only server-generated UUIDs and validated integers enter this script.
    // Retain the unit until observation; the pipe waiter exits after stop below.
    format!(
        r#"
unit=postern-{id}.service
n=0
while [ "$n" -lt {attempts} ]; do
    state= result= code= status= group=
    data=$(systemctl --user show "$unit" -p SubState -p Result -p ExecMainCode -p ExecMainStatus -p ControlGroup 2>/dev/null) || data=
    while IFS='=' read -r key value; do
        case "$key" in
            SubState) state=$value;; Result) result=$value;;
            ExecMainCode) code=$value;; ExecMainStatus) status=$value;;
            ControlGroup) group=$value;;
        esac
    done <<END
$data
END
    case "$state" in
        exited|failed)
            if [ -n "$group" ]; then
                events=/sys/fs/cgroup$group/cgroup.events
                if [ -e "$events" ] && ! grep -qx 'populated 0' "$events"; then
                    n=$((n+1)); sleep 0.1; continue
                fi
            fi
            printf '%s\n' "$result" "$code" "$status"
            systemctl --user stop "$unit" >/dev/null 2>&1 || exit 1
            systemctl --user reset-failed "$unit" >/dev/null 2>&1 || :
            exit 0;;
    esac
    n=$((n+1))
    sleep 0.1
done
exit 1
"#,
        attempts = (seconds + 4) * 10
    )
}

pub(super) fn parse(data: &[u8]) -> Result<Observed> {
    let text = std::str::from_utf8(data)?;
    let lines: Vec<_> = text.lines().collect();
    ensure!(lines.len() == 3, "invalid supervisor response");
    let code: i32 = lines[1].parse()?;
    let status: i32 = lines[2].parse()?;
    let (outcome, exit_code) = match (lines[0], code) {
        ("timeout", _) => ("timed_out", None),
        ("success" | "exit-code", 1) if (0..=255).contains(&status) => ("completed", Some(status)),
        ("signal" | "core-dump", 2 | 3) => ("terminated", None),
        _ => bail!("supervisor did not confirm command completion"),
    };
    Ok(Observed {
        outcome,
        exit_code,
        result: lines[0].into(),
    })
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn rejects_missing_or_incomplete_manager_evidence() {
        for data in [
            "",
            "timeout\n",
            "unknown\n0\n0\n",
            "success\n0\n0\n",
            "success\n1\n0\nextra\n",
        ] {
            assert!(parse(data.as_bytes()).is_err());
        }
    }

    #[test]
    fn remote_exit_255_is_distinct_from_transport_failure() {
        let result = parse(b"exit-code\n1\n255\n").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(result.outcome, "completed");
        assert_eq!(result.exit_code, Some(255));
    }
}
