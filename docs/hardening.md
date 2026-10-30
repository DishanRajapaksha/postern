# Implemented security controls

Status as of **2026-10-30**: this document describes implemented HTTP, identity and build controls. Public gateway access and audited SSH are merged; see [current scope](branch-scope.md), [execution](execution.md) and the [roadmap](roadmap.md) for their controls, recorded evidence and open gates. These controls do not certify production readiness.

| Boundary | Enforced behavior |
| --- | --- |
| HTTP destination | Fixed IPv4 loopback TCP listener and optional private Unix socket. Exactly one Host must name `127.0.0.1:8765`, `localhost:8765` or the explicitly enabled public HTTPS origin's authority. |
| Browser origin | Origin may be absent. If present, exactly one Origin must match the allowed local HTTP origin or configured public HTTPS origin for that Host; otherwise HTTP 403. |
| Work budget | At most 32 admitted requests, 16 KiB bodies and a 15-second body-read deadline. Overall handler deadline: 15 seconds in inventory mode, 75 seconds with execution. Full capacity returns 503; body size/read failure returns 413; the overall deadline returns 408. |
| Public rate budget | Explicit public mode adds a shared 30 requests/second budget with burst 120; exhausted capacity returns 429. It is not a per-principal or edge connection limit. |
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

The HTTP limits begin after Hyper has parsed request headers. They do not constitute an operating-system connection limit, slow-header defense or edge rate limiter. Public TLS/OAuth, durable execution auditing, isolated deployment and non-root SSH acceptance are recorded for Gont in the [deployment evidence](../deploy/gont/verification.json). Reviewed connection/header budgets, per-principal limits, monitoring/alerts, credential rotation and recovery drills remain operational work. Native remote deadline tests passed. The [connector report](connector-regression.md) confirms the timeout fix for the reproduced command; connector child-process/cancellation-race coverage and comparison with the [local native benchmark](local-verification.md) remain open. An independent security review and the remaining requirements must be completed before claiming broader production readiness.
