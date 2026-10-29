# Deployment and configuration design

Status: **proposed**; examples are illustrative and **not working configuration** until a schema is implemented.

## Expected deployment

- A small Linux service, preferably a Rust single binary managed by **systemd**.
- Streamable HTTP MCP behind HTTPS termination. For a remote ChatGPT client, the MCP endpoint must be reachable through a supported integration; private SSH hosts can remain accessible only from the gateway over Tailscale/private networking.
- Keycloak as the OIDC issuer; validate issued access tokens at the Postern origin. Do not assume Cloudflare headers or a reverse proxy identity prove authentication.
- A dedicated `postern` Unix user with restricted home/config/audit/runtime directories. No root service or unnecessary ambient capabilities.
- Pre-provisioned SSH key identities and server host-key pins, outside the repository.

## Illustrative configuration

```yaml
# Design sketch only. Not executable or validated.
http:
  listen: "127.0.0.1:8765"
  public_mcp_url: "https://YOUR-GATEWAY.example/mcp"
oidc:
  issuer: "https://YOUR-KEYCLOAK.example/realms/YOUR-REALM"
  audience: "postern"
  required_scope: "postern:use"
ssh:
  known_hosts_file: "/etc/postern/ssh_known_hosts"
  identities:
    operator:
      private_key_file: "/etc/postern/keys/operator"
  hosts:
    lab:
      address: "lab.internal"
      port: 22
      username: "postern-operator"
      identity: "operator"
policy:
  # Identity/group mappings, syntax to be finalised.
  # Never allow arbitrary destination details in MCP arguments.
  default: deny
sessions:
  idle_timeout: "15m"      # illustrative
  absolute_timeout: "1h"   # illustrative
  max_per_principal: 3     # illustrative
execution:
  default_timeout: "60s"   # illustrative
  max_timeout: "10m"       # illustrative
  max_output_bytes: 1048576 # illustrative, combined output
audit:
  sqlite_path: "/var/lib/postern/audit.sqlite3"
  persist_output_bodies: false
```

No private key material, access token, password, server fingerprint or live host address should be committed to a public repository. The YAML intentionally uses placeholders and should not be copied into production unreviewed.

## Bootstrap outline

1. Confirm the target ChatGPT integration really supports connecting to the custom remote MCP endpoint and calling state-changing tools. Do not mistake successful discovery for execution capability.
2. Create a dedicated Keycloak client/resource audience and configure a valid OAuth flow for the intended MCP client. Test access-token issuance and exact issuer/audience/scope validation.
3. Establish private connectivity from the gateway to test SSH host(s). Install pinned host keys through an independent trusted channel.
4. Create dedicated remote SSH user(s) and provision public-key access. Disable password auth for this integration and avoid root initially.
5. Store SSH credentials and audit data in permission-restricted directories, distinct from application code and HTTP static content.
6. Configure TLS and controlled public routing for the MCP endpoint. Add rate limiting and origin verification.
7. Run against disposable machines; verify all authorisation, audit-failure and crash-recovery tests before production host profiles.
8. Document key rotation, host-key replacement, credential revocation, operator recovery and incident response.

## Host security

SSH access is as powerful as its remote account. The host controls `sudo`, filesystem and network permissions. If Postern operates on a machine that also contains its own audit database, ensure the SSH identity cannot reach or modify that store. Prefer a physically/logically separate gateway and remote target where practical.

## systemd hardening (design intent)

- `User=postern`, `Group=postern`, no root/capability grants.
- Private runtime and state directories for control sockets and SQLite, correct ownership and mode.
- Restrict filesystem reads/writes as much as compatible with OpenSSH identity and host-key access.
- Consider `NoNewPrivileges`, `ProtectSystem`, `PrivateTmp`, device/network restrictions and system call filtering **after testing actual OpenSSH dependencies**.
- Scrub secrets from logs and crash dumps; use service restart limits and health/ready probes.

## Observability

Expose non-sensitive counters (requests, authorisation denials, active sessions, command outcomes, audit lag), readiness and liveness. Never expose command strings, stdout/stderr or token claims to a public metrics endpoint.

## Day-two procedures

Document commands to invalidate every session, disable a host profile, revoke a Keycloak grant, rotate a key, replace a pinned host key with out-of-band verification, check audit integrity, restore from backup and identify unresolved executions after restart.

**Do not claim that a timeout, SIGTERM or connection close will kill every remote child process.**
