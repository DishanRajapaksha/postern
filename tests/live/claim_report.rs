//! Print boolean validation outcomes, never the underlying identity or token.
use super::state::ISSUER;
use anyhow::Result;
use serde_json::Value;

pub fn print(claims: &Value) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    for (name, value) in [
        ("active", claims["active"] == true),
        ("bearer", claims["token_type"] == "Bearer"),
        ("issuer_matches", claims["iss"] == ISSUER),
        (
            "audience_matches",
            claims["aud"] == "postern"
                || claims["aud"]
                    .as_array()
                    .is_some_and(|v| v.iter().any(|a| a == "postern")),
        ),
        (
            "scope_matches",
            claims["scope"]
                .as_str()
                .is_some_and(|s| s.split_ascii_whitespace().any(|s| s == "postern:use")),
        ),
        (
            "subject_present",
            claims["sub"].as_str().is_some_and(|s| !s.is_empty()),
        ),
        (
            "future_expiry",
            claims["exp"].as_u64().is_some_and(|v| v > now),
        ),
        ("nbf_valid", claims["nbf"].as_u64().is_none_or(|v| v <= now)),
        ("iat_valid", claims["iat"].as_u64().is_none_or(|v| v <= now)),
    ] {
        println!("{name}: {value}");
    }
    Ok(())
}
