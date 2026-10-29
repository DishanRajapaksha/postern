# Gont deployment packaging

These files prepare the existing loopback-only preview for a native systemd service. They do not establish that a deployment has occurred. Verify the host, systemd compatibility, available port, binary architecture and Keycloak settings before installation.

Use `/srv/services/postern/` for runtime files. Keep releases under `releases/<commit>/postern` and point `current` at the accepted release. A root-owned `private/environment` file supplies the settings shown in `environment.example`; keep its real secret on Gont only. The systemd manager reads the file before dropping privileges, so the service account does not need permission to read it directly.

Create a dedicated system account named `postern`, without a login shell, home directory or SSH keys. Make releases and configuration root-owned; give the service group read/traverse access to the inventory. The process needs no writable persistent directory for this feature. Install the unit in `/etc/systemd/system/postern.service`, verify it with `systemd-analyze verify`, then enable/start it after configuring the issuer and grants.

The shipped inventory is empty and denies all visibility. Populate actual grants only from independently verified Keycloak subjects. Configure a dedicated confidential introspection client, audience and `postern:use` scope. Preserve the existing realm's passkey flow and disabled password grants. Do not reuse Leyline's client secret or alter shared routing to test this preview.

## Acceptance checks

1. Build the exact accepted Git commit for Gont's architecture; compare SHA-256 hashes before and after transfer.
2. Run the native Rust/Docker integration suite on Gont in isolation from the deployed service. The fixed test port is safe inside its Docker network namespace.
3. Check the systemd account, sandbox, restart state and socket: only `127.0.0.1:8765` may listen. Verify no public/LAN/tunnel route was created.
4. From Gont, check missing/invalid credentials return 401, foreign Origin/Host return 403 and oversized bodies return 413. Check private response headers.
5. Use short-lived diagnostic principals in the actual issuer to verify MCP initialisation, the exact single-tool inventory, private/shared/default-denied host visibility and immediate revocation. Keep tokens in memory and clean up diagnostic clients/grants afterwards. Never weaken the owner's passkey policy for automation.
6. Recheck the final owner inventory and existing shared services. Record the deployed source commit, binary hash, unit/config hashes, checks, limitations and recovery commands without secrets.

Gont deployment changes also belong in the infrastructure repository under `deployed/postern/`, with its service table, deployment status and non-secret verification record updated and committed. Keep source-code checkouts separate from active runtime state.

## Recovery

For an initial failed installation, stop/disable `postern.service` and retain its private settings and release for inspection. For a failed upgrade, stop the service, repoint `current` to the previously recorded release and start it, then repeat the socket/authentication checks. Do not expose or tunnel this branch to make it remotely usable: public OAuth discovery/login and deployment hardening remain separate work.
