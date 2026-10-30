# Engineering rules

Keep handwritten source, tests, scripts and configuration files at **100 lines or fewer**, including comments and blank lines. Split by responsibility: token parsing, issuer communication, claim validation and HTTP admission are separate concerns. Do not compress statements to evade the limit. Markdown documentation and generated `Cargo.lock` are excluded; Cargo requires one complete lockfile.

Run `cargo test --locked --test source_size` to enforce the limit. CI and the Docker test runner enforce it too. Application code, test fixtures, HTTP clients, TLS mocks and file-length checks are Rust; the shell wrapper only orchestrates Docker Compose.

Use Rust 1.93 or newer and build with `--locked`. Commit dependency changes with their lockfile update. Format with rustfmt and resolve Clippy warnings. Project code forbids unsafe Rust, and production code denies `unwrap`/`expect`; test assertions may use them.

Explain each module's job in a short module comment. Comment security decisions and the reason for a limit or rejection. Prefer named types, small functions, ordinary control flow and narrow visibility over clever abstractions. Describe trust boundaries in plain language.

Never log access tokens, refresh tokens, client secrets, authorization headers or raw issuer errors. Never derive identity from tool arguments or a previous MCP call. Authentication failures and upstream outages must deny access. Keep the public host response type separate from private grants.

Run the complete local suite with `sh scripts/test_docker.sh`. It needs Docker/Compose and publishes no host ports. For dependencies, install `cargo-audit` 0.22.2 with `--locked`, then run `cargo audit --deny warnings`. CI is configured to audit branch/PR changes; the daily schedule takes effect when the workflow is on the default branch.

Passing tests and an advisory scan establish specific evidence; they do not certify production readiness. Preserve inventory-only defaults; public routing and SSH require explicit operator configuration, non-root acceptance and deployment verification. Privileged accounts and claimed ChatGPT compatibility require their own acceptance.
