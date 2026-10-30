//! Session-bound, write-capable tools; the server does not classify shell contents.
use super::*;
use crate::execution::{AuditQuery, Execute, OpenSession, SessionId};
use rmcp::handler::server::wrapper::Parameters;

#[tool_router(router = execution_router, vis = "pub(super)")]
impl Postern {
    #[tool(description = "Open a time-limited SSH transport to an authorised configured host.")]
    async fn open_session(
        &self,
        Parameters(args): Parameters<OpenSession>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let principal = self.principal(ctx).await?;
        Self::result(self.executor()?.open(&principal.subject, args).await)
    }

    #[tool(
        description = "Execute arbitrary shell text in an owned SSH session. Every invocation is durably audited; never retry automatically. Timeout 1..60 seconds; output is bounded.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn exec(
        &self,
        Parameters(args): Parameters<Execute>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let principal = self.principal(ctx).await?;
        Self::result(self.executor()?.execute(&principal.subject, args).await)
    }

    #[tool(
        description = "Close an owned SSH session. Already dispatched commands drain under their timeout."
    )]
    async fn close_session(
        &self,
        Parameters(args): Parameters<SessionId>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let principal = self.principal(ctx).await?;
        Self::result(self.executor()?.close(&principal.subject, args).await)
    }

    #[tool(
        description = "Read only your own restricted command audit records. Limit 1..100; use next_cursor as before for pagination.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true
        )
    )]
    async fn audit(
        &self,
        Parameters(args): Parameters<AuditQuery>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let principal = self.principal(ctx).await?;
        Self::result(self.executor()?.audit(&principal.subject, args))
    }
}
