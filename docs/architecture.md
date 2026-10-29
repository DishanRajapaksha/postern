# Architecture and execution lifecycle

Status: **proposed**. Precise implementation interfaces remain open.

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
          Postern (origin validates JWT)
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

1. **MCP transport:** Streamable HTTP, JSON-RPC/MCP semantics implemented by an established SDK (Rust `rmcp` preferred). TLS can terminate at a reverse proxy; the origin still validates authentication.
2. **Identity validator:** OIDC JWT verification against Keycloak's discovery/JWKS configuration. Check issuer, audience, signature, expiry and any required scope/role for each request.
3. **Authorisation:** static/configured mapping from principal or validated group/role to named host profiles and permitted MCP operations. Default deny.
4. **Session registry:** opaque random IDs bound to principal, host profile and SSH identity; idle and absolute deadlines; bounded concurrency; in-memory state acceptable for v1. Restart revokes all sessions.
5. **SSH supervisor:** launch system OpenSSH using argv, not a local shell. Strict host-key verification; connection multiplexing using unique per-session control sockets in private directories. SSH keys live on the gateway.
6. **Command runner:** each `exec` launches a separate remote shell command over the authorised SSH connection. Bound execution time, output bytes and concurrency; record actual exit status separately from transport errors.
7. **Audit writer:** commit execution intent and later outcome to SQLite (WAL + synchronous durability settings proposed) and provide a restricted query interface. Off-host delivery is a separate hardening concern.

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
    |      launch SSH remote command without local shell interpolation
    |      update audit to "running" if possible
    |      capture bounded output, status, timestamps
    |      finalise audit (completed/failed/timed_out/cancelled/unknown)
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
- Long-running or detached programs can outlive an MCP request. Timeouts do not guarantee termination of every remote descendant. A command audit record describes the observed SSH invocation, not the lifetime of every spawned child.

## Shutdown and crash handling

- Reject new requests during graceful shutdown; allow bounded draining of active commands, then record cancelled or unknown outcomes as warranted.
- When SSH transport disappears, distinguish its failure from the remote command's exit code.
- On service restart, in-memory sessions are invalidated; orphan sockets are removed from private run directories. Pre-crash accepted/running commands without conclusively recorded completion become `unknown`.
- Recoverability and forensic completeness are separate: a process death can create uncertainty about whether a remote side effect occurred.

## Technology choices (proposed)

- **Rust** with official MCP Rust SDK, Tokio/Axum, mature JOSE/OIDC validation library and SQLite driver.
- **System OpenSSH** for transport, authentication, host-key verification, SSH agent/identity integration and multiplexing.
- **Keycloak** for user authentication, **SQLite** for local audit, **systemd** for service supervision.
- Tailscale/private routing when appropriate; no public exposure of SSH ports is required.

Simplicity outranks a fashionable language choice. If runtime behaviour or security testing supports a simpler stack, revisit this through an ADR.
