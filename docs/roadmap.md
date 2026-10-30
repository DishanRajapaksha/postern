# Implementation and verification plan

Current implementation and verification are described in [branch scope](branch-scope.md) and [execution](execution.md). Design proposals below are requirements, not evidence that every acceptance gate has passed.

Status: project roadmap. Read-only Keycloak-introspection MCP host-inventory development slice implemented in [PR #1](https://github.com/DishanRajapaksha/postern/pull/1), with Rust formatting, Clippy, 21 unit tests, mock MCP HTTP tests and Docker integration against disposable Keycloak passing locally on 2026-10-29. See [branch scope validation](branch-scope.md) and [security hardening](hardening.md). The expanded branch implements public proxy support, SSH sessions/execution and auditing; live deployment and ChatGPT login require the separate acceptance record.

## First implementation slice: read-only local MCP inventory (2026-10-29)

- [x] Rust Streamable HTTP MCP server with `hosts()` and explicit JSON host IDs/descriptions.
- [x] Loopback-only development endpoint with no SSH operations.
- [x] Strict inventory validation and unit tests (21 passed across configuration, authentication, HTTP limits and host visibility).
- [x] GitHub Actions formatting, Clippy, unit tests and MCP HTTP smoke test.
- [x] Commit dependency lockfile, enforce locked builds and declare/test Rust 1.93 as the minimum toolchain.
- [x] Keycloak introspection gate on every HTTP request, stateless MCP, subject-scoped host visibility and deny-by-default policy (verified against disposable real Keycloak; live user integration remains outstanding).
- [x] Docker integration with real Keycloak over verified HTTPS, including concurrent subject isolation, expiry, logout revocation and fail-closed authentication errors; CI job added.
- [x] Implement RFC 9728 OAuth protected-resource metadata and 401 discovery challenges (PR #2, still in review).
- [ ] Configure/verify real ChatGPT OAuth client registration and login through Keycloak, including PKCE S256, resource/audience binding and final public HTTPS integration, before SSH execution.

This slice does **not** satisfy the Phase 0 ChatGPT integration gate or the Phase 1 SSH execution milestone.

## Phase 0: validate feasibility and freeze contract

- [ ] Verify the user's **actual ChatGPT account/integration path** can connect to a private custom HTTP MCP tool and issue an `exec` call. Record observed restrictions; client-side confirmations cannot be bypassed by Postern.
- [ ] Select the concrete Rust MCP SDK version and supported HTTP/OAuth integration, with upstream documentation.
- [ ] Review security/operations requirements, sample config, accepted command/audit scope and output retention.
- [ ] Specify session, error and audit schemas with fixture examples and explicit concurrency invariants.
- [ ] Choose development target and isolated disposable SSH host.

**Gate:** verified end-to-end tool invocation in the target client, or a documented limitation with a workable alternative client/integration. Do not claim ChatGPT compatibility solely from compiling a server.

## Phase 1: minimal end-to-end implementation

- [ ] Rust project layout, locked dependencies, formatting/lint/test pipeline.
- [ ] Streamable HTTP MCP with `hosts`, `open_session`, `exec`, `close_session`, `audit` tools.
- [ ] Explicit configured SSH target allowlist and key paths; system OpenSSH launcher with strict pinned host verification.
- [ ] Session registry with actor ownership, deadlines and safe connection-multiplexing socket management.
- [ ] SQLite audit schema and transactional intent write **before** spawn, bounded output capture and final status writes.
- [ ] No shell-content filtering or per-command policy layer.
- [ ] Integration tests using disposable SSH targets, including intentional non-zero exits and complex shell scripts.

**Gate:** one authenticated test principal can execute a command on an authorised test host with a verified audit record.

## Phase 2: close security and failure gaps

- [ ] Keycloak issuer/audience/signature/expiry/scope validation on every request.
- [ ] Per-principal host authorisation and cross-user session/audit isolation.
- [ ] Audit-database unavailable-before-dispatch test; crash injection at every state transition.
- [ ] Output size, timeout, cancellation and simultaneous invocation tests.
- [ ] Strict host-key mismatch tests and credential/path/SSH option injection tests.
- [ ] Session expiry, restart cleanup and revocation tests.
- [ ] Static security checks, dependency vulnerability scanning and minimal privilege deployment.

**Gate:** all Must requirements and acceptance criteria in [requirements.md](requirements.md) pass; manual security review completed.

## Phase 3: operational hardening

- [ ] systemd unit and installation documentation with safe directory permissions.
- [ ] Keycloak integration guide, reverse proxy/TLS examples and private/Tailscale target routing.
- [ ] Metrics, health checks, audit retention and backups.
- [ ] Off-host audit replication and detection of replication lag; decide whether production requires it.
- [ ] Recovery exercises, key rotation and incident-response runbook.

**Gate:** failure drills demonstrate clear state/unknown outcomes and recoverable operation, including audit integrity checks.

## Optional later work

- [ ] Host-level auditd/eBPF/terminal capture integration, accurately described as additional evidence rather than perfect visibility.
- [ ] File transfer tools with explicit path policy and auditing.
- [ ] Streaming command output and robust remote job handles.
- [ ] Interactive PTY support only after documenting its auditing, session isolation and escape/terminal-security implications.
- [ ] Multiple auth providers or management API, only if operational demand justifies them.

## Proposed repository structure (future)

```text
postern/
  README.md
  docs/
    requirements.md
    architecture.md
    mcp-api.md
    security.md
    auditing.md
    operations.md
    roadmap.md
    adr/
  crates/ or src/            # implementation, not yet created
  tests/                      # integration/adversarial fixtures
  packaging/                  # optional systemd/config examples
```

## Invariants worth enforcing in tests

- No pre-dispatch audit commit -> no SSH execution.
- No token/host authorisation -> no SSH execution.
- No host-key pin match -> no SSH execution.
- Another principal cannot reuse a session ID.
- A completed HTTP response is not evidence of remote success unless the SSH exit result is observed.
- No retries of side-effecting command execution without a designed idempotency protocol.
- Every command accepted produces a unique command ID, but a crash may make its final remote outcome **unknown**.
- Parsing/classifying commands is outside the product: acceptance tests include arbitrary shells and scripts against a throwaway machine.
