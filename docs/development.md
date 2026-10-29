# Development preview: first feature

The current first feature is a **read-only host inventory** with a real Rust MCP Streamable HTTP server and an explicit configuration file.

This is deliberately **not yet an SSH gateway**. There is no OAuth authentication, no session opening, no command execution and no audit implementation. The server binds to `127.0.0.1:8765` only and must **never** be exposed through a tunnel or reverse proxy. Do not use production host details or keys in this preview.

## Build and run

Requires a recent stable Rust toolchain supporting edition 2024. The exact minimum supported compiler version has not been established yet.

```sh
cp config/postern.example.json config/postern.local.json
POSTERN_CONFIG=config/postern.local.json cargo run
```

The only supported MCP endpoint is `http://127.0.0.1:8765/mcp`. It implements MCP Streamable HTTP using the official `rmcp` crate.

To test the protocol, use an MCP client or Inspector **on the same machine**, and call `hosts()`. It returns JSON text with the configured IDs and descriptions:

```json
{"hosts":[{"id":"lab","description":"Disposable local test machine (illustrative only)"}]}
```

The example is an inventory label, not an SSH hostname. No actual connection is attempted.

## Configuration

Environment variable `POSTERN_CONFIG` is required. The JSON must be an object containing a `hosts` array; unknown fields are refused.

Each host has only `id` and `description`. The ID must start with lowercase ASCII `a-z`, followed by at most 63 characters from `a-z`, `0-9`, `-` and `_`. Descriptions must be nonempty, without leading/trailing whitespace or control characters and at most 256 Unicode characters. Limits: 64 hosts and a 64 KiB config file. Duplicate IDs are refused, results sorted by ID.

The v1 configuration intentionally has **no host address, user, port or SSH key fields**. These will be introduced only when authenticated host authorisation and strict SSH host-key verification are implemented.

## Tests

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

GitHub Actions runs those checks on the feature branch, plus a real loopback HTTP protocol smoke test. After `cargo build`, run `python3 scripts/smoke_mcp.py` to check `initialize`, `tools/list` and `hosts()` against a locally launched Postern process. The check is intentionally local and makes no SSH connection. The first real ChatGPT round-trip still requires a separately deployed, **authenticated** MCP endpoint and must not be claimed from this preview.

## Next feature

Keycloak/OIDC request validation and principal-scoped host policy, before SSH session or command execution. Until then this remains local development machinery.
