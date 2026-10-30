# Product requirements

Current implementation and verification are described in [branch scope](branch-scope.md) and [execution](execution.md). Design proposals below are requirements, not evidence that every acceptance gate has passed.

Status: requirements baseline, reconciled on **2026-10-30**. This is broader than the shipped implementation: admin-wide audit access and operational failure drills remain open. See [current scope](branch-scope.md), [execution semantics](execution.md) and the [roadmap](roadmap.md) for implemented behavior and recorded acceptance.

## Purpose

Postern is a small self-hosted MCP-to-SSH gateway for a trusted operator using ChatGPT or another MCP client. It is deliberately able to perform arbitrary remote shell actions *after* a principal is authorised to access a particular configured host and SSH identity. Every execution request must leave a durable audit trail.

The user has explicitly requested **unrestricted commands and auditing**, with no server-side per-command approval engine. Implementation language is flexible; Rust is currently preferred, not mandatory.

## Goals

- Connect ChatGPT, when its available integration supports these tools, to private SSH hosts over an authenticated HTTPS MCP endpoint.
- Authenticate users with Keycloak/OIDC and authorise *host/session access* rather than command contents.
- Reuse mature OpenSSH instead of implementing SSH cryptography or transport.
- Permit arbitrary shell strings: pipelines, scripts, redirections, background commands and standard administrative operations within the remote account's permissions.
- Persist an attributable audit record **before** dispatching each command, including failed, denied, timed-out and uncertain execution outcomes.
- Preserve a small operational footprint, readable code and straightforward deployment.

## Functional requirements

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-01 | Must | Provide an HTTPS Streamable HTTP MCP endpoint with the standard MCP tool discovery/call flow. |
| FR-02 | Must | Validate every request's access token and derive a stable principal identity; do not accept caller-supplied identities. |
| FR-03 | Must | List only administrator-configured SSH target IDs authorised for that principal. No user-supplied arbitrary destination, username, port, proxy or private-key path. |
| FR-04 | Must | Create an opaque, principal-bound, host-bound, time-limited SSH session. |
| FR-05 | Must | Establish SSH only through system OpenSSH, with key-based noninteractive authentication and strict server host-key verification. |
| FR-06 | Must | Execute **any** shell command supplied by the authorised caller without command parsing, classification, sanitisation, rewriting or approval at the Postern layer. Separate command data from *local* process argument construction. |
| FR-07 | Must | Capture command outcome, exit status, timestamps and bounded stdout/stderr; report truncation explicitly. |
| FR-08 | Must | Write an execution-intent audit record durably before command dispatch; if that write fails, do not dispatch. |
| FR-09 | Must | Attempt to finalise audit records after execution and mark unfinished runs as **unknown** or **interrupted** on recovery, never silently classify them as successful/failed. |
| FR-10 | Must | Enforce session ownership, idle/absolute expiry, revocation and a maximum concurrent-session limit. |
| FR-11 | Must | Offer explicit session closure; release connection resources and reject subsequent use of the session ID. |
| FR-12 | Must | Permit authorised actors to query their audit records; provide an admin-only view for broader investigation. |
| FR-13 | Must | Avoid exposing private keys, passwords, access tokens, SSH control paths and sensitive configuration through MCP results, telemetry or errors. |
| FR-14 | Must | Report transport errors, command errors, authorisation errors and indeterminate outcomes distinctly. |
| FR-15 | Must | Reject execution when session or host authorisation has been revoked, even if the SSH master connection is still alive. |
| FR-16 | Should | Recover from process crashes by expiring orphan sessions and reconciling unfinished audit attempts. |
| FR-17 | Should | Provide rate limits, maximum command length, output limits and execution timeouts as resource bounds, **not command-content censorship**. |
| FR-18 | Later | Optional host-side process/terminal auditing and reliable off-host audit replication. |
| FR-19 | Later | Optional file transfer, streaming, interactive PTY or durable jobs, each with separately specified auditing semantics. |

## Non-functional requirements

- **Security:** treat the MCP endpoint as privileged remote-code-execution infrastructure. Authenticate every call, authorise every operation and make host destinations explicit.
- **Audit durability:** an accepted command cannot start without a committed audit-intent event. An audit write failure after dispatch must produce an observable degraded/unknown state, not a fabricated clean outcome.
- **Simplicity:** one service, one deployment, minimal internal state; use existing implementations for cryptography, OIDC validation and SSH.
- **Operational clarity:** structured logs, health checks and separate metrics; never make health endpoints execute shell code.
- **Concurrency:** simultaneous commands have distinct IDs and cannot collide in session, output or audit records.
- **Portability:** Linux is the initial server target. Other server OSes are not required in v1.
- **Transparency:** clearly document which actions can and cannot be accounted for by MCP-level audit logging.

## Non-goals for v1

- Per-command approval, command denylisting, command risk scoring, shell parsing or LLM safety classification.
- Circumventing confirmations or restrictions imposed by ChatGPT or any other MCP client.
- Arbitrary hostnames entered by users, SSH password handling or private-key uploads from ChatGPT.
- A web terminal, multi-user collaboration interface, fleet inventory or secret-management product.
- Persistent interactive shell/PTY state between `exec` calls.
- A guarantee of capturing every subprocess, shell built-in, file write or network action on the remote host.
- A full job scheduler or persistent background-task manager.

## Acceptance criteria

1. A missing, expired, wrongly issued or wrong-audience token cannot list targets, open sessions, execute commands or read private audit data.
2. A valid user can access only host profiles explicitly granted by policy. Another user cannot reuse or inspect that user's session.
3. Known-host key mismatches fail closed. No unattended trust-on-first-use.
4. A valid session can execute deliberately complex or destructive shell syntax **within its SSH account privileges**, without an MCP content filter. Tests use a disposable environment.
5. Every execution that begins has a durable, uniquely identified audit intent created first.
6. Storage-unavailable-before-dispatch tests prove *zero* SSH command launches.
7. Normal completion, non-zero exit, SSH disconnect, timeout, process crash and cancellation produce distinguishable outcomes; uncertain ones remain uncertain.
8. Output overflow does not deadlock child processes and is marked as truncated.
9. Session expiry/revocation blocks new executions even with an existing multiplexed SSH connection.
10. Access to raw commands and captured output is restricted because they may contain secrets.
11. Tests include concurrent callers, stale sessions, repeated identifiers, invalid tokens and attempts to inject arbitrary hosts or SSH options.
12. A real ChatGPT MCP tool invocation is demonstrated in the user's actual supported integration before claiming ChatGPT compatibility.

## Unresolved choices

- The owner confirmed OAuthProxy login and command/audit operations through the actual integration on 2026-10-30. The user subsequently confirmed the connector timeout fix for the reproduced command; broader connector child-process/cancellation-race coverage and comparison with the [local native benchmark](local-verification.md) remain open. See the [connector report](connector-regression.md); client confirmations and restrictions remain outside Postern's control.
- Exact production host list, per-host account privileges, Keycloak realm/client configuration and how the gateway reaches Tailscale hosts.
- Current policy stores exact command text and outcome metadata without stdout/stderr bodies. Any future output retention requires a separate privacy and storage decision.
- Current session/command limits are documented in [execution](execution.md). Review their capacity and suitability before expanding deployment; illustrative values elsewhere are not configuration guarantees.
- Whether to require off-host audit forwarding at initial launch or as a production hardening gate.
