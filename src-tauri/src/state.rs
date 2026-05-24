use std::sync::Arc;

use crate::config::store::ConfigStore;
use crate::proxy::manager::ProxyManager;

pub struct AppState {
    pub store: Arc<ConfigStore>,
    pub manager: Arc<ProxyManager>,
}
