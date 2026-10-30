#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
# A unique project prevents accidental reuse of imported realms or test state.
project="postern-test-$(date +%s)-$$"
compose() { docker compose -p "$project" -f tests/integration/compose.yml "$@"; }
cleanup() {
    result=$?
    trap - EXIT
    if [ "$result" -ne 0 ]; then compose logs --no-color keycloak; fi
    compose down --volumes --remove-orphans
    exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
compose build
compose up -d keycloak
# Dependencies already ran; never regenerate TLS fixtures under the running issuer.
compose run --rm --no-deps integration sh -ec '
    cargo fmt --all -- --check
    cargo clippy --locked --all-targets -- -D warnings
    cargo test --locked --all-targets
    cargo test --locked --test keycloak -- --ignored --nocapture
    cargo test --locked --test ssh -- --ignored --nocapture
'
