# ADR 0001: Session-authorised unrestricted SSH execution

- **Status:** Proposed
- **Date:** 2026-10-29
- **Context:** Postern is intended to give ChatGPT or another trusted MCP client remote SSH access with mandatory auditing but no per-command approval engine.

## Decision

Authorise a specific authenticated user to open a short-lived session for a server-configured SSH host and identity. Within that grant, accept arbitrary shell commands and execute them with system OpenSSH. Do not parse or restrict commands at the MCP gateway. Persist the command request before dispatch and record observed outcomes afterwards.

An SSH session in v1 means a **persistent authenticated transport**, not a long-lived shell process/PTY. Every `exec` invocation is a distinct audited command request.

## Rationale

- Policy enforcement on shell syntax is fragile and easily bypassed with interpreters, encodings, scripts, redirections and nested command execution.
- Mature OpenSSH is preferable to hand-rolled SSH transport.
- Host and SSH account boundaries offer understandable OS-enforced permissions.
- A durable per-request audit is easier to reason about than attempting to reconstruct opaque terminal interactions.
- Minimal components and clear failure modes facilitate independent security review.

## Consequences

### Positive

- Straightforward API and implementation.
- Powerful remote administration and development without command-by-command policy.
- Clear actor/session/host/command audit attribution.
- Easy to run against ordinary OpenSSH servers.

### Negative / risks

- A compromised client token or prompt-injected agent can perform any actions allowed by the remote account.
- Commands may launch child programs, create detached processes or alter state outside the lifetime of one MCP request.
- Command logs alone cannot prove all activity on a host.
- Each `exec` has fresh shell state; clients must specify working directory/environment explicitly.
- Some MCP clients, including ChatGPT, may impose confirmations or tool restrictions regardless of Postern policy.
- For credible tamper evidence, audit data needs retention and off-host protections beyond a local SQLite file.

## Rejected alternatives

1. **Command denylist / regex approvals:** porous and high maintenance; contradicts the requirement.
2. **LLM safety classifier on every command:** nondeterministic, bypassable and adds complexity to a security boundary.
3. **Bespoke SSH transport:** duplicates mature cryptographic and operational code.
4. **Always-open interactive terminal for v1:** makes detailed invocation accounting, output boundaries and recovery substantially harder.
5. **Production root account as default:** catastrophic blast radius for an agent-exposed shell.

## Review trigger

Revisit if the product needs interactive terminals, higher assurance host activity recording, multiple isolated tenants or broader distribution. Any change that bypasses audit-before-execute requires a new explicit security review.
