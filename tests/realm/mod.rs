//! Fixed public test identities; no external account or production secret is used.
pub mod boundary;
pub mod discovery;
pub mod environment;
pub mod failures;
pub mod isolation;
pub mod issuer;
pub mod lifecycle;
pub mod scenarios;
pub const ALICE: &str = "00000000-0000-0000-0000-000000000001";
pub const SECRET: &str = "disposable-fixture-secret";
pub const ISSUER: &str = "https://localhost:8443/realms/postern-test";
