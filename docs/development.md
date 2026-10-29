# Keycloak-authenticated MCP development preview

This feature adds a real Rust MCP Streamable HTTP server with a **read-only `hosts()` tool** and **per-request Keycloak access-token introspection**.

**No SSH connections, command execution, audit pipeline, public listener or verified ChatGPT OAuth login flow are implemented.** Resource discovery metadata is implemented in PR #2, but actual browser sign-in remains a separate acceptance gate. The server binds to `127.0.0.1:8765` and must not be exposed through a proxy or tunnel. A local MCP client must supply an access token obtained independently from Keycloak. The OAuth discovery metadata enables a compatible client to locate the issuer but does not establish ChatGPT app compatibility.

## Required configuration

Configure a trusted Keycloak realm and an appropriate **confidential introspection client** (a client authorised to introspect access tokens) in Keycloak. Configure a dedicated audience and required scope for Postern, including protocol mappers/client scopes so that the relevant access tokens actually carry:

- `iss`: the exact configured Keycloak realm issuer URL.
- `aud`: `postern` or another configured audience.
- `sub`: the stable Keycloak user identifier.
- `scope`: includes `postern:use` as a separate word.
- `exp`: valid future expiry. Optional `nbf`/`iat` cannot be in the future.

On Keycloak 26.6.2 and newer, tokens must also include the **introspection client's ID** in their audience. For example, with required audience `postern` and introspection client `postern-resource`, configure **two separate audience mappers** on the issuing client: one custom audience `postern` and one client audience `postern-resource`, both enabled for access tokens and introspection. Keep Keycloak's audience validation enabled. See the [Keycloak migration guidance](https://www.keycloak.org/docs/26.8.1/upgrading/#migrating-to-26-6-2).

Postern only accepts tokens when Keycloak introspection returns `active: true`. It rejects missing claims, wrong audiences, issuers or scopes, expired/revoked tokens, and all malformed introspection responses. **No token introspection cache or fallback is permitted.** If Keycloak is unavailable, requests fail closed with HTTP 503. Missing/invalid credentials receive HTTP 401. Token revocation therefore takes effect on the next request, subject to Keycloak's reported active state and availability.

Environment variables:

| Variable | Purpose |
| --- | --- |
| `POSTERN_CONFIG` | Path to local JSON host inventory with subject allowlists |
| `POSTERN_OAUTH_RESOURCE` | Canonical intended public HTTPS MCP resource URL (e.g. `https://postern.example.org/mcp`); required for discovery |
| `POSTERN_KEYCLOAK_ISSUER` | Exact HTTPS realm URL, e.g. `https://sso.example.org/realms/postern` |
| `POSTERN_KEYCLOAK_AUDIENCE` | Required audience, e.g. `postern` |
| `POSTERN_KEYCLOAK_SCOPE` | Required scope, e.g. `postern:use` |
| `POSTERN_KEYCLOAK_CLIENT_ID` | Confidential client for Keycloak introspection |
| `POSTERN_KEYCLOAK_CLIENT_SECRET` | Confidential client's secret (never commit this) |
| `POSTERN_KEYCLOAK_CA_FILE` | Optional trusted PEM CA certificate for an internal Keycloak instance |

The configured issuer must be an HTTPS URL ending in `/realms/<realm>` without userinfo, query or fragment. Postern derives `<issuer>/protocol/openid-connect/token/introspect` and forbids HTTP redirects. Startup fails if any mandatory variable is missing or invalid. Keycloak credentials are sent **only** to the configured HTTPS introspection endpoint, using Basic authentication and strict TLS certificate verification.

See [OAuth discovery setup](oauth.md) for the anonymous metadata endpoints, precise 401 challenge, Keycloak client registration and why this still does not authorise public deployment.

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

Install Rust 1.93 or newer and configure Keycloak as above. Store secrets in a permission-controlled environment or service-credential mechanism rather than in this public repository. See [engineering rules](../CONTRIBUTING.md) and [implemented hardening controls](hardening.md).

```sh
cp config/postern.example.json config/postern.local.json
# Export the required Keycloak environment variables securely.
POSTERN_CONFIG=config/postern.local.json cargo run --locked
```

Endpoint: `http://127.0.0.1:8765/mcp` **on the same machine only**. The localhost MCP transport is not TLS because it must never leave the machine; the Keycloak introspection connection uses verified HTTPS.

Every HTTP request (including initialise, discovery, tools and unsupported GET/DELETE operations) must have `Authorization: Bearer <access-token>`. Requests without a valid, currently active token are denied before MCP processing. The MCP transport operates **statelessly**: no `Mcp-Session-Id` can be used to inherit an earlier caller's authorisation. This is separate from the principal-bound SSH execution sessions planned later.

## Tests

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

The native Rust integration test in `tests/mcp.rs` starts the actual binary and a temporary HTTPS **mock Keycloak introspection server**, backed by rustls and a generated CA/server certificate pair. It verifies unauthenticated access is denied, active credentials are accepted, expiry/issuer/audience/scope failures are denied, Keycloak outages fail closed, different subject IDs see different hosts, no grant lists are leaked and token checks occur on every request. It uses no real Keycloak account, SSH server or secrets. OpenSSL is required for local certificate generation. Cargo also runs the 100-line file-limit test. Docker-only fixture/real-realm tests are explicitly ignored in normal Cargo runs.

### Docker integration with real Keycloak

With Docker Engine/Desktop running and Docker Compose available:

```sh
sh scripts/test_docker.sh
```

No local Rust or OpenSSL installation is required. The runner builds the branch with Rust 1.93 and runs formatting, Clippy, unit tests and native Rust integration tests, then exercises a disposable **Keycloak 26.8.1** realm over verified HTTPS. First use downloads the images and Rust dependencies. There is no Python harness or Python runtime requirement.

All handwritten code/configuration files stay within 100 lines; generated Cargo.lock and Markdown documentation are excluded. HTTP clients must use an allowed local Host and, when sending Origin, the matching local origin. Requests are limited to 16 KiB and 15 seconds, with 32 admitted requests at a time. See [hardening](hardening.md) for exact behavior and limits.

The suite generates a temporary CA and signed server certificate, imports three test users and confidential clients with explicit subject/audience mappers, and tests real access tokens through MCP initialisation, tool discovery and calls. It checks exact host grants and response fields, empty visibility for an ungranted user, alternating and concurrent callers, rejection of duplicate credentials, fabricated session handles, refresh tokens, missing scope, wrong audience, expired access tokens and access tokens after Keycloak logout. It also verifies the actual loopback bind, absence of MCP session response headers, absence of SSH/audit tools, failure on invalid startup configuration, and HTTP 503 for untrusted TLS, bad introspection credentials and an unreachable issuer. Unusable ambient proxy settings exercise direct issuer connections. The mock suite additionally checks malformed/oversized responses, refusal to follow redirects, revocation between the HTTP gate and tool dispatch and exactly two introspections for a successful `hosts` call.

Compose publishes **no host ports**. The runner shares Keycloak's disposable network namespace so that Postern can retain its fixed loopback bind; there is no forwarding proxy or public listener. The wrapper uses a unique project and removes that project's containers, network and fixture volume on success or failure. Fixture passwords and client secrets are public test data, and generated private keys stay in the disposable volume. Password grants are enabled only in this test realm to obtain tokens without an interactive browser.

The setup uses Keycloak's documented [realm import](https://www.keycloak.org/server/containers#_importing_a_realm_on_startup) and [PEM TLS configuration](https://www.keycloak.org/server/enabletls). It validates this development slice against a local real issuer; the public OAuth/ChatGPT acceptance gate below remains outstanding.

Compose runs `cargo test --locked --test fixtures -- --ignored` in its init container to generate disposable realm/certificate files. The test runner then runs `cargo test --locked --all-targets` followed by `cargo test --locked --test keycloak -- --ignored`. Keep fixture generation inside Docker; it writes to the mounted `/fixtures` and `/realms` volumes.

The fixture includes a separate token-issuing client and verifies its token through the configured introspection client. The Compose runner does not restart dependencies during test execution, preventing certificate regeneration under a running issuer on older Compose versions.

**Before external deployment:** test against an actual configured Keycloak realm and the user's actual ChatGPT MCP integration, including the OAuth discovery/authorisation flow; implement TLS/public endpoint hardening and bind any future SSH sessions to verified principals. Unit and mock tests alone do not establish production security.

## Next work

A compatible ChatGPT login flow and real callback/PKCE acceptance test; eventual SSH profile/session support with strict host-key verification and durable audit-before-execute. No SSH privilege should be enabled until this is verified.
