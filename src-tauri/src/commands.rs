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
