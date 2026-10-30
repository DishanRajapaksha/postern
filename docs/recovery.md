# Audit backup, recovery and incident procedures

These procedures accompany the recovery/storage regressions in `tests/ssh/`. They describe operator actions; they have not been exercised against the deployed Gont service by this change. Keep private settings, keys and command records outside Git.

## Create a consistent snapshot

Build the maintenance tool with `cargo build --locked --release --bin postern-audit`. It is a separate operator binary and is not an MCP tool. Run it as an account able to read the audit database and its WAL, with a destination directory accessible only to that operator (0700).

```sh
/path/to/postern-audit backup /absolute/audit/audit.sqlite3 /absolute/private-backups/unique-snapshot.sqlite3
```

The source may remain online. The tool uses SQLite's online backup API, includes committed WAL contents, verifies database integrity and the audit table, and publishes a mode-0600 standalone snapshot without overwriting an existing destination. Copying only a live `audit.sqlite3` file with `cp` can omit committed WAL events. The copy loop checks a 30-second deadline between SQLite steps and fails on a persistently busy source. This is not a wall-clock limit for the complete command: individual SQLite operations, integrity verification and filesystem synchronization may take longer.

A snapshot contains sensitive exact command text. Store it on encrypted storage with access separate from the SSH target identity. Check backup exit status and freshness; alert on failures and on a missing daily snapshot. A local copy is not protection against host loss or a malicious administrator. Off-host storage, encryption/key ownership and acceptable replication lag remain deployment decisions.

Retain audit events until an explicit retention policy is approved; this tool never purges them. A suggested starting policy is daily snapshots with 30 days of backup history, plus incident holds. Capacity, legal/privacy needs and off-host copies must be reviewed before automating expiry. Never remove the last verified recovery point.

## Restore into a fresh directory

1. Disable new Postern traffic and stop `postern.service`. Retain the existing audit directory and private configuration as root-only recovery material. Do not replace a database beneath a running writer.
2. Select a verified snapshot and inspect its timestamp. Any later events will be absent: record that gap and preserve the original store for reconciliation.
3. Create a new mode-0700 audit directory owned by the service account. Use `postern-audit backup SNAPSHOT NEW-DIRECTORY/audit.sqlite3` with absolute paths to recheck and copy the snapshot. Leave the destination name unused beforehand. Do not copy old `-wal`, `-shm` or lock files into it.
4. Point only the private execution configuration's `audit_dir` at the restored directory. Preserve its ownership and permissions. Keep runtime and credentials in their existing private locations.
5. Start the service and verify health, real authentication and actor-scoped audit retrieval. Startup marks accepted/running records unknown; old session handles must fail. The runtime lock excludes concurrent gateways and startup reclaims abandoned SSH masters/directories.
6. Run an authorised non-root `exit 23` smoke test and verify its durable record. Re-enable traffic only after acceptance. Retain the original store for investigation; do not merge SQLite event streams manually.

An unknown recovered record remains unknown even if a client previously saw successful output. Recovery cannot invent a missing durable final result. Remote systemd deadlines continue independently after gateway failure; restarting the gateway does not reverse completed effects or cancel external jobs.

## Storage failure response

- A failed pre-dispatch audit append blocks execution. A failed final append preserves observed output with `audit_degraded: true`, disables new executions and may recover as unknown later.
- Stop new traffic, preserve diagnostics without tokens/commands in general logs, and repair capacity or mount permissions. Removing the cause alone does not clear the degraded latch; restart after repair.
- Verify SQLite integrity using a new snapshot and exercise a non-root smoke test. Keep incident evidence and record any final events that were not durable.
- Docker regressions exhaust a private 1 MiB tmpfs, reject execution without a marker, reject startup on a read-only filesystem, and verify repair/restart. They also inject a post-dispatch final-write failure. Physical power loss remains untested.

## Rotate SSH credentials and host keys

1. Provision a distinct replacement private key outside Git with mode 0600; keep the same reviewed non-root target profile and forwarding/PTY restrictions.
2. Add its public key to the target through a trusted administrator connection. Verify pinned-host authentication and the intended account permissions before switching the gateway profile.
3. Pause new traffic, allow dispatched work to finish within its independent deadline, update the private `identity_file`, and restart Postern to invalidate old sessions and transports.
4. Repeat real principal, non-root execution, audit and isolation checks. Remove the old authorized key only after the replacement passes; then verify old-key authentication fails. Retain recovery credentials only under the incident/rotation policy.
5. For a host-key change, obtain the replacement public key through a separately trusted administrator channel. Update only the reviewed pin and repeat acceptance. An unexplained mismatch is an incident, not permission to disable strict checking or use a first network scan as trust.

## Rotate identity credentials and revoke access

Rotate the native introspection secret and public OAuthProxy upstream/signing/encryption credentials independently using their owning service's supported procedure. Keep values in private credential files; do not print environment files or place secrets in shell arguments. Signing/encryption changes can invalidate sessions or stored tokens; preserve an authorised rollback and repeat registration, consent, S256, owner login, token refresh and caller-isolation acceptance.

Remove a host grant or revoke a Keycloak token to block the next operation. Confirm denial using the real issuer. Existing dispatched commands drain under their independent deadlines; grant revocation is not a promise to reverse or immediately cancel them. Remove a compromised SSH key at the target as well, and inspect active target services through the trusted administrator channel.

## Incident and rollback

Stop new access first; preserve audit snapshots, service/configuration hashes and timestamps. Investigate the gateway and target independently, since MCP auditing does not capture every host action. Use a trusted administration path to contain target-side activity when necessary. Avoid replaying side-effecting commands to determine whether they ran.

Restore a retained known-good binary and private configuration only after checking its execution semantics. Releases before the systemd deadline fix killed local SSH without proving remote termination; they are not an acceptable timeout-safety rollback. Follow the Gont runbook for gateway routes and verify existing Leyline, Heimdall, Pensieve and Trilium health after any shared-service change. Reopen access only after relevant acceptance and record unresolved outcomes honestly.
