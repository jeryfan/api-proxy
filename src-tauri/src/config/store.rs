use super::{Endpoint, GlobalConfig};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Runtime, Wry};
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "config.json";
const KEY_GLOBAL: &str = "global";
const KEY_ENDPOINTS: &str = "endpoints";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    pub global: GlobalConfig,
    pub endpoints: Vec<Endpoint>,
}

pub struct ConfigStore {
    inner: Mutex<ConfigSnapshot>,
}

impl ConfigStore {
    pub fn load<R: Runtime>(app: &AppHandle<R>) -> Result<Self, String> {
        let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
        let global: GlobalConfig = match store.get(KEY_GLOBAL) {
            Some(v) => serde_json::from_value(v).unwrap_or_default(),
            None => GlobalConfig::default(),
        };
        let endpoints: Vec<Endpoint> = match store.get(KEY_ENDPOINTS) {
            Some(v) => serde_json::from_value(v).unwrap_or_default(),
            None => vec![],
        };
        Ok(Self {
            inner: Mutex::new(ConfigSnapshot { global, endpoints }),
        })
    }

    pub fn snapshot(&self) -> ConfigSnapshot {
        self.inner.lock().unwrap().clone()
    }

    pub fn global(&self) -> GlobalConfig {
        self.inner.lock().unwrap().global.clone()
    }

    pub fn endpoints(&self) -> Vec<Endpoint> {
        self.inner.lock().unwrap().endpoints.clone()
    }

    pub fn save_global(&self, app: &AppHandle<Wry>, new: GlobalConfig) -> Result<(), String> {
        {
            let mut s = self.inner.lock().unwrap();
            s.global = new.clone();
        }
        let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
        store.set(KEY_GLOBAL, serde_json::to_value(&new).unwrap());
        store.save().map_err(|e| e.to_string())
    }

    pub fn save_endpoints(&self, app: &AppHandle<Wry>, new: Vec<Endpoint>) -> Result<(), String> {
        {
            let mut s = self.inner.lock().unwrap();
            s.endpoints = new.clone();
        }
        let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
        store.set(KEY_ENDPOINTS, serde_json::to_value(&new).unwrap());
        store.save().map_err(|e| e.to_string())
    }
}
