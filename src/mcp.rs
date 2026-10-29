//! Read-only host inventory, scoped to a freshly validated Keycloak principal.
//! Every tool invocation authenticates again, in addition to the HTTP gate.

use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, ContentBlock},
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use serde::Serialize;

use crate::{auth::Authenticator, config::Host};

#[derive(Clone)]
pub struct Postern {
    hosts: Arc<Vec<Host>>,
    auth: Arc<Authenticator>,
    tool_router: ToolRouter<Self>,
}

#[derive(Serialize)]
struct HostsResponse<'a> {
    hosts: Vec<PublicHost<'a>>,
}

#[derive(Serialize)]
struct PublicHost<'a> {
    id: &'a str,
    description: &'a str,
}

#[tool_router]
impl Postern {
    pub fn new(hosts: Arc<Vec<Host>>, auth: Arc<Authenticator>) -> Self {
        Self {
            hosts,
            auth,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "List only the configured host IDs and descriptions authorised for the authenticated Keycloak user. No SSH operations are available."
    )]
    async fn hosts(&self, ctx: RequestContext<RoleServer>) -> Result<CallToolResult, McpError> {
        // Missing HTTP request parts is a hard denial. Never trust client-provided
        // identity arguments or assume a prior request authenticated this session.
        let parts = ctx
            .extensions
            .get::<axum::http::request::Parts>()
            .ok_or_else(|| McpError::internal_error("HTTP authentication context missing", None))?;
        let principal = self
            .auth
            .authenticate(&parts.headers)
            .await
            .map_err(|_| McpError::internal_error("request authentication failed", None))?;
        let visible = allowed_hosts(&self.hosts, &principal.subject);
        let response = HostsResponse { hosts: visible };
        let json =
            serde_json::to_string(&response).expect("Host consists only of serialisable strings");
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }
}

fn allowed_hosts<'a>(hosts: &'a [Host], subject: &str) -> Vec<PublicHost<'a>> {
    hosts
        .iter()
        .filter(|host| host.allowed_subjects.iter().any(|s| s == subject))
        .map(|h| PublicHost {
            id: &h.id,
            description: &h.description,
        })
        .collect()
}

#[tool_handler(
    router = self.tool_router,
    name = "postern",
    version = "0.1.0",
    instructions = "Read-only host inventory. Every HTTP request and hosts() invocation is checked with Keycloak. No SSH sessions or command execution."
)]
impl ServerHandler for Postern {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_is_exact_and_denies_everything_else() {
        let hosts = vec![
            Host {
                id: "a".into(),
                description: "one".into(),
                allowed_subjects: vec!["alice".into()],
            },
            Host {
                id: "b".into(),
                description: "two".into(),
                allowed_subjects: vec!["bob".into()],
            },
            Host {
                id: "c".into(),
                description: "three".into(),
                allowed_subjects: vec![],
            },
        ];
        assert_eq!(
            allowed_hosts(&hosts, "alice")
                .iter()
                .map(|h| h.id)
                .collect::<Vec<_>>(),
            vec!["a"]
        );
        assert_eq!(
            allowed_hosts(&hosts, "bob")
                .iter()
                .map(|h| h.id)
                .collect::<Vec<_>>(),
            vec!["b"]
        );
        assert!(allowed_hosts(&hosts, "charlie").is_empty());
        assert!(allowed_hosts(&hosts, "ali").is_empty());
        let json = serde_json::to_value(HostsResponse {
            hosts: allowed_hosts(&hosts, "alice"),
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({"hosts":[{"id":"a","description":"one"}]})
        );
        assert!(!json.to_string().contains("allowed_subjects"));
    }
}
