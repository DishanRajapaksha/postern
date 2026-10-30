# Gont public SSH gateway

Postern is deployed under `/srv/services/postern/` as an unprivileged native systemd service. Source stays under `/srv/source/postern/<commit>/`. Release binaries live under `releases/<commit>/postern`; `current` selects the active binary. Private settings and real grants remain on Gont. Keep the reproducible deployment files and verification record in the infrastructure repository's `deployed/postern/` too.

The public resource is `https://leyline.talesfrom.earth/postern`, using the existing Leyline gateway. Its metadata URL is `https://leyline.talesfrom.earth/.well-known/oauth-protected-resource/postern`. Caddy rewrites the MCP path to `/mcp` and connects to `/run/postern/http.sock`, a read-only bind mount of `/srv/services/postern/proxy`. Postern owns a mode-0600 socket. TCP stays on `127.0.0.1:8765` and must not be publicly forwarded.

Use `environment.example` and `execution.example.json` as templates. Set the required audience to the exact public resource URL; issuing clients also need the `postern-resource` audience for Keycloak introspection. The systemd manager reads the root-only environment before dropping privileges. Configuration and credentials are root-controlled and group-readable only where the service requires them. The service owns its mode-0700 `audit`, `run`, and `proxy` directories, the only persistent write locations in its sandbox.

Create `postern-ssh` as a separate non-root target account with a home directory and ordinary shell, no sudo grants and a unique authorized key restricted against forwarding/PTY/agent use. Generate the gateway key on Gont; keep it in the service credentials directory with mode 0600. Pin the host's public key by reading `/etc/ssh/ssh_host_ed25519_key.pub` through the trusted admin connection. Do not use unauthenticated network scanning to establish trust. The target account must not be in the `postern` service group or be able to access audit data.

The host ID remains `gont`; its address, account, port, credentials and host-key path are entirely operator-configurable. A later switch to an administrator requires a separate credential/grant review and an execution-profile edit plus restart. Neither MCP commands nor tool arguments can select an arbitrary account or destination.

Keycloak remains the existing Heimdall `gont` realm. The dedicated FastMCP OAuthProxy adapter is deployed from the infrastructure repository's `deployed/postern/oauth/`; it uses `postern-login`, a confidential client with mandatory S256 and the fixed callback `https://leyline.talesfrom.earth/postern/auth/callback`. Keep the existing passkey flow and audience validation. MCP clients use automatic registration with blank client ID/secret. The old manual `postern-chatgpt` client stays disabled. The native introspection client remains separate.

Remote execution requires cgroup v2 and systemd 250+. For the configured non-root account, run `loginctl enable-linger postern-ssh` and `systemctl start user@$(id -u postern-ssh).service` as the host administrator. This grants no sudo rights to the target account. Commands run in transient user services with independent deadlines and cgroup-wide stop/escalation. Keep lingering enabled for every configured target account; there is no unsupervised fallback. See [execution semantics](../../docs/execution.md).

Run formatting, Clippy, all Cargo tests and `sh scripts/test_docker.sh` before release. Compare binary hashes across build and installation. The full suite publishes no host ports and tests both genuine Keycloak and OpenSSH.

Before replacing shared gateway configuration, compare live and tracked hashes and preserve any live-only routes. Validate Caddy configuration before applying it. A new Unix-socket bind mount requires recreating the Caddy container with its existing private runtime override, then verifying all existing routes. Never print runtime environment/credential files.

Acceptance checks:

1. Only loopback TCP plus the private Unix socket listens; LAN/Tailscale probes cannot connect to port 8765.
2. Public metadata is JSON 200, advertises the exact resource and issuer, and has no-store/nosniff headers. Anonymous/invalid MCP calls return 401 with the path-specific challenge; foreign Origin/Host and oversized bodies are rejected.
3. A real issuer token sees exactly the five tools. Authorised test SSH commands return the non-root account identity, a durable command record and bounded output. Other principals cannot use the session or query its audit data. Revocation blocks the next operation.
4. Automatic registration, CSRF consent, S256 and owner passkey login complete, followed by real tool calls. User confirmed this path on 2026-10-30. Repeat after relevant OAuth changes.
5. Timeout a delayed marker, a TERM-ignoring child and a detached process group; confirm no marker appears after the original delay and that audit reports confirmed termination. Transport loss must report unknown/unconfirmed while the remote timer continues.
6. Check Heimdall discovery, existing Leyline/Pensieve MCP and Trilium health after changes. Record source/binary/config hashes and any pending gates without secrets.

Recovery: stop `postern.service`, restore the previously recorded private environment/config and unit, repoint `current` to the retained previous release, reload systemd and start it. Remove only the Postern gateway routes/mount if withdrawing public access, keeping all existing routes. Retain audit data; restarting invalidates sessions and marks unfinished command records unknown. Private rollback copies must remain root-only on Gont.
