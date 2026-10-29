//! Realm JSON includes explicit subject/audience mappers for introspection.
use super::{ALICE, BOB, CHARLIE, SECRET};
use serde_json::{Value, json};

pub fn realm() -> Value {
    let subject = json!({"name":"subject", "protocol":"openid-connect",
        "protocolMapper":"oidc-sub-mapper", "config":{
            "access.token.claim":"true", "introspection.token.claim":"true"}});
    let audience = json!({"name":"audience", "protocol":"openid-connect",
        "protocolMapper":"oidc-audience-mapper", "config":{
            "included.custom.audience":"postern", "included.client.audience":"postern-test", "access.token.claim":"true",
            "introspection.token.claim":"true", "id.token.claim":"false"}});
    let clients: Vec<_> = ["postern-test", "postern-caller", "wrong-audience", "short-lived"].into_iter().map(|id| {
        let mut mappers = vec![subject.clone()];
        if id != "wrong-audience" { mappers.push(audience.clone()); }
        let attributes = if id == "short-lived" { json!({"access.token.lifespan":"3"}) } else { json!({}) };
        json!({"clientId":id, "secret":SECRET, "enabled":true, "protocol":"openid-connect",
            "publicClient":false, "fullScopeAllowed":false, "directAccessGrantsEnabled":true, "standardFlowEnabled":false,
            "optionalClientScopes":["postern:use"], "protocolMappers":mappers, "attributes":attributes})
    }).collect();
    let users: Vec<_> = [("alice", ALICE), ("bob", BOB), ("charlie", CHARLIE)]
        .into_iter()
        .map(|(name, id)| {
            json!({"id":id, "username":name, "enabled":true, "emailVerified":true,
            "firstName":name, "lastName":"Fixture", "email":format!("{name}@example.invalid"),
            "credentials":[{"type":"password", "value":"fixture-password", "temporary":false}]})
        })
        .collect();
    json!({"realm":"postern-test", "enabled":true, "sslRequired":"all", "accessTokenLifespan":300,
        "clientScopes":[{"name":"postern:use", "protocol":"openid-connect",
            "attributes":{"include.in.token.scope":"true"}}], "clients":clients, "users":users})
}

pub fn hosts() -> Value {
    json!({"hosts":[
        {"id":"bob", "description":"Bob only", "allowed_subjects":[BOB]},
        {"id":"alice", "description":"Alice only", "allowed_subjects":[ALICE]},
        {"id":"shared", "description":"Shared", "allowed_subjects":[ALICE,BOB]},
        {"id":"denied", "description":"Nobody", "allowed_subjects":[]}]})
}
