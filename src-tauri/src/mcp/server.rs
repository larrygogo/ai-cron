use crate::db::DbConn;
use crate::scheduler::engine::SchedulerState;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::*;
use rmcp::{tool_handler, ErrorData as McpError};
use std::sync::Arc;
use tauri::AppHandle;

#[derive(Clone)]
pub struct AiCronMcp {
    pub db: Arc<DbConn>,
    pub scheduler: Arc<SchedulerState>,
    pub app_handle: AppHandle,
    tool_router: ToolRouter<Self>,
}

impl AiCronMcp {
    pub fn new(db: Arc<DbConn>, scheduler: Arc<SchedulerState>, app_handle: AppHandle) -> Self {
        Self {
            db,
            scheduler,
            app_handle,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_handler]
impl rmcp::handler::server::ServerHandler for AiCronMcp {
    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_server_info(Implementation::new("ai-cron", env!("CARGO_PKG_VERSION")))
        .with_instructions(
            "AI Cron - AI Agent 定时任务调度器。\n\
             可通过 tools 创建/管理定时任务、查询运行历史、触发执行。\n\
             可通过 resources 读取任务和运行数据。\n\
             可通过 prompts 获取任务创建引导、诊断失败运行等模板。",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        super::resources::list_resources(&self.db).await
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        super::resources::list_resource_templates()
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        super::resources::read_resource(&self.db, &request.uri).await
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        super::prompts::list_prompts()
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let arguments = request.arguments.map(|map| {
            map.into_iter()
                .map(|(k, v)| (k, v.as_str().unwrap_or_default().to_string()))
                .collect()
        });
        super::prompts::get_prompt(&self.db, &request.name, arguments).await
    }
}
