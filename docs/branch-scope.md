# Branch scope validation: feat/hosts-mcp

Compared with `main`, this branch implements the first read-only development slice. The broader requirements describe a future SSH gateway and are not acceptance claims for this branch.

| Area | Implementation and validation |
| --- | --- |
| Transport | Rust/Axum Streamable HTTP MCP on fixed `127.0.0.1:8765/mcp`; stateless responses. Docker integration inspects the actual listening socket and HTTP session headers. |
| Tool surface | Only `hosts`, returning IDs/descriptions. Integration asserts the complete tool list and rejects calls to the planned execution/session/audit tools. |
| Authentication | HTTPS Keycloak introspection on each HTTP request, plus a fresh check inside `hosts`; exact issuer/audience/scope and temporal/access-token checks; no redirects or ambient proxy routing. Real-Keycloak tests complement deterministic mock and unit tests. |
| Authorisation | Exact subject allowlists, empty grants deny access, sorted inventory. Real users exercise private/shared/invisible hosts and alternating requests; exact response comparisons detect grant leakage. |
| Configuration | Explicit inventory and mandatory issuer/client settings; bounded schema, unknown fields rejected, no SSH connection data. Unit tests cover inventory validation; Docker tests cover startup failures. |

The review found two test blind spots and corrected them: the old smoke test checked for a session header in the JSON body rather than the HTTP response, and the description-validation unit test omitted mandatory `allowed_subjects`, allowing failure for the wrong reason. The smoke suite now also asserts the exact introspection count on a successful tool call and rejection when the second introspection sees a revoked token.

Docker integration adds an actual disposable Keycloak issuer without changing application behavior or exposing the development endpoint. Run `sh scripts/test_docker.sh`; CI runs the same entry point. See [development instructions](development.md) for fixtures, coverage and cleanup.

Verified on 2026-10-29 using Docker Desktop on macOS/ARM64: formatting, Clippy with warnings denied, all 21 unit tests, the expanded mock HTTPS smoke suite and all real-Keycloak integration scenarios passed. Automatic teardown removed the test containers, network and both fixture volumes. CI is configured to run the same suite on Ubuntu; that CI run has not been observed as part of this local verification.

Still outside this branch's implemented scope: SSH execution, persistent execution sessions, durable auditing, OAuth resource discovery and client login, public TLS/listener hardening, deployment packaging and verification with the user's ChatGPT account. Consequently neither the Phase 0 ChatGPT gate nor the Phase 1 SSH milestone is satisfied. These tests do not justify public deployment.

The follow-up hardening adds a dependency lockfile, locked builds, Rust 1.93 as the declared/tested minimum, pinned CI actions/container images, scheduled advisory scans, and enforced 100-line limits for handwritten code/configuration. Origin/Host checks, bounded work, same-principal checks and private response headers are covered by regression tests; see [implemented security controls](hardening.md). A local `cargo audit --deny warnings` with cargo-audit 0.22.2 passed against 1,386 advisories and 188 locked dependency entries. This checks known Rust advisories, not container vulnerabilities or undiscovered defects.

All test clients, TLS mocks, realm/certificate fixture generation and file-length checks are now native Rust Cargo integration tests. The Python harness was removed. Normal Cargo runs execute the mock and source-size tests; the two Docker-specific targets are ignored until the Compose wrapper explicitly runs fixture generation and the genuine Keycloak test.
