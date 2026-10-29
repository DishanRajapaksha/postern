# Security model and threat analysis

Status: **proposed security requirements**. Do not deploy as trusted infrastructure before implementation, adversarial testing and review.

## Security position

Postern is a remote code execution gateway by design. The user explicitly wants **no server-side command restrictions after session authorisation**. Security therefore rests on authenticating the caller, limiting which SSH accounts/hosts it can reach, isolating credentials, constraining session lifetime, observing usage and reducing blast radius at the operating-system/network layer.

**Do not treat a prompt, approval-looking text, a header supplied by a reverse proxy without verification, or a tool argument as an identity.**

## Authentication and authorisation

- Support Keycloak/OIDC with standards-based token verification at the **origin**: validate cryptographic signature and trusted algorithm, issuer, expected audience, `exp`/`nbf`, and required scope/role. Retrieve signing keys via configured JWKS and handle rotation safely.
- Authentication must be verified on every MCP request, including tool discovery, execution and audit reads. Never allow unauthenticated state-changing calls.
- Use stable token principal identifiers, preferably issuer + subject, not display names. Avoid using email addresses as sole identity keys.
- Keep the OAuth flow compatible with the chosen MCP client; verify actual ChatGPT capabilities rather than assuming an integration works on any plan.
- Policy maps principal/validated groups to explicit target IDs and operations. Server-side checks apply at `open_session` **and** `exec`, `close_session`, `audit`. Default deny when policy is missing or ambiguous.
- Session IDs are strong random opaque identifiers, bound to their owner, host profile, identity and deadlines. Never accept session IDs from a different principal.
- Restrict access to the MCP endpoint with TLS, request-size bounds, rate limiting and optionally additional network restrictions. A reverse proxy or Cloudflare Tunnel is not a substitute for checking OAuth at Postern itself.
- If token verification, Keycloak metadata/JWKS or authorisation state is unavailable, fail closed for new privileged requests rather than accepting unverified identities.

## SSH and credentials

- Use **system OpenSSH**, pinned `known_hosts` files and `StrictHostKeyChecking=yes`. No silent trust-on-first-use, no `UserKnownHostsFile=/dev/null`.
- Prefer an individually provisioned, dedicated remote SSH account with the least OS permissions compatible with the intended tasks. Avoid direct root login; if sudo is necessary, scope it explicitly at the OS layer and understand the resulting access.
- Keep SSH keys on the gateway using a secure credential store, tightly permissioned file or agent. Do not store them in Git, MCP configuration responses or model-visible prompts.
- Do not accept client-chosen `ProxyCommand`, arbitrary SSH options, usernames, key paths, destinations or environment variables that alter SSH behaviour. Configuration is operator-controlled.
- When invoking the SSH process, build argv directly and never invoke a **local** shell to concatenate parameters. Remote shell execution is intentional; local shell injection is not.
- Place SSH multiplexing control sockets in private, owner-only runtime directories. Prevent session collisions and unlinking/attaching to another user's socket.
- Harden the gateway systemd user and filesystem access, but allow only the explicit key, control-socket and audit paths it requires.
- Ideally route SSH to private or Tailscale IPs; do not expose SSH to the public Internet solely to make the MCP gateway function.

## Threats and countermeasures

| Threat | Mitigation or accepted residual risk |
| --- | --- |
| Stolen bearer token | Short-lived tokens, TLS, restricted audience, scoped target access, revocation/expiry strategy and monitoring. Bearer theft within token lifetime remains dangerous. |
| Prompt injection | Arbitrary execution is an intended capability, so malicious tool-use suggestions can induce real side effects. Use dedicated remote accounts, constrained host allowlists, backups and ChatGPT's independent tool controls. |
| Host-key substitution / MITM | Pre-pin trusted host keys, require strict verification and reject key changes until manually approved. |
| Forged host/session | Resolve target only from server config; bind session ID to actor/host/identity; check every tool call. |
| Local SSH option/shell injection | No caller-supplied SSH flags; no local `sh -c`; argv construction with explicit bounds. |
| Cross-user data leak | Separate ownership of sockets, sessions, records and audit queries; test ID guessing and concurrent use. |
| Gateway compromise | Dedicated OS user, restricted filesystem, non-root service, private routing, key storage, deployment updates and monitoring. |
| Command hides follow-on actions | **Accepted limitation** of application-level auditing. Add host-side exec/process/terminal instrumentation if complete behavioural reconstruction is required. |
| Secret disclosure in logs | Restricted audit access, encryption-at-rest/backups where appropriate, configurable output collection, retention and never log OAuth/SSH credentials. |
| Local audit destruction | Keep gateway audit store separate from remote SSH target; forward immutable or access-controlled copies off-host for stronger tamper resistance. |
| Timeout/cancel | Best-effort process termination does not guarantee remote children stop. Preserve honest outcome/uncertainty. |
| Resource exhaustion | Limits on request sizes, session count, command runtime, output capture, simultaneous processes and audit queries. |

## Unrestricted means unrestricted

The gateway will not block commands based on shell keywords, regexes, danger ratings or an LLM judgement. A permitted command can run `sudo`, invoke other interpreters, change files or contact the network **to the extent the remote OS account allows it**.

Consequences:
- One trusted session can still destroy data if its SSH account can.
- Audit records of requested command strings **cannot prove** what an arbitrary script or interpreter did inside the target machine.
- A malicious instruction embedded in repository text, a web page or command output can influence an agent to use its execution privileges.
- No cryptographic token or language choice eliminates these risks.

## Security tests required before production

- OIDC malformed/expired/wrong-issuer/wrong-audience/incorrect-scope cases.
- Key rotation, JWKS outage, session ownership and revoked grants.
- Fake SSH servers and changed host keys.
- Host/path/SSH option injection, oversized arguments and malformed unicode.
- Cross-user audit access and socket isolation; concurrency and stale IDs.
- SQLite/audit filesystem failures before and after command dispatch.
- Prompt-injected instructions from untrusted command output, with test-only SSH hosts.
- Forced process kill/network loss between each audit and dispatch transition.
- Dependency review, fuzzing of exposed parsers, and secret-scrubbing review.

**Deployment rule:** no production SSH targets or privileged accounts until the acceptance and adversarial tests pass.
