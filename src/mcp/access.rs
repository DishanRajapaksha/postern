//! Every tool repeats introspection and compares the HTTP gate's identity.
use super::*;
impl Postern {
    pub(super) async fn principal(
        &self,
        ctx: RequestContext<RoleServer>,
    ) -> Result<Principal, McpError> {
        let denied = || McpError::internal_error("request authentication failed", None);
        let parts = ctx
            .extensions
            .get::<axum::http::request::Parts>()
            .ok_or_else(denied)?;
        let principal = self
            .auth
            .authenticate(&parts.headers)
            .await
            .map_err(|_| denied())?;
        let original = parts.extensions.get::<Principal>().ok_or_else(denied)?;
        if original != &principal {
            return Err(denied());
        }
        Ok(principal)
    }

    pub(super) fn executor(&self) -> Result<&Arc<crate::execution::Execution>, McpError> {
        self.execution
            .as_ref()
            .ok_or_else(|| McpError::internal_error("execution disabled", None))
    }

    pub(super) fn result(
        result: anyhow::Result<serde_json::Value>,
    ) -> Result<CallToolResult, McpError> {
        // Never expose issuer errors, private paths or raw SSH diagnostics in tool errors.
        match result {
            Ok(value) => Ok(CallToolResult::success(vec![ContentBlock::text(
                value.to_string(),
            )])),
            Err(_) => Ok(CallToolResult::error(vec![ContentBlock::text(
                "Operation denied or unavailable; inspect the restricted server state.",
            )])),
        }
    }
}
