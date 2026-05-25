use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, Wry};

use crate::config::{self, Endpoint, GlobalConfig, ServerStatus};
use crate::events;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitPayload {
    pub global: GlobalConfig,
    pub endpoints: Vec<Endpoint>,
    pub status: ServerStatus,
}

#[tauri::command]
pub fn init_data(state: State<'_, AppState>) -> InitPayload {
    InitPayload {
        global: state.store.global(),
        endpoints: state.store.endpoints(),
        status: state.manager.status(),
    }
}

#[tauri::command]
pub fn list_endpoints(state: State<'_, AppState>) -> Vec<Endpoint> {
    state.store.endpoints()
}

#[tauri::command]
pub async fn save_endpoint(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    mut endpoint: Endpoint,
) -> Result<Endpoint, String> {
    endpoint.normalize();
    endpoint.validate().map_err(|e| e.to_string())?;

    let mut endpoints = state.store.endpoints();
    let now = chrono::Utc::now().timestamp();
    if endpoint.id.is_empty() {
        endpoint.id = uuid::Uuid::new_v4().to_string();
        endpoint.created_at = now;
        endpoint.updated_at = now;
        endpoints.push(endpoint.clone());
    } else {
        let pos = endpoints
            .iter()
            .position(|e| e.id == endpoint.id)
            .ok_or_else(|| format!("未找到端点：{}", endpoint.id))?;
        endpoint.created_at = endpoints[pos].created_at;
        endpoint.updated_at = now;
        endpoints[pos] = endpoint.clone();
    }
    config::validate_unique_paths(&endpoints).map_err(|e| e.to_string())?;

    state.store.save_endpoints(&app, endpoints.clone())?;
    state.manager.replace_routes(endpoints.clone());
    let _ = app.emit(events::ENDPOINTS_CHANGED, &endpoints);
    Ok(endpoint)
}

#[tauri::command]
pub fn delete_endpoint(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let mut endpoints = state.store.endpoints();
    let before = endpoints.len();
    endpoints.retain(|e| e.id != id);
    if endpoints.len() == before {
        return Err(format!("未找到端点：{id}"));
    }
    state.store.save_endpoints(&app, endpoints.clone())?;
    state.manager.replace_routes(endpoints.clone());
    let _ = app.emit(events::ENDPOINTS_CHANGED, &endpoints);
    Ok(())
}

#[tauri::command]
pub fn toggle_endpoint(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    let mut endpoints = state.store.endpoints();
    let target = endpoints
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or_else(|| format!("未找到端点：{id}"))?;
    target.enabled = enabled;
    target.updated_at = chrono::Utc::now().timestamp();
    config::validate_unique_paths(&endpoints).map_err(|e| e.to_string())?;
    state.store.save_endpoints(&app, endpoints.clone())?;
    state.manager.replace_routes(endpoints.clone());
    let _ = app.emit(events::ENDPOINTS_CHANGED, &endpoints);
    Ok(())
}

#[tauri::command]
pub fn get_global_config(state: State<'_, AppState>) -> GlobalConfig {
    state.store.global()
}

#[tauri::command]
pub async fn apply_global_config(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    new: GlobalConfig,
) -> Result<ServerStatus, String> {
    let old = state.store.global();
    if new.listen_port == 0 {
        return Err("端口必须在 1-65535".into());
    }
    if new.request_timeout_secs == 0 || new.request_timeout_secs > 600 {
        return Err("超时秒数必须在 1-600".into());
    }

    state.store.save_global(&app, new.clone())?;
    state.manager.replace_timeout(new.request_timeout_secs);

    let must_restart = state.manager.status().running
        && (old.listen_address != new.listen_address || old.listen_port != new.listen_port);
    let status = if must_restart {
        state
            .manager
            .restart(new.listen_address.clone(), new.listen_port)
            .await?
    } else {
        state.manager.status()
    };
    let _ = app.emit(events::STATUS_CHANGED, &status);
    Ok(status)
}

#[tauri::command]
pub fn server_status(state: State<'_, AppState>) -> ServerStatus {
    state.manager.status()
}

