# MCP tool contract (design draft)

Status: proposed tool interface; **no server or tools implemented yet**. Field names can evolve through protocol/SDK tests.

All tools require authenticated HTTPS requests. The server resolves actor identity from the verified token; it never trusts a user- or model-provided identity. Authorisation is checked on **every** tool call.

## `hosts()`

Returns only targets available to the authenticated principal.

Response fields per target: `id`, `description`, optional capability metadata. **Never** return secret keys, SSH options or internal connection files. Host names and addresses need not be exposed if an opaque ID suffices.

## `open_session({ host_id })`

Input: configured opaque host ID. No raw hostnames, usernames, SSH port, key path or SSH flags.

Returns: `session_id`, `host_id`, `created_at`, `idle_expires_at`, `absolute_expires_at`.

The session is bound to the authenticated principal, effective host policy and configured SSH identity. Opening can fail because of missing rights, unavailable SSH transport, invalid pinned host key, authentication failure or capacity limits.

## `exec({ session_id, command, timeout_seconds? })`

- `command`: arbitrary UTF-8 remote shell command, passed as data into the SSH invocation. Postern does **not** filter or interpret content.
- `timeout_seconds`: bounded resource control; default and maximum are deployment configuration. Timeout cannot promise to kill all remote descendants.
- Returns `command_id`, `outcome`, `exit_code` (nullable), bounded `stdout`, `stderr`, truncation flags, timestamps and duration.
- `outcome` must distinguish `completed` (remote command exited; exit status may be non-zero), `transport_error`, `timed_out`, `cancelled`, and `unknown` (remote execution result cannot be confirmed).
- Missing/expired session, unauthorised actor or failed pre-execution audit commit: **no command dispatch**.

Examples of allowable command strings are ordinary shell expressions such as `pwd && id`, `cd /srv/app && git status`, or a script invocation. They are not analysed for danger.

For a client-visible response, an execution result is distinct from a delivery acknowledgement: a network timeout can hide a successful remote command. Do not automatically repeat a command because its result was lost.

## `close_session({ session_id })`

Authorises ownership, revokes the token and best-effort closes owned SSH resources. Concurrent commands need a documented cancellation/drain policy; session closure blocks subsequent commands immediately.

Returns `closed` or `already_closed` without exposing other actors' session existence. Avoid leaking identifiers via error distinctions.

## `audit({ session_id?, command_id?, limit?, cursor? })`

List/read the actor's allowed audit events in reverse chronological order, with pagination. A separate administrative policy can authorise cross-user investigation. Do not treat an arbitrary `actor_id` supplied by the caller as proof of permission.

Fields include `command_id`, `session_id`, `actor_id`, `host_id`, `command`, `accepted_at`, `started_at`, `finished_at`, `outcome`, `exit_code`, `error_class` and optional output/truncation metadata.

The audit query must distinguish a command accepted but never known to have started from one known to be running or finished. The audit record may contain secrets in command arguments. Query and export access must be explicitly restricted.

## Protocol and operating rules

- No raw SSH access details are accepted from clients; configured profiles only.
- All tool arguments have defined byte length, type and count bounds, including session IDs and audit pagination.
- Unauthorised access, missing sessions and invalid resource claims must not leak key material or host information.
- A command's success/failure is not inferred from the SSH tool call merely returning HTTP 200.
- MCP tools should return structured data for predictable automation and human-readable summaries when useful.
- User interfaces such as ChatGPT may independently insert confirmations, deny certain actions or disable custom MCP write tools. Postern cannot override them.
- No promise of infinite output capture or indefinitely held interactive sessions.
