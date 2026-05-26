use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;
use chrono::Utc;
use tauri::{AppHandle, Wry};

use crate::config::{Endpoint, ServerStatus};
use crate::proxy::log_store::LogStore;
use crate::proxy::server::{spawn_server, RunningServer};

pub struct ProxyManager {
    routes: Arc<ArcSwap<Vec<Endpoint>>>,
    timeout: Arc<AtomicU64>,
    log_store: Arc<LogStore>,
    state: Mutex<Inner>,
}

struct Inner {
    running: Option<RunningServer>,
    started_at: Option<i64>,
    last_error: Option<String>,
}

impl ProxyManager {
    pub fn new(routes: Vec<Endpoint>, timeout_secs: u64, log_store: Arc<LogStore>) -> Self {
        Self {
            routes: Arc::new(ArcSwap::from_pointee(routes)),
            timeout: Arc::new(AtomicU64::new(timeout_secs)),
            log_store,
            state: Mutex::new(Inner {
                running: None,
                started_at: None,
                last_error: None,
            }),
        }
    }

    pub fn replace_routes(&self, endpoints: Vec<Endpoint>) {
        self.routes.store(Arc::new(endpoints));
    }

    pub fn replace_timeout(&self, secs: u64) {
        self.timeout.store(secs, Ordering::Relaxed);
    }

    pub fn status(&self) -> ServerStatus {
        let inner = self.state.lock().unwrap();
        match &inner.running {
            Some(s) => ServerStatus {
                running: true,
                listen_address: Some(s.address.clone()),
                listen_port: Some(s.port),
                started_at: inner.started_at,
                last_error: inner.last_error.clone(),
            },
            None => ServerStatus {
                running: false,
                listen_address: None,
                listen_port: None,
                started_at: None,
                last_error: inner.last_error.clone(),
            },
        }
    }

    pub async fn start(
        &self,
        address: String,
        port: u16,
        app_handle: AppHandle<Wry>,
    ) -> Result<ServerStatus, String> {
        {
            let inner = self.state.lock().unwrap();
            if inner.running.is_some() {
                return Err("代理服务已在运行".into());
            }
        }
        let routes = self.routes.clone();
        let timeout = self.timeout.clone();
        let log_store = self.log_store.clone();
        let server =
            match spawn_server(address, port, routes, timeout, log_store, Some(app_handle)).await {
                Ok(s) => s,
                Err(e) => {
                    self.set_last_error(Some(e.clone()));
                    return Err(e);
                }
            };
        {
            let mut inner = self.state.lock().unwrap();
            inner.running = Some(server);
            inner.started_at = Some(Utc::now().timestamp());
            inner.last_error = None;
        }
        Ok(self.status())
    }

    pub async fn stop(&self) -> Result<ServerStatus, String> {
        let server = {
            let mut inner = self.state.lock().unwrap();
            inner.started_at = None;
            inner.running.take()
        };
        if let Some(s) = server {
            let _ = s.shutdown.send(());
            let _ = s.join.await;
        }
        Ok(self.status())
    }

    pub async fn restart(
        &self,
        address: String,
        port: u16,
        app_handle: AppHandle<Wry>,
    ) -> Result<ServerStatus, String> {
        self.stop().await?;
        self.start(address, port, app_handle).await
    }

    fn set_last_error(&self, msg: Option<String>) {
        let mut inner = self.state.lock().unwrap();
        inner.last_error = msg;
    }
}
