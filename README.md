# Postern

**An auditable SSH gateway for MCP clients.**

Postern is a proposed small, self-hosted server that lets authorised MCP clients, including ChatGPT where custom tool capabilities permit, run arbitrary shell commands on explicitly configured SSH hosts. It is designed around **session-level authorisation, unrestricted commands within the SSH account's privileges, and mandatory audit records**.

> **Status: early development.** PR #1 implements a loopback-only MCP host-inventory server with Keycloak token introspection on **every** HTTP request and subject-scoped host visibility. There is no SSH execution, persistent SSH session, OAuth client discovery, public deployment or audit pipeline. This is not production-ready.

## Principles

1. **Authorise access, not shell syntax.** An authenticated principal may open a session only to an approved host and SSH identity. Once authorised, Postern does not parse, classify, rewrite or selectively approve commands.
2. **Audit before execution.** No execution attempt proceeds unless a durable audit record is created. Record completion, failure or unknown outcome afterwards. Do not claim to observe every subprocess, shell built-in or side effect without host-level auditing.
3. **Use OpenSSH.** Reuse the system SSH binary, strict server host-key pinning and connection multiplexing. Avoid implementing a new SSH protocol.
4. **Keep credentials outside ChatGPT.** SSH private keys stay with the gateway. The MCP client receives only opaque host IDs and short-lived session identifiers.
5. **Stay small and inspectable.** Prefer one service, an explicit host allowlist, a simple HTTP MCP endpoint, standard identity verification and a local durable event store.
6. **Be honest about the boundary.** A shell that can execute arbitrary commands can also destroy data, exfiltrate secrets and bypass application-layer command accounting. Limit its OS permissions and network reach accordingly.

## Planned flow

```text
ChatGPT / MCP client
       |
       | HTTPS + OAuth access token
       v
Postern (MCP over Streamable HTTP)
       |-- validate identity, token audience and authorisation
       |-- approve a configured host + SSH identity for a bounded session
       |-- persist command audit records before dispatch
       |-- execute through system OpenSSH with multiplexing
       |-- finalise audit with exit status and outcome
       |
       +---- SSH ----> allowlisted host, dedicated OS account
```

The proposed default authentication provider is **Keycloak** and the preferred implementation is **Rust** using the official MCP Rust SDK, Tokio/Axum and system OpenSSH. These are design preferences, not implementation claims or immutable requirements.

## Proposed MCP tools

| Tool | Intent |
| --- | --- |
| `hosts` | List SSH targets visible to the authenticated principal. |
| `open_session` | Open a time-limited SSH access session against one configured target. |
| `exec` | Execute arbitrary remote shell text in a session; return a command ID, stdout/stderr, exit status and outcome. |
| `close_session` | Terminate or revoke a session and release owned resources. |
| `audit` | Inspect permitted audit events, including incomplete and unknown outcomes. |

A **persistent SSH connection is not a persistent interactive shell**. Each `exec` in the initial design starts a fresh remote shell command. Environment changes such as `cd` do not automatically carry between calls. Arbitrary scripts, pipelines and compound commands remain possible.

## Development preview

The first feature, a read-only `hosts()` tool and loopback-only Rust Streamable HTTP MCP server, is under development on `feat/hosts-mcp`. It verifies every HTTP request through Keycloak's HTTPS token-introspection endpoint and filters hosts by the validated `sub`. See [local development instructions](docs/development.md). This is still a **loopback-only preview**; public OAuth discovery and ChatGPT compatibility are not implemented and SSH commands are unavailable. Never expose the development server through a proxy or tunnel.

## Documentation

- [Requirements and acceptance criteria](docs/requirements.md)
- [Architecture and lifecycle](docs/architecture.md)
- [MCP tool contract](docs/mcp-api.md)
- [Security model and threat analysis](docs/security.md)
- [Implemented security controls](docs/hardening.md)
- [Engineering rules and 100-line source limit](CONTRIBUTING.md)
- [Audit model](docs/auditing.md)
- [Deployment and configuration design](docs/operations.md)
- [Implementation and verification plan](docs/roadmap.md)
- [Current branch scope and Docker verification](docs/branch-scope.md)
- [ADR 0001: session-authorised, unrestricted execution](docs/adr/0001-session-authorised-execution.md)

## Project boundaries

The gateway itself will not implement a command policy language, a terminal emulator, shell syntax analysis, per-command approval prompts, a general-purpose fleet manager or a bespoke SSH implementation. External MCP clients, including ChatGPT, may independently require confirmations or restrict tool execution. Server-side authorisation cannot override client-side safety controls.

No licence has yet been selected. Do not assume permission to redistribute or reuse the future code until a licence is added.

## Security warning

Postern is a **remote code execution gateway by design**. Never expose an unauthenticated endpoint or give its SSH identity more permissions than intended. In particular, do not test early builds against a production root account. See [security requirements](docs/security.md) before deployment.
