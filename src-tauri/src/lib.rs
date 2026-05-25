mod commands;
pub mod config;
mod events;
pub mod proxy;
mod state;
mod tray;

use std::sync::Arc;

use tauri::{Emitter, Manager, RunEvent, WindowEvent};

use crate::config::store::ConfigStore;
use crate::proxy::manager::ProxyManager;
use crate::state::AppState;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::init_data,
            commands::list_endpoints,
            commands::save_endpoint,
            commands::delete_endpoint,
            commands::toggle_endpoint,
            commands::get_global_config,
            commands::apply_global_config,
            commands::server_status,
            commands::start_server,
            commands::stop_server,
            commands::open_config_dir,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let store = Arc::new(ConfigStore::load(&handle).map_err(|e| {
                anyhow::anyhow!(e)
            })?);
            let g = store.global();
            crate::proxy::http_client::init(if g.proxy_url.is_empty() {
                None
            } else {
                Some(g.proxy_url.as_str())
            })
            .map_err(|e| anyhow::anyhow!(e))?;
            let manager = Arc::new(ProxyManager::new(store.endpoints(), g.request_timeout_secs));
            app.manage(AppState {
                store: store.clone(),
                manager: manager.clone(),
            });

            tray::build_tray(&handle).ok();

            if g.auto_start_server {
                let handle_for_start = handle.clone();
                let mgr_for_start = manager.clone();
                tauri::async_runtime::spawn(async move {
                    let status = match mgr_for_start
                        .start(g.listen_address.clone(), g.listen_port)
                        .await
                    {
                        Ok(s) => s,
                        Err(e) => {
                            let _ = handle_for_start.emit(
                                events::CONFIG_ERROR,
                                &serde_json::json!({"message": e}),
                            );
                            mgr_for_start.status()
                        }
                    };
                    let _ = handle_for_start.emit(events::STATUS_CHANGED, &status);
                    tray::refresh_tray(&handle_for_start);
                });
            }

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                if let Some(state) = app.try_state::<AppState>() {
                    if state.store.global().close_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error building tauri app")
        .run(|app, event| {
            if let RunEvent::ExitRequested { .. } = event {
                if let Some(state) = app.try_state::<AppState>() {
                    let mgr = state.manager.clone();
                    tauri::async_runtime::block_on(async move {
                        let _ = mgr.stop().await;
                    });
                }
            }
        });
}
