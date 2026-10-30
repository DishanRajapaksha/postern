# Local audit recovery and concurrency verification

Verification started on **2026-10-30** and completed on **2026-10-31** against the uncommitted worktree based on `f2cd8e5`. This is local test evidence, separate from Gont deployment or actual connector acceptance. Run `sh scripts/test_docker.sh` to reproduce the disposable real Keycloak/OpenSSH suite. The fixture uses root inside an isolated privileged container to exercise real systemd/cgroup and filesystem behavior; deployed non-root acceptance remains separate.

## Audit backup and restart

- `tests/backup.rs` snapshots a live SQLite WAL writer, confirms committed events are included, and checks that later source writes do not change the snapshot. Existing destinations, public destination directories and source symlinks are rejected; snapshot permissions are private.
- `tests/ssh/recovery.rs` waits for a remote dispatch marker, snapshots the live audit, kills the gateway, restarts it and then restores the snapshot into a fresh directory. Both restarts reject the old session handle and expose the unfinished record as `unknown` with termination unconfirmed. Abandoned SSH runtime directories are reclaimed, and the delayed remote side effect does not occur after the independent deadline.
- `tests/ssh/runtime.rs` starts a second gateway with a separate audit directory and the same runtime directory. It must fail without removing the first gateway's control socket; the original session must still execute successfully.
- `tests/ssh/finalization.rs` injects a final-write rejection after remote dispatch. Observed output and exit 23 remain in the degraded response, further execution is denied, and restart recovers the missing durable result as unknown.
- `tests/ssh/storage.rs` exhausts a private 1 MiB tmpfs and verifies pre-dispatch denial without a side effect. Removing the filler does not clear the degraded latch. Read-only remount blocks startup; repair and restart permit an audited exit-23 command.
- `tests/ssh/races.rs` closes a busy session, verifies new work is denied and checks that its TERM-ignoring command leaves no delayed marker. Near-deadline completion races must report either the observed exit code or a manager-confirmed timeout, with matching audit evidence.

These drills do not cover physical power loss, every crash boundary, session-expiry races, off-host replication, deployed recovery/rotation, or connector-originated cancellation.

## Native concurrency measurement

`tests/ssh/benchmark.rs` runs three sequential/parallel pairs through authenticated native MCP HTTP and separate SSH sessions. Each command prints remote timestamps around a one-second sleep. The test requires at least 0.5 seconds of actual remote overlap in each parallel pair and correlates all 12 unique command IDs with their durable audit records.

The final complete run measured:

| Pair | Sequential seconds | Parallel seconds | Remote overlap seconds |
| --- | ---: | ---: | ---: |
| 1 | 2.114 | 1.063 | 1.002 |
| 2 | 2.119 | 1.164 | 1.002 |
| 3 | 2.124 | 1.064 | 1.002 |

These are fixture measurements, not a throughput guarantee. Comparison with the public adapter and actual connector remains open; these results alone do not locate connector serialization.

## Quality gates

Formatting, warnings-denied Clippy, all non-ignored Cargo tests, the 100-line handwritten-file limit, genuine Keycloak integration, and the complete OpenSSH recovery/storage suite passed. `cargo audit --deny warnings` scanned 198 locked dependencies against 1,395 advisories without findings. Current-worktree GitHub CI and deployment acceptance are separate checks.
