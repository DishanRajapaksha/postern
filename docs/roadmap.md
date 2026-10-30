# Implementation and verification roadmap

Status as of **2026-10-30**, reconciled with merged PRs #1–#3 on `main`. Postern implements inventory, OAuth discovery, opt-in public gateway access, principal-bound SSH sessions, durable audit and target-side command deadlines. Inventory-only loopback mode remains the default.

This roadmap separates implemented behavior, recorded acceptance and work still required. It does not certify production readiness. [Current scope](branch-scope.md) and [execution semantics](execution.md) describe the supported boundary.

## Recorded evidence

- [Deployment verification](../deploy/gont/verification.json): user-confirmed OAuthProxy login and command/audit operations, real issuer checks, non-root Gont SSH and service isolation.
- [Connector regression report](connector-regression.md): user-reported timeout fix in the reproduced scenario, successful binary round-trip, permission restrictions, failed-command reporting, output limits, auditing and session closure. Concurrent execution remained sequential through the connector.
- [Supervision verification](../deploy/gont/supervision-verification.json): live native MCP deadline tests and Docker systemd/OpenSSH regressions, including transport loss and concurrent sessions.
- The records identify deployed source `0afb9fef7c76bac064a73b186e73c9c2cea53486` and its binary hash. `main` includes this work through merge `f2cd8e5`; a merge commit is not a new deployment attestation.
- The recorded local checks passed: formatting, warnings-denied Clippy, 29 unit tests, native integration, source-size enforcement, full Keycloak/OpenSSH Docker suite and cargo-audit without warnings. Adapter tests passed in the exact deployed FastMCP runtime; that implementation lives in the infrastructure repository.
- The original deployment record did not observe CI. GitHub run [41139123668](https://github.com/DishanRajapaksha/postern/actions/runs/41139123668) was subsequently inspected: `check`, `integration` and `audit` passed for merged `main` at `f2cd8e5`. This does not establish CI for the current uncommitted worktree.
- [Local recovery and concurrency verification](local-verification.md): live WAL backup/restore, gateway crash/restart, runtime ownership, final-write failure, real disk-full/read-only storage and repeated native concurrency measurements. This evidence applies to the local worktree, not the deployed service.

## Next acceptance and maintenance work

- [x] Record the actual connector timeout retest: the user confirmed the fix for the reproduced command scenario. See the [regression report](connector-regression.md) for scope and evidence limits.
- [ ] Extend connector acceptance to child processes and cancellation races; capture side-effect checks and termination evidence without treating unknown outcomes as confirmed kills.
- [x] Benchmark native MCP independently of ChatGPT using separate sessions, repeated sequential/parallel measurements and audit correlation. See the [local results](local-verification.md).
- [ ] Compare those native measurements with the public adapter/connector path to locate serialization.
- [x] Observe all three CI jobs passing for merged `main`; inspect branch protection and repository rulesets separately.
- [ ] Require the passing checks and pull requests on `main` using the tracked protection policy.
- [x] Exercise audit backup/restore and gateway crash/restart during execution locally; verify unresolved records remain unknown and old session handles fail. Repeat against the deployed service before claiming deployment acceptance.
- [x] Test real disk-full/read-only storage and post-dispatch audit failure in the disposable Docker fixture. Physical power loss remains untested.
- [x] Document credential rotation, host-key replacement, incident response and backup/restore in the [recovery runbook](recovery.md).
- [ ] Exercise rotation and incident procedures against the deployed service; local recovery drills do not complete deployment acceptance.

## Feasibility, identity and client integration

- [x] Use the official Rust MCP SDK with Streamable HTTP, Tokio/Axum, locked dependencies and Rust 1.93.
- [x] Implement native RFC 9728 resource metadata and path-specific Bearer challenges.
- [x] Validate fresh Keycloak HTTPS introspection at HTTP admission and every tool dispatch; require matching principals, issuer, audience, scope and valid time claims. Keycloak performs token validation; the Rust core does not implement local JWT/JWKS verification.
- [x] Test genuine Keycloak tokens, expiry, logout revocation, subject isolation and fail-closed issuer errors in Docker.
- [x] Deploy the dedicated FastMCP OAuthProxy adapter with automatic registration, CSRF consent, S256 and the fixed upstream Keycloak callback.
- [x] Record user-confirmed owner login and actual command/audit operations on 2026-10-30.
- [x] Record user-confirmed connector timeout cancellation for the reproduced command. Broader child-process/cancellation-race acceptance remains open.

Client confirmations and restrictions remain outside Postern's control. Repeat owner login/tool acceptance after relevant OAuth or connector changes.

## SSH execution and audit foundation

- [x] Provide `hosts`, `open_session`, `exec`, `close_session` and `audit` in opt-in execution mode.
- [x] Restrict destinations and credentials to administrator-configured profiles; use system OpenSSH with pinned host keys and no ambient agent/configuration.
- [x] Bind sessions to principal and host with idle/absolute deadlines, bounded capacity and one active command per session.
- [x] Reload grants before dispatch, reject cross-user access and block new commands after closure/revocation.
- [x] Commit accepted/running SQLite audit events before command spawn; fail closed on pre-dispatch write failure.
- [x] Recover unfinished records as unknown on store reopen; restrict audit queries to the current actor.
- [x] Capture bounded output while draining excess bytes; preserve arbitrary shell text and observed non-zero exit codes.
- [x] Test real SSH execution, host-key mismatch, grant revocation, isolation, output overflow and injected pre-dispatch audit failure.
- [ ] Add crash injection across accept/dispatch/result boundaries and end-to-end session expiry/restart cleanup coverage.

The current audit stores exact commands and outcome metadata, not stdout/stderr bodies. Admin-wide audit queries remain unimplemented; the broader requirements are not all satisfied.

## Remote deadlines and concurrency

- [x] Require target-side systemd user services and cgroup v2; provide no unsupervised fallback.
- [x] Confirm deadline termination using service-manager evidence and an empty command cgroup.
- [x] Verify delayed markers, TERM-ignoring children, detached process groups and background descendants do not survive tested deadlines.
- [x] Report lost observation as unknown with termination unconfirmed while the independent remote deadline continues.
- [x] Test shell dollar/percent preservation, exit codes 0/23/255 and independent-session concurrency.
- [ ] Locate the observed connector serialization using the independent native benchmark above. Native and exact-runtime adapter concurrency passed; the source of user-observed sequential acceptance remains unresolved.

Supervision cannot undo completed writes or cancel external jobs. The target account and service manager remain trusted. Historical timeout records with null termination fields do not prove remote termination.

## Security and operational hardening

- [x] Enforce Host/Origin checks, body/concurrency/deadline limits and a shared public request-rate budget.
- [x] Enforce the 100-line handwritten source/config limit, locked builds, formatting, Clippy and configured CI/RustSec jobs.
- [x] Record a clean cargo-audit scan; ongoing scans and patch review remain necessary.
- [x] Deploy an unprivileged service with private credentials, audit/runtime directories and a private proxy socket; keep native TCP loopback-only.
- [x] Accept non-root Gont execution without sudo and record existing-service health checks.
- [ ] Review connection/header budgets, per-principal limits, monitoring, health/readiness signals and alerts.
- [ ] Define audit retention, SQLite-aware backups and tested restore/integrity procedures.
- [ ] Decide whether production requires off-host audit replication; implement lag detection and outage policy if selected.
- [ ] Complete failure drills and an independent security review before claiming broader production readiness.
- [ ] Review separate credentials, grants and acceptance before enabling any privileged target account.

## Optional later work

- [ ] Host-level auditd/eBPF/terminal capture with explicit visibility limits.
- [ ] File transfer with path policy and audit semantics.
- [ ] Streaming output and durable job handles.
- [ ] Interactive PTY support after reviewing isolation and terminal-security implications.
- [ ] Admin-wide audit API or additional auth providers only when required.

## Invariants to preserve

- A command launches only with durable pre-dispatch audit commits, a valid identity, a current host grant and a pinned host-key match.
- Another principal cannot reuse a session or query its audit records.
- A successful HTTP response alone does not establish remote command success.
- Lost execution evidence remains unknown; a timeout is confirmed only with target-side evidence.
- Side-effecting commands are never transparently retried.
- Each invocation has a unique command ID; command text is unrestricted within the configured SSH account's privileges.
