# Architecture and execution lifecycle

Current implementation and verification are described in [branch scope](branch-scope.md) and [execution](execution.md). Design proposals below are requirements, not evidence that every acceptance gate has passed.

Status as of **2026-10-30**: the core architecture is implemented. The lifecycle below reflects the merged implementation; shutdown/recovery goals still require the failure drills tracked in the [roadmap](roadmap.md). Gont adds a dedicated FastMCP OAuthProxy adapter before the private backend; see [OAuth setup](oauth.md).

## Trust boundaries

```text
Untrusted prompts and MCP command arguments
                    |
     ChatGPT / other MCP client
                    |
             TLS + OAuth bearer
                    |
  [HTTPS reverse proxy / optional tunnel]
                    |
          Postern (Keycloak introspection)
      +-------------+---------------+
      | authn/authz | durable audit |
      | host policy | session state |
      +-------------+---------------+
                    |
           system OpenSSH process
         strict known_hosts + keys
                    |
      private/Tailscale/network path
                    |
        remote SSH daemon + OS user
```

The MCP client and prompt content are **not** trusted to select a host, identity, audit principal or SSH option. The authenticated principal's policy determines the eligible host profiles. Commands themselves are treated as intentionally arbitrary and dangerous.

## Components

1. **MCP transport:** Streamable HTTP, JSON-RPC/MCP semantics implemented by an established SDK (Rust `rmcp`). TLS can terminate at a reverse proxy; the origin still validates authentication.
2. **Identity validator:** Fresh HTTPS Keycloak introspection at HTTP admission and each tool dispatch. Require matching principals and validate active Bearer type, issuer, audience, expiry/time claims and required scope. Token cryptographic validation is delegated to Keycloak; the Rust core does not use local JWT/JWKS verification.
3. **Authorisation:** configured exact-subject allowlists for named host profiles. Default deny.
4. **Session registry:** opaque random IDs bound to principal, host profile and SSH identity; idle and absolute deadlines; bounded concurrency; in-memory state acceptable for v1. Restart revokes all sessions.
5. **SSH supervisor:** launch system OpenSSH using argv, not a local shell. Strict host-key verification; connection multiplexing using unique per-session control sockets in private directories. SSH keys live on the gateway.
6. **Command runner:** each `exec` launches a separate remote shell command over the authorised SSH connection. Run commands in target-side systemd user services with cgroup deadlines; bound output and concurrency. Use independent service-manager evidence for exit status and termination; missing evidence remains unknown.
7. **Audit writer:** commit execution intent and later outcome to SQLite (WAL + `synchronous=FULL`) and provide a restricted query interface. Off-host delivery is a separate hardening concern.

## Session and command lifecycle

```text
HTTP request -> validate token -> authorise tool and host
    |
    +-- hosts: return visible host profiles
    |
    +-- open_session:
    |      generate session ID, allocate private control socket,
    |      verify/pin host key, connect as configured SSH user,
    |      register principal + host + deadlines, return session ID
    |
    +-- exec:
    |      revalidate principal and host/session grant
    |      check expiry + revocation + resource bounds
    |      create command ID and COMMIT audit "accepted"
    |      if audit fails: reject, DO NOT SPAWN SSH
    |      COMMIT audit "running"; reject if this write fails
    |      launch supervised remote command without local shell interpolation
    |      capture bounded output, status, timestamps
    |      finalise audit with observed outcome or unknown
    |      return observed outcome + command ID
    |
    +-- close_session:
           revoke session ID and clean up owned multiplexing resources
```

A lost HTTP response does **not** mean the command failed, and retrying an arbitrary command is generally unsafe. V1 records every invocation with a unique command ID and exposes audit lookup. **No transparent execution retries.** Future idempotency keys require a separate documented protocol.

## Meaning of "persistent session"

- The gateway can reuse a persistent authenticated SSH **transport** with OpenSSH `ControlMaster`.
- Each command may use a new remote shell. `cd /tmp` in one invocation does not change the next invocation's working directory.
- Clients can run `cd /path && command` or scripts as a single command. This is an explicit API contract, not an implementation accident.
- No exposed raw control socket or general interactive PTY in v1. PTYs can obscure auditing and introduce input/terminal escape complexity.
- Target-side deadlines continue after SSH loss or gateway restart. Confirmed termination requires service-manager evidence and an empty command cgroup; missing evidence is unknown with termination unconfirmed. Supervision cannot undo completed writes or cancel external jobs, and the target account/service manager remain trusted. Historical timeout records without termination evidence do not prove a remote kill.

## Shutdown and crash handling requirements

- Reject new requests during graceful shutdown; allow bounded draining of active commands, then record cancelled or unknown outcomes as warranted.
- When SSH transport disappears, distinguish its failure from the remote command's exit code.
- On service restart, in-memory sessions are invalidated; orphan sockets are removed from private run directories. Pre-crash accepted/running commands without conclusively recorded completion become `unknown`.
- Recoverability and forensic completeness are separate: a process death can create uncertainty about whether a remote side effect occurred.

## Implemented technology choices

- **Rust** with official MCP Rust SDK, Tokio/Axum, Keycloak HTTPS introspection and a SQLite driver.
- **System OpenSSH** for transport, authentication, host-key verification, explicit private-key identities and multiplexing; ambient agent/configuration is disabled.
- **Keycloak** for user authentication, **SQLite** for local audit, **systemd** for service supervision.
- Tailscale/private routing when appropriate; no public exposure of SSH ports is required.

Simplicity outranks a fashionable language choice. If runtime behaviour or security testing supports a simpler stack, revisit this through an ADR.
