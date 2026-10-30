# Branch scope: feat/oauth-discovery

Compared with `main` (`fd50078`), this branch implements OAuth discovery and, at the operator's explicit request on 2026-10-30, public gateway support and audited SSH execution. The earlier hosts-only branch report is superseded.

| Area | Implemented boundary |
| --- | --- |
| Discovery | Static RFC 9728 JSON at the root compatibility URL and the configured resource-specific well-known path; path-specific 401 challenge with the required scope. No caller-derived URLs or issuer requests for metadata. |
| Authentication | Fresh Keycloak HTTPS introspection at the HTTP gate and inside every tool; the two principals must match. Invalid credentials return 401; issuer failures return 503. |
| Public access | Explicit `POSTERN_PUBLIC_PROXY=true` allows only the configured HTTPS origin/Host. TCP remains loopback; optional mode-0600 Unix socket supports Caddy. Forwarding headers cannot select identities, origins or metadata. |
| SSH | Optional separate operator configuration registers four execution tools. System OpenSSH, pinned host keys, private key files, no ambient configuration/agent/proxy, no local shell interpolation. |
| Sessions | Random principal/host-bound handles, 32 live/opening sessions, one active command per session, 300-second idle and 3600-second absolute expiry. Current grants are reloaded before dispatch. Closure/revocation blocks new work; dispatched commands drain. |
| Audit | SQLite WAL with FULL synchronization and exclusive writer ownership. Append accepted and running events before SSH command spawn. Read only the current issuer+subject's records. Recovery marks unfinished commands unknown. Write failure disables new executions. |
| Bounds | Commands at most 8192 bytes; timeout 1..60 seconds; stdout/stderr each retain at most 65536 bytes while pipes continue draining. Outputs are returned but not stored in audit. Public requests have a shared 30/second budget with burst 120, 32-request concurrency, 16 KiB bodies and a 15-second body-read deadline. Execution-enabled handlers have a 75-second overall deadline. |

The default remains inventory-only. SSH destinations, usernames, ports, key paths and host-key pins are administrator inputs, never tool inputs. No command-content policy or per-command server approval is added. [Execution details](execution.md) specify limitations and account switching.

Verification uses formatting, warnings-denied Clippy, Cargo tests, enforced 100-line source/config limits, a disposable genuine Keycloak realm and a disposable real OpenSSH daemon. Tests cover cross-user isolation, revocation, host-key mismatch, complex shell text, output overflow, timeout, recovery and audit-write failure before dispatch. Run `sh scripts/test_docker.sh` for the complete suite.

Live Gont deployment and the actual ChatGPT callback/passkey round-trip are recorded separately in `deploy/gont/verification.json`; do not infer those from local tests. The ChatGPT connection can require user interaction and depends on the account's available custom MCP capabilities.

Remaining scope: interactive shells/PTYS, durable background jobs, file transfer, admin-wide audit API, host activity auditing, tamper-proof/off-host audit replication, and power-loss guarantees beyond SQLite/filesystem durability. Remote execution now requires systemd user-service supervision: confirmed timeouts require manager evidence and an empty cgroup. Lost observation is unknown with termination unconfirmed. Legacy timeout records predate this guarantee. These limitations must not be represented as completed acceptance criteria.

## Verification on 2026-10-30

Formatting, warnings-denied Clippy, 26 unit tests, native mock integration, source-size checks and a complete disposable Keycloak/OpenSSH Docker suite passed locally. The Docker suite also passed on Gont after fixing an existing test race: writing excess chunks after a correct early 413 could fail with EPIPE. The final SDK Host correction has a real authenticated public-Host-through-Unix-socket SSH regression and passed live on Gont. Cargo-audit 0.22.2 scanned 198 locked crates against 1,386 advisories without warnings. CI was not observed.

The public deployment now uses the existing Leyline gateway with a dedicated FastMCP OAuthProxy adapter, Redis-backed encrypted state and the fixed Keycloak callback documented in [OAuth setup](oauth.md). The user confirmed login and real command/audit operations. The adapter implementation and deployment records live in the infrastructure repository. The original direct-Keycloak/manual-callback acceptance description is historical.

A subsequent live user test revealed that killing local SSH did not stop remote commands. The follow-up replaces that behavior with target-side systemd cgroup deadlines and explicit termination evidence. Its regression suite covers delayed markers, TERM-ignoring shells, new process groups, background children, connection loss and independent-session concurrency. Consult the current deployment record for which revision is live; the original `timed_out` result alone was not evidence of termination.
