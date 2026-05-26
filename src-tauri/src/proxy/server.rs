use std::net::SocketAddr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use arc_swap::ArcSwap;
use axum::routing::any;
use axum::Router;
use tauri::{AppHandle, Wry};
use tokio::sync::oneshot;

use crate::config::Endpoint;
use crate::proxy::handler::{proxy_handler, ProxyState};
use crate::proxy::log_store::LogStore;

pub struct RunningServer {
    pub address: String,
    pub port: u16,
    pub shutdown: oneshot::Sender<()>,
    pub join: tokio::task::JoinHandle<()>,
}

pub async fn spawn_server(
    address: String,
    port: u16,
    routes: Arc<ArcSwap<Vec<Endpoint>>>,
    timeout: Arc<AtomicU64>,
    log_store: Arc<LogStore>,
    app_handle: Option<AppHandle<Wry>>,
) -> Result<RunningServer, String> {
    let state = ProxyState {
        routes,
        timeout,
        log_store,
        app_handle,
    };
    let app: Router = Router::new()
        .fallback(any(proxy_handler))
        .with_state(state);

    let addr: SocketAddr = format!("{address}:{port}")
        .parse()
        .map_err(|e: std::net::AddrParseError| format!("地址解析失败：{e}"))?;

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("绑定端口失败 {addr}：{e}"))?;
    let local_addr = listener.local_addr().map_err(|e| e.to_string())?;

    let (tx, rx) = oneshot::channel::<()>();
    let join = tokio::spawn(async move {
        let _ = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            let _ = rx.await;
        })
        .await;
    });

    Ok(RunningServer {
        address: local_addr.ip().to_string(),
        port: local_addr.port(),
        shutdown: tx,
        join,
    })
}
