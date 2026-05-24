use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Wry,
};

use crate::events;
use crate::state::AppState;

const ID_TOGGLE: &str = "toggle";
const ID_SHOW: &str = "show";
const ID_QUIT: &str = "quit";
const ID_STATUS: &str = "status";

pub fn build_tray(app: &AppHandle<Wry>) -> tauri::Result<()> {
    let status_item = MenuItem::with_id(app, ID_STATUS, status_text(app), false, None::<&str>)?;
    let toggle_item = MenuItem::with_id(app, ID_TOGGLE, toggle_text(app), true, None::<&str>)?;
    let show_item = MenuItem::with_id(app, ID_SHOW, "显示主窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, ID_QUIT, "退出", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[&status_item, &toggle_item, &show_item, &sep, &quit_item],
    )?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default icon".into()))?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("API 代理")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            ID_SHOW => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            ID_TOGGLE => {
                let app_clone = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_clone.state::<AppState>();
                    let manager = state.manager.clone();
                    let status = if manager.status().running {
                        manager.stop().await
                    } else {
                        let g = state.store.global();
                        manager.start(g.listen_address, g.listen_port).await
                    };
                    if let Ok(s) = status {
                        let _ = app_clone.emit(events::STATUS_CHANGED, &s);
                    }
                    refresh_tray(&app_clone);
                });
            }
            ID_QUIT => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn status_text(app: &AppHandle<Wry>) -> String {
    let Some(state) = app.try_state::<AppState>() else {
        return "⚫ 未初始化".into();
    };
    let st = state.manager.status();
    if st.running {
        format!(
            "🟢 运行中：{}:{}",
            st.listen_address.unwrap_or_default(),
            st.listen_port.unwrap_or(0)
        )
    } else {
        "⚫ 已停止".into()
    }
}

fn toggle_text(app: &AppHandle<Wry>) -> String {
    let Some(state) = app.try_state::<AppState>() else {
        return "启动代理".into();
    };
    if state.manager.status().running {
        "停止代理".into()
    } else {
        "启动代理".into()
    }
}

pub fn refresh_tray(app: &AppHandle<Wry>) {
    // 重新设置菜单文本（Tauri 2 暂不支持 in-place update 时整体重建即可）
    let _ = build_tray(app);
}