#[tauri::command]
pub async fn start_server(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
) -> Result<ServerStatus, String> {
    let g = state.store.global();
    let status = state.manager.start(g.listen_address, g.listen_port).await?;
    let _ = app.emit(events::STATUS_CHANGED, &status);
    Ok(status)
}

#[tauri::command]
pub async fn stop_server(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
) -> Result<ServerStatus, String> {
    let status = state.manager.stop().await?;
    let _ = app.emit(events::STATUS_CHANGED, &status);
    Ok(status)
}

#[tauri::command]
pub fn open_config_dir(app: AppHandle<Wry>) -> Result<(), String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    open_path(&dir)
}

fn open_path(path: &std::path::Path) -> Result<(), String> {
    let path = path.to_string_lossy().to_string();
    #[cfg(target_os = "macos")]
    let cmd = ("open", path);
    #[cfg(target_os = "windows")]
    let cmd = ("explorer", path);
    #[cfg(target_os = "linux")]
    let cmd = ("xdg-open", path);
    std::process::Command::new(cmd.0)
        .arg(cmd.1)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyTestResult {
    pub success: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedProxy {
    pub url: String,
    pub proxy_type: String,
    pub port: u16,
}

#[tauri::command]
pub fn get_global_proxy_url(state: State<'_, AppState>) -> String {
    state.store.global().proxy_url
}

#[tauri::command]
pub async fn set_global_proxy_url(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    url: String,
) -> Result<(), String> {
    let trimmed = url.trim().to_string();
    let opt = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.as_str())
    };
    crate::proxy::http_client::validate_proxy(opt)?;

    let mut g = state.store.global();
    g.proxy_url = trimmed.clone();
    state.store.save_global(&app, g)?;

    crate::proxy::http_client::apply_proxy(opt)?;
    Ok(())
}

#[tauri::command]
pub async fn test_proxy_url(url: String) -> Result<ProxyTestResult, String> {
    if url.trim().is_empty() {
        return Err("代理 URL 为空".into());
    }

    let start = Instant::now();

    let proxy = reqwest::Proxy::all(&url).map_err(|e| format!("代理 URL 无效：{e}"))?;
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("构造测试客户端失败：{e}"))?;

    let test_urls = [
        "https://httpbin.org/get",
        "https://www.google.com",
        "https://api.anthropic.com",
    ];

    let mut last_error: Option<String> = None;
    for target in test_urls {
        match client.head(target).send().await {
            Ok(_) => {
                return Ok(ProxyTestResult {
                    success: true,
                    latency_ms: start.elapsed().as_millis() as u64,
                    error: None,
                });
            }
            Err(e) => {
                last_error = Some(e.to_string());
            }
        }
    }

    Ok(ProxyTestResult {
        success: false,
        latency_ms: start.elapsed().as_millis() as u64,
        error: last_error,
    })
}

const SCAN_PORTS: &[(u16, &str, bool)] = &[
    (7890, "http", true),     // Clash mixed
    (7891, "socks5", false),  // Clash socks
    (1080, "socks5", false),
    (8080, "http", false),
    (8888, "http", false),
    (3128, "http", false),
    (10808, "socks5", false), // V2Ray socks
    (10809, "http", false),   // V2Ray http
];

#[tauri::command]
pub async fn scan_local_proxies() -> Vec<DetectedProxy> {
    tokio::task::spawn_blocking(|| {
        let mut found = Vec::new();
        for &(port, primary, is_mixed) in SCAN_PORTS {
            let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
            if TcpStream::connect_timeout(&addr.into(), Duration::from_millis(150)).is_ok() {
                found.push(DetectedProxy {
                    url: format!("{primary}://127.0.0.1:{port}"),
                    proxy_type: primary.into(),
                    port,
                });
                if is_mixed {
                    let alt = if primary == "http" { "socks5" } else { "http" };
                    found.push(DetectedProxy {
                        url: format!("{alt}://127.0.0.1:{port}"),
                        proxy_type: alt.into(),
                        port,
                    });
                }
            }
        }
        found
    })
    .await
    .unwrap_or_default()
}
