# Keycloak-authenticated MCP development preview

This feature adds a real Rust MCP Streamable HTTP server with a **read-only `hosts()` tool** and **per-request Keycloak access-token introspection**.

**No SSH connections, command execution, audit pipeline, public listener or OAuth discovery/login flow are implemented.** The server binds to `127.0.0.1:8765` and must not be exposed through a proxy or tunnel. A local MCP client must supply an access token obtained independently from Keycloak. This does not yet establish ChatGPT app compatibility.

## Required configuration

Configure a trusted Keycloak realm and an appropriate **confidential introspection client** (a client authorised to introspect access tokens) in Keycloak. Configure a dedicated audience and required scope for Postern, including protocol mappers/client scopes so that the relevant access tokens actually carry:

- `iss`: the exact configured Keycloak realm issuer URL.
- `aud`: `postern` or another configured audience.
- `sub`: the stable Keycloak user identifier.
- `scope`: includes `postern:use` as a separate word.
- `exp`: valid future expiry. Optional `nbf`/`iat` cannot be in the future.

Postern only accepts tokens when Keycloak introspection returns `active: true`. It rejects missing claims, wrong audiences, issuers or scopes, expired/revoked tokens, and all malformed introspection responses. **No token introspection cache or fallback is permitted.** If Keycloak is unavailable, requests fail closed with HTTP 503. Missing/invalid credentials receive HTTP 401. Token revocation therefore takes effect on the next request, subject to Keycloak's reported active state and availability.

Environment variables:

| Variable | Purpose |
| --- | --- |
| `POSTERN_CONFIG` | Path to local JSON host inventory with subject allowlists |
| `POSTERN_KEYCLOAK_ISSUER` | Exact HTTPS realm URL, e.g. `https://sso.example.org/realms/postern` |
| `POSTERN_KEYCLOAK_AUDIENCE` | Required audience, e.g. `postern` |
| `POSTERN_KEYCLOAK_SCOPE` | Required scope, e.g. `postern:use` |
| `POSTERN_KEYCLOAK_CLIENT_ID` | Confidential client for Keycloak introspection |
| `POSTERN_KEYCLOAK_CLIENT_SECRET` | Confidential client's secret (never commit this) |
| `POSTERN_KEYCLOAK_CA_FILE` | Optional trusted PEM CA certificate for an internal Keycloak instance |

The configured issuer must be an HTTPS URL ending in `/realms/<realm>` without userinfo, query or fragment. Postern derives `<issuer>/protocol/openid-connect/token/introspect` and forbids HTTP redirects. Startup fails if any mandatory variable is missing or invalid. Keycloak credentials are sent **only** to the configured HTTPS introspection endpoint, using Basic authentication and strict TLS certificate verification.

## Host grants

`config/postern.example.json` contains a label-only inventory, with explicit `allowed_subjects` arrays:

```json
{
  "hosts": [
    {
      "id": "lab",
      "description": "Disposable test host (illustrative only)",
      "allowed_subjects": ["00000000-0000-0000-0000-000000000001"]
    }
  ]
}
```

The server **defaults to deny**: no matching `sub` means an empty host list. Grants use exact string matching and are checked inside `hosts()` after **a second token introspection**, independently of the HTTP gate. The MCP tool returns only `id` and `description`. The `allowed_subjects` list is never returned to a client.

The inventory schema does not accept SSH addresses, usernames, password fields, keys or unknown fields. Host IDs are limited to lowercase ASCII identifiers. Grant arrays are bounded, cannot contain empty/control-character identifiers and reject duplicates.

## Running locally

Install a recent Rust toolchain with edition 2024 support and configure Keycloak as above. Store secrets in a permission-controlled environment or service-credential mechanism rather than in this public repository.

```sh
cp config/postern.example.json config/postern.local.json
# Export the required Keycloak environment variables securely.
POSTERN_CONFIG=config/postern.local.json cargo run
```

Endpoint: `http://127.0.0.1:8765/mcp` **on the same machine only**. The localhost MCP transport is not TLS because it must never leave the machine; the Keycloak introspection connection uses verified HTTPS.

Every HTTP request (including initialise, discovery, tools and unsupported GET/DELETE operations) must have `Authorization: Bearer <access-token>`. Requests without a valid, currently active token are denied before MCP processing. The MCP transport operates **statelessly**: no `Mcp-Session-Id` can be used to inherit an earlier caller's authorisation. This is separate from the principal-bound SSH execution sessions planned later.

## Tests

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --quiet
python3 scripts/smoke_mcp.py
```

The smoke test creates a temporary HTTPS **mock Keycloak introspection server** with an isolated self-signed certificate. It verifies unauthenticated access is denied, active credentials are accepted, expiry/issuer/audience/scope failures are denied, Keycloak outages fail closed, different subject IDs see different hosts, no grant lists are leaked and token checks occur on every request. It uses no real Keycloak account, SSH server or secrets.

**Before external deployment:** test against an actual configured Keycloak realm and the user's actual ChatGPT MCP integration, including the OAuth discovery/authorisation flow; implement TLS/public endpoint hardening and bind any future SSH sessions to verified principals. Unit and mock tests alone do not establish production security.

## Next work

OAuth resource metadata and compatible ChatGPT login flow; connection testing against Keycloak; eventual SSH profile/session support with strict host-key verification and durable audit-before-execute. No SSH privilege should be enabled until this is verified.
