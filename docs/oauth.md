# OAuth discovery and Keycloak login (PR #2)

**Status: development preview, not production ChatGPT connectivity.** This feature only publishes the metadata that tells an MCP client where to authenticate. Login, authorisation-code issuance, PKCE checks, client registration and refresh tokens are handled by **Keycloak**, never Postern.

## What is implemented

- `GET /.well-known/oauth-protected-resource` and `GET /.well-known/oauth-protected-resource/mcp` return identical [RFC 9728](https://www.rfc-editor.org/rfc/rfc9728.html) JSON documents.
- `resource` identifies the externally intended HTTPS `/mcp` endpoint, `authorization_servers` contains the pinned Keycloak realm, `scopes_supported` advertises the required scope, and `bearer_methods_supported` is `["header"]`.
- Unauthorized MCP requests receive HTTP 401 with `WWW-Authenticate: Bearer resource_metadata="<resource-metadata-url>", scope="<scope>"`. Keycloak outages remain 503; invalid tokens never fall back to anonymous calls.
- Public metadata does not weaken the existing authentication middleware: **every request to `/mcp` still requires fresh introspection**, and `hosts()` still checks the principal a second time.
- Metadata requests are still covered by Postern's Host/Origin validation, 16 KiB request cap, 15-second HTTP deadline, concurrency cap and no-store response headers.

## Configuration

The existing `POSTERN_KEYCLOAK_ISSUER`, `POSTERN_KEYCLOAK_SCOPE` and introspection variables remain mandatory. PR #2 additionally **requires**:

```sh
POSTERN_OAUTH_RESOURCE=https://postern.example.org/mcp
```

Use the *exact eventual externally visible* HTTPS resource URL, including any explicit port. The parser rejects HTTP, missing hosts, credentials in URLs, query strings, fragments, trailing slashes and noncanonical URL syntax. No OAuth location is inferred from untrusted Host, Origin or forwarding headers. Invalid or absent configuration aborts startup. **Do not publish credentials or private infrastructure addresses in Git.**

The computed discovery URL is `https://postern.example.org/.well-known/oauth-protected-resource` and the advertised issuer comes from the already configured `POSTERN_KEYCLOAK_ISSUER`. The server is still **hard-wired to loopback**; this configuration alone does not make it publicly accessible or ready for ChatGPT.

## Keycloak setup needed before real ChatGPT sign-in

1. Create/configure a Keycloak realm that supports OAuth authorization code with **PKCE S256** and publishes standard OAuth/OIDC discovery metadata.
2. Use a **pre-registered OAuth client** for ChatGPT, or configure a supported dynamic-client-registration / client-ID-metadata approach. Whether Keycloak supports the chosen mechanism securely must be verified. Do not enable unrestricted anonymous client registration just to make an integration work.
3. Register the **exact redirect URI presented in the ChatGPT connection flow**. Do not guess callback URLs, or allow arbitrary redirect URI wildcards.
4. Issue **access tokens**, not ID or refresh tokens, with the expected issuer, `aud` containing Postern's configured resource audience and any Keycloak-required introspection-client audience, `scope` containing the configured Postern scope and a stable `sub`.
5. Ensure the client passes/forwards the OAuth `resource` parameter where required, and that Keycloak issues audience-bound tokens. Reject tokens minted for other resource servers.
6. Validate genuine login, token refresh, logout/revocation, wrong audience and cross-user host separation using the final public HTTPS MCP endpoint.

**Never** implement Keycloak's authorization/token endpoints inside Postern or accept a user password through an MCP tool. The current introspection client remains confidential and separate from the ChatGPT OAuth client.

## Local verification

`cargo test --locked --all-targets` exercises URL validation, discovery payloads, authentication challenges and real loopback HTTP against a temporary TLS issuer. `sh scripts/test_docker.sh` exercises a disposable genuine Keycloak realm. The unit test uses an illustrative public resource URL; it does *not* test actual browser sign-in.

## Release gate still open

This PR does **not** add a public listener, a TLS termination configuration, proxy header trust, client registration, public OAuth login, ChatGPT connection verification, SSH sessions or command auditing. These require separate review, request/header limits at the edge, rate limiting and an actual authenticated ChatGPT round-trip before the service is exposed to production.

References: [MCP authorisation discovery](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization) · [OpenAI plugin authentication](https://developers.openai.com/plugins/build/auth) · [RFC 9728](https://www.rfc-editor.org/rfc/rfc9728.html)
