# Audit design

Current implementation and verification are described in [branch scope](branch-scope.md) and [execution](execution.md). Design proposals below are requirements, not evidence that every acceptance gate has passed.

Status: proposed. **Postern must account for every MCP command request it accepts**, while recognising that an arbitrary remote shell may perform more operations than any one request reveals.

## Two different meanings of "every command"

1. **MCP execution audit (required):** record each `exec` invocation, command text and attributable result. A command may include a script or shell pipeline. This is the guarantee Postern aims to provide.
2. **Host activity audit (optional):** Linux audit subsystem/process accounting/terminal recording can give additional evidence about executed programs and interactive activity. No single `execve` log captures shell built-ins, interpreter internals or every possible effect. Host instrumentation is a separate integration, not automatically supplied by MCP.

Calling `bash -c '...'` or running a script is still one auditable MCP request; its internal statements are not individually observed by the gateway.

## Record model (proposed)

| Field | Meaning |
| --- | --- |
| `event_id` | Unique, immutable audit event ID. |
| `command_id` | Unique execution identifier common to all state transitions. |
| `session_id` | The owning session; bind to authenticated identity, never caller-supplied identity. |
| `actor_id` | Keycloak issuer+subject or another validated stable principal. |
| `host_id` / `ssh_identity_id` | Approved target profile and configured credential identity, not key content. |
| `command` / `command_sha256` | Exact requested command bytes/text and integrity digest; may contain secrets. |
| `accepted_at`, `started_at`, `finished_at` | UTC timestamps for observed transitions. |
| `state` / `outcome` | Accepted, running, completed, transport_error, timed_out, cancelled or unknown. |
| `exit_code` | Remote command status when actually observed; nullable. |
| `error_class` | Bounded non-secret classification for failures. |
| `stdout_bytes`, `stderr_bytes`, `truncated` | Output size and capture state. |
| `output_reference` | Optional restricted storage reference, if output retention is enabled. |
| `trace_id` | Optional request correlation; must not expose OAuth tokens. |

Use an append-only event stream or immutable transitions plus a materialised current-state table. A row that is merely updated in place is operationally useful but weak evidence against a malicious local administrator. Stronger tamper resistance requires off-host append-only forwarding, signatures or storage-level immutability.

## Required write sequence

1. Validate token, authorise session and check constraints.
2. Allocate `command_id`; write and durably commit `accepted` with exact command, actor and target.
3. **Only after a successful commit**, dispatch the SSH command.
4. Best-effort record `running` and eventually `completed`, `transport_error`, `timed_out`, `cancelled` or `unknown`.
5. If the post-dispatch audit store fails, report degraded state, stop accepting *new* executions and preserve forensic uncertainty. Do **not** claim the remote command failed just because logging failed.
6. On restart, reconcile incomplete records as indeterminate unless there is reliable independent evidence of their completion.

SQLite with WAL and `synchronous=FULL` is an initial candidate; validate actual durability guarantees on the chosen filesystem, hardware and transaction boundaries. Disk-full, read-only filesystem and power-loss tests are release gates.

## Failure matrix

| Failure | Expected result |
| --- | --- |
| Audit intent cannot commit | Reject request; no SSH subprocess dispatched. |
| Authz/token validation fails | Reject; optionally record a minimal security event without exposing credentials. |
| SSH could not start after audit accepted | Record transport failure and its reason class. |
| SSH exits non-zero with observed exit code | Record `completed` with non-zero exit code; command ran and failed. |
| MCP HTTP response lost | Audit retains command ID and outcome when observable; client must not blindly retry. |
| Service crashes after dispatch | Mark unresolved attempt `unknown` on restart, not successful or failed. |
| Output exceeds capture limit | Keep draining/discarding excess bytes to avoid deadlock; mark truncated. |
| Session revoked while executing | Block new executions; apply documented policy to in-flight commands and log result/uncertainty. |
| Off-host audit sink unavailable | Local durable queue retains events; expose replication lag and decide operational fail-closed threshold. |

## Privacy and integrity

- Commands can include passwords or API keys; they must be treated as **sensitive audit data**.
- Default proposal: store the exact command and execution metadata, but **do not persist stdout/stderr bodies by default**. Keep byte counts, truncation flags and optionally digests. Output is still returned to the authorised current caller and may reveal secrets there.
- Restrict who can query/export audit. Apply retention, secure deletion/backups and log access monitoring.
- Separate audit writer storage/permissions from the remote SSH user's control. For credible tamper-evidence, replicate off-host to storage the SSH identity cannot change.
- Any log redaction weakens exact replay capability; if added, mark explicitly which original fields were retained and which were transformed.
- Avoid logging request headers, bearer tokens, SSH private keys, agent sockets or complete environment dumps.

## Verification plan

The test suite must prove pre-dispatch persistence using a forced audit-write failure, then kill the process at the accept/dispatch/result boundaries and inspect recovery. Include concurrent commands, very large output, binary-like output, repeated client requests, invalid session IDs and attempted unauthorised audit reads.
