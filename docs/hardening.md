# Implemented security controls

Current implementation and verification are described in [branch scope](branch-scope.md) and [execution](execution.md). Design proposals below are requirements, not evidence that every acceptance gate has passed.

This document describes the inventory foundation; additional public and SSH controls are documented in execution.md, not a security certification or production deployment approval.

| Boundary | Enforced behavior |
| --- | --- |
| HTTP destination | Fixed IPv4 loopback listener. Exactly one Host header must name `127.0.0.1:8765` or `localhost:8765`. |
| Browser origin | Origin may be absent. If present, exactly one Origin must equal `http://` plus that request's allowed Host; otherwise HTTP 403. |
| Work budget | At most 32 admitted requests, 16 KiB of body per request and a 15-second deadline covering body ingestion and handler response creation. Excess work receives 503, oversized bodies 413, deadlines 408. |
| Issuer transport | Configured HTTPS endpoint, TLS 1.2 minimum with certificate/hostname checks, five-second request timeout, 16 KiB response cap, no redirects or ambient HTTP proxy. |
| Issuer load | At most 32 simultaneous introspections, with immediate rejection when full. This is a concurrency cap, not a requests-per-second rate limiter. |
| Identity | Fresh active Bearer token validation at HTTP admission and tool execution. Both checks must return the same principal. Missing/wrong claims, malformed responses and issuer failure deny access. |
| Host visibility | Exact subject allowlists; empty grants deny access. Public responses contain only IDs/descriptions and never serialize the internal grant type. |
| Configuration | Inventory and custom CA files are limited during reads, not after reading an unlimited file. Unknown inventory fields, invalid grants and non-regular files fail startup. |
| Secret handling | No raw issuer error logging. Credential buffers owned by settings are wiped on drop; environment and HTTP-library copies have separate lifetimes and are not covered by that guarantee. |
| Response handling | Successes and errors carry `Cache-Control: no-store` and `X-Content-Type-Options: nosniff`. HTTP 401 includes a Bearer authentication challenge. |
| Build inputs | Committed dependency lockfile and locked builds; Docker image digests and CI action commits are pinned. RustSec checks are configured for changes and a daily default-branch schedule. |
| Rust implementation | Unsafe code forbidden in this crate; production `unwrap`/`expect` denied by Clippy. Modules, tests and handwritten scripts/configs stay within 100 lines. |

The [MCP transport security requirements](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2025-11-25/basic/transports.mdx) motivate the Origin/loopback checks. [RustSec cargo-audit](https://github.com/rustsec/rustsec/tree/main/cargo-audit) checks locked dependencies against known advisories; it cannot prove dependencies free of undiscovered flaws. Pinning inputs makes updates explicit and still requires ongoing patch review.

## Verification and remaining deployment requirements

Native Rust regression tests cover malformed credentials, changed subjects between introspections, revocation during dispatch, concurrent user isolation, genuine Keycloak logout and expiry, foreign origins/hosts, oversized fixed/chunked bodies, overload, timeout cancellation and capacity recovery. Cargo runs the actual binary against a rustls HTTPS mock, plus a file-length test. Docker supplies the genuine HTTPS Keycloak realm for an additional Cargo integration test; unit tests use a controlled clock for deadlines. There is no Python test harness.

The HTTP limits begin after Hyper has parsed request headers. They do not constitute an operating-system connection limit, slow-header defense or edge rate limiter. Before any public deployment, add reviewed connection/header budgets, per-principal rate limits, public TLS/OAuth discovery, durable security/audit events, secret provisioning and rotation, monitoring/alerts, deployment isolation and incident/recovery procedures. Independently review the implementation and test the actual client integration. SSH execution additionally requires all audit-before-execute and host-key/session controls in the project requirements.
