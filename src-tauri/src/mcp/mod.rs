pub mod prompts;
pub mod resources;
pub mod server;
pub mod tools;

use crate::db::DbConn;
use crate::scheduler::engine::SchedulerState;
use server::AiCronMcp;
use std::sync::Arc;
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

/// MCP server status, managed as Tauri state
pub struct McpState {
    pub port: u16,
    pub cancel: CancellationToken,
}

pub async fn start_mcp_server(
    db: Arc<DbConn>,
    scheduler: Arc<SchedulerState>,
    app_handle: AppHandle,
    app_data_dir: &str,
) -> anyhow::Result<McpState> {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig,
        tower::StreamableHttpService,
    };

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    let service = StreamableHttpService::new(
        move || Ok(AiCronMcp::new(db.clone(), scheduler.clone(), app_handle.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );

    let app = axum::Router::new()
        .nest_service("/mcp", service)
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                [("content-type", "application/json")],
                r#"{"error":"not_found"}"#,
            )
                .into_response()
        });

    // Bind to port 0 to let OS assign an available port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let actual_port = listener.local_addr()?.port();

    // Write port to file for mcp-bridge.mjs
    let port_file = format!("{}/mcp-port", app_data_dir);
    std::fs::write(&port_file, actual_port.to_string())?;

    log::info!("MCP server listening on http://127.0.0.1:{}/mcp", actual_port);

    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                cancel_clone.cancelled().await;
            })
            .await
            .ok();
    });

    Ok(McpState {
        port: actual_port,
        cancel,
    })
}
