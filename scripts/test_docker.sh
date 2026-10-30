#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
# A unique project prevents accidental reuse of imported realms or test state.
project="postern-test-$(date +%s)-$$"
compose() { docker compose -p "$project" -f tests/integration/compose.yml "$@"; }
cleanup() {
    result=$?
    trap - EXIT
    if [ "$result" -ne 0 ]; then compose logs --no-color keycloak integration; fi
    compose down --volumes --remove-orphans
    exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
compose build
compose up -d keycloak
# Dependencies already ran; never regenerate TLS fixtures under the running issuer.
compose up -d integration
compose exec -T integration sh -ec '
    for attempt in $(seq 1 100); do
        if systemctl show-environment >/dev/null 2>&1; then break; fi
        sleep 0.1
    done
    systemctl start user@0.service
    cargo fmt --all -- --check
    cargo clippy --locked --all-targets -- -D warnings
    cargo test --locked --all-targets
    cargo test --locked --test keycloak -- --ignored --nocapture
    cargo test --locked --test ssh -- --ignored --nocapture
'
