# v2 实施计划：出站代理 + Header 重构 + Logo

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 apiproxy 添加全局出站代理（HTTP/SOCKS5），精简顶部 Header（只留 Logo+标题+设置 / 服务开关+添加），添加品牌 Logo，并清理代码中对参考项目的命名残留。

**Architecture:** 引入 `proxy/http_client.rs` 模块用 `OnceLock<RwLock<reqwest::Client>>` 持有可热切换的全局 HTTP 客户端；ProxyManager 改为每请求时取最新 client；在 Settings 面板新增「出站代理」组件；用 Tauri 命令做配置 + 测试 + 扫描；用纯 SVG 实现 BrandLogo，配合 `pnpm tauri icon` 自动生成全套 App icon。

**Tech Stack:** Tauri 2, Rust（reqwest 加 `socks` feature, url, tokio, OnceLock）, React 18, TypeScript, Tailwind CSS, lucide-react

**Spec:** [/docs/superpowers/specs/2026-05-25-v2-proxy-and-ui-design.md](../specs/2026-05-25-v2-proxy-and-ui-design.md)

---

## Phase 1：后端 — 出站代理基础设施

### Task 1：reqwest 加 socks feature 并新建 http_client 模块

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/proxy/http_client.rs`
- Modify: `src-tauri/src/proxy/mod.rs`

- [ ] **Step 1: 修改 `src-tauri/Cargo.toml` reqwest 行加 socks feature**

找到现有：
```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream", "json"] }
```
改成：
```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream", "json", "socks"] }
```

- [ ] **Step 2: 创建 `src-tauri/src/proxy/http_client.rs`**

```rust
//! 全局 HTTP 客户端模块
//!
//! 提供支持全局代理配置的 reqwest 客户端。
//! 所有发往上游的 HTTP 请求都应通过本模块取 client。

use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use reqwest::Client;

static GLOBAL_CLIENT: OnceLock<RwLock<Client>> = OnceLock::new();
static CURRENT_PROXY_URL: OnceLock<RwLock<Option<String>>> = OnceLock::new();

/// 初始化全局客户端（应用启动时调用一次）
pub fn init(proxy_url: Option<&str>) -> Result<(), String> {
    let effective = proxy_url.filter(|s| !s.trim().is_empty());
    let client = build_client(effective)?;
    if GLOBAL_CLIENT.set(RwLock::new(client)).is_err() {
        return apply_proxy(proxy_url);
    }
    let _ = CURRENT_PROXY_URL.set(RwLock::new(effective.map(|s| s.to_string())));
    Ok(())
}

/// 仅验证代理 URL，不应用
pub fn validate_proxy(proxy_url: Option<&str>) -> Result<(), String> {
    let effective = proxy_url.filter(|s| !s.trim().is_empty());
    build_client(effective).map(|_| ())
}

/// 应用代理（已 validate 过）
pub fn apply_proxy(proxy_url: Option<&str>) -> Result<(), String> {
    let effective = proxy_url.filter(|s| !s.trim().is_empty());
    let new_client = build_client(effective)?;
    if let Some(lock) = GLOBAL_CLIENT.get() {
        let mut c = lock.write().map_err(|_| "client lock poisoned".to_string())?;
        *c = new_client;
    } else {
        return init(proxy_url);
    }
    if let Some(lock) = CURRENT_PROXY_URL.get() {
        let mut u = lock.write().map_err(|_| "url lock poisoned".to_string())?;
        *u = effective.map(|s| s.to_string());
    }
    Ok(())
}

/// 取当前客户端
pub fn get() -> Client {
    GLOBAL_CLIENT
        .get()
        .and_then(|lock| lock.read().ok())
        .map(|c| c.clone())
        .unwrap_or_else(|| build_client(None).unwrap_or_default())
}

/// 当前代理 URL（用于状态展示）
pub fn get_current_proxy_url() -> Option<String> {
    CURRENT_PROXY_URL
        .get()
        .and_then(|lock| lock.read().ok())
        .and_then(|u| u.clone())
}

/// 把 URL 中的 username:password 中的密码部分换成 ***（用于日志）
pub fn mask_url(url: &str) -> String {
    match url::Url::parse(url) {
        Ok(mut u) => {
            if !u.password().unwrap_or("").is_empty() {
                let _ = u.set_password(Some("***"));
            }
            u.to_string()
        }
        Err(_) => url.to_string(),
    }
}

fn build_client(proxy_url: Option<&str>) -> Result<Client, String> {
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(600))
        .connect_timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(10)
        .tcp_keepalive(Duration::from_secs(60));

    if let Some(url) = proxy_url {
        let parsed = url::Url::parse(url)
            .map_err(|e| format!("代理 URL 无效 '{}': {}", mask_url(url), e))?;
        let scheme = parsed.scheme();
        if !["http", "https", "socks5", "socks5h"].contains(&scheme) {
            return Err(format!(
                "不支持的代理协议 '{}'，支持 http / https / socks5 / socks5h",
                scheme
            ));
        }
        let proxy = reqwest::Proxy::all(url)
            .map_err(|e| format!("构造代理失败 '{}': {}", mask_url(url), e))?;
        builder = builder.proxy(proxy);
    }

    builder.build().map_err(|e| format!("构建 HTTP 客户端失败：{}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_http_proxy() {
        assert!(validate_proxy(Some("http://127.0.0.1:7890")).is_ok());
    }

    #[test]
    fn validate_accepts_socks5_proxy() {
        assert!(validate_proxy(Some("socks5://127.0.0.1:1080")).is_ok());
        assert!(validate_proxy(Some("socks5h://127.0.0.1:1080")).is_ok());
    }

    #[test]
    fn validate_accepts_https_proxy() {
        assert!(validate_proxy(Some("https://proxy.example.com:443")).is_ok());
    }

    #[test]
    fn validate_accepts_empty_as_direct() {
        assert!(validate_proxy(None).is_ok());
        assert!(validate_proxy(Some("")).is_ok());
        assert!(validate_proxy(Some("   ")).is_ok());
    }

    #[test]
    fn validate_rejects_ftp_scheme() {
        let err = validate_proxy(Some("ftp://x.com")).unwrap_err();
        assert!(err.contains("不支持的代理协议"));
    }

    #[test]
    fn validate_rejects_garbage_url() {
        assert!(validate_proxy(Some("not a url")).is_err());
    }

    #[test]
    fn validate_accepts_url_with_auth() {
        assert!(validate_proxy(Some("http://user:pass@127.0.0.1:7890")).is_ok());
    }

    #[test]
    fn mask_url_hides_password() {
        let m = mask_url("http://alice:s3cret@127.0.0.1:7890");
        assert!(m.contains("alice"));
        assert!(!m.contains("s3cret"));
        assert!(m.contains("***"));
    }

    #[test]
    fn mask_url_passthrough_when_no_password() {
        assert_eq!(mask_url("http://127.0.0.1:7890"), "http://127.0.0.1:7890/");
    }
}
```

- [ ] **Step 3: 更新 `src-tauri/src/proxy/mod.rs`**

```rust
pub mod handler;
pub mod http_client;
pub mod manager;
pub mod server;
pub mod transform;
```

- [ ] **Step 4: 运行单元测试**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo test proxy::http_client:: -- --nocapture
```

预期：9 个测试全部通过。

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(backend): add global http_client with hot-swappable proxy

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2：GlobalConfig 增加 proxyUrl 字段

**Files:**
- Modify: `src-tauri/src/config/mod.rs`

- [ ] **Step 1: 修改 `GlobalConfig` 结构，在最后一个字段后增加 `proxy_url`**

找到 `pub struct GlobalConfig { ... }`，在 `pub auto_start_server: bool,` 后面加：

```rust
    #[serde(default)]
    pub proxy_url: String,
```

`Default` impl 中在末尾加：

```rust
            proxy_url: String::new(),
```

最终样子：
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalConfig {
    pub listen_address: String,
    pub listen_port: u16,
    pub request_timeout_secs: u64,
    pub theme: String,
    pub close_to_tray: bool,
    pub auto_start_server: bool,
    #[serde(default)]
    pub proxy_url: String,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            listen_address: "0.0.0.0".into(),
            listen_port: 8118,
            request_timeout_secs: 60,
            theme: "system".into(),
            close_to_tray: true,
            auto_start_server: true,
            proxy_url: String::new(),
        }
    }
}
```

- [ ] **Step 2: 编译**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo build 2>&1 | tail -5
```

预期：成功（warnings OK）。

- [ ] **Step 3: 提交**

```bash
git add -A && git commit -m "feat(backend): add proxy_url field to GlobalConfig

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 3：ProxyManager 改用全局 http_client

**Files:**
- Modify: `src-tauri/src/proxy/manager.rs`
- Modify: `src-tauri/src/proxy/handler.rs`
- Modify: `src-tauri/src/proxy/server.rs`

- [ ] **Step 1: 重写 `src-tauri/src/proxy/manager.rs`，删除自有 client 字段**

把整个文件替换为：

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;
use chrono::Utc;

use crate::config::{Endpoint, ServerStatus};
use crate::proxy::server::{spawn_server, RunningServer};

pub struct ProxyManager {
    routes: Arc<ArcSwap<Vec<Endpoint>>>,
    timeout: Arc<AtomicU64>,
    state: Mutex<Inner>,
}

struct Inner {
    running: Option<RunningServer>,
    started_at: Option<i64>,
    last_error: Option<String>,
}

impl ProxyManager {
    pub fn new(routes: Vec<Endpoint>, timeout_secs: u64) -> Self {
        Self {
            routes: Arc::new(ArcSwap::from_pointee(routes)),
            timeout: Arc::new(AtomicU64::new(timeout_secs)),
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

    pub async fn start(&self, address: String, port: u16) -> Result<ServerStatus, String> {
        {
            let inner = self.state.lock().unwrap();
            if inner.running.is_some() {
                return Err("代理服务已在运行".into());
            }
        }
        let routes = self.routes.clone();
        let timeout = self.timeout.clone();
        let server = match spawn_server(address, port, routes, timeout).await {
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

    pub async fn restart(&self, address: String, port: u16) -> Result<ServerStatus, String> {
        self.stop().await?;
        self.start(address, port).await
    }

    fn set_last_error(&self, msg: Option<String>) {
        let mut inner = self.state.lock().unwrap();
        inner.last_error = msg;
    }
}
```

- [ ] **Step 2: 修改 `src-tauri/src/proxy/handler.rs` ProxyState 删除 client 字段**

找到：
```rust
#[derive(Clone)]
pub struct ProxyState {
    pub routes: Arc<ArcSwap<Vec<Endpoint>>>,
    pub client: reqwest::Client,
    pub timeout: Arc<std::sync::atomic::AtomicU64>,
}
```

改成：
```rust
#[derive(Clone)]
pub struct ProxyState {
    pub routes: Arc<ArcSwap<Vec<Endpoint>>>,
    pub timeout: Arc<std::sync::atomic::AtomicU64>,
}
```

- [ ] **Step 3: 在 `handler.rs` 的 `proxy_handler` 内换 client 来源**

找到：
```rust
    let mut req_builder = state
        .client
        .request(method, upstream_url.clone())
```

改成：
```rust
    let client = crate::proxy::http_client::get();
    let mut req_builder = client
        .request(method, upstream_url.clone())
```

- [ ] **Step 4: 修改 `src-tauri/src/proxy/server.rs` 删除 client 参数**

把：
```rust
pub async fn spawn_server(
    address: String,
    port: u16,
    routes: Arc<ArcSwap<Vec<Endpoint>>>,
    timeout: Arc<AtomicU64>,
    client: reqwest::Client,
) -> Result<RunningServer, String> {
    let state = ProxyState {
        routes,
        client,
        timeout,
    };
```

改成：
```rust
pub async fn spawn_server(
    address: String,
    port: u16,
    routes: Arc<ArcSwap<Vec<Endpoint>>>,
    timeout: Arc<AtomicU64>,
) -> Result<RunningServer, String> {
    let state = ProxyState { routes, timeout };
```

- [ ] **Step 5: 编译**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo build 2>&1 | tail -10
```

预期：成功。

- [ ] **Step 6: 修复集成测试 `tests/proxy_integration.rs` 的 `spawn_server` 调用**

把文件里所有
```rust
spawn_server("127.0.0.1".into(), 0, routes, timeout, reqwest::Client::new())
spawn_server("127.0.0.1".into(), 0, routes, timeout, client)
spawn_server("127.0.0.1".into(), 0, routes, timeout, client.clone())
```
都改成（去掉最后一个参数）：
```rust
spawn_server("127.0.0.1".into(), 0, routes, timeout)
```

且测试文件开头需要初始化 http_client。在每个 `#[tokio::test]` 函数最开始（紧接函数体第一行）加一行：

```rust
let _ = apiproxy_lib::proxy::http_client::init(None);
```

这是幂等的（init 之后再 init 会自动 fallback 到 apply_proxy 路径，并以 None 走直连）。

- [ ] **Step 7: 运行所有测试**

```bash
cargo test 2>&1 | tail -20
```

预期：29 + 9 = 38 个测试全部通过。

- [ ] **Step 8: 提交**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy
git add -A && git commit -m "refactor(backend): ProxyManager uses global http_client instead of owned client

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 4：lib.rs setup 初始化 http_client

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 修改 `src-tauri/src/lib.rs` 的 `setup` 闭包**

找到：
```rust
            let g = store.global();
            let manager = Arc::new(ProxyManager::new(store.endpoints(), g.request_timeout_secs));
```

在前面插入 init 调用：
```rust
            let g = store.global();
            crate::proxy::http_client::init(if g.proxy_url.is_empty() {
                None
            } else {
                Some(g.proxy_url.as_str())
            })
            .map_err(|e| anyhow::anyhow!(e))?;
            let manager = Arc::new(ProxyManager::new(store.endpoints(), g.request_timeout_secs));
```

- [ ] **Step 2: 编译**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo build 2>&1 | tail -5
```

预期：成功。

- [ ] **Step 3: 提交**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy
git add -A && git commit -m "feat(backend): initialize global http_client at startup

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 2：出站代理 Tauri 命令

### Task 5：实现 get/set/test/scan 四个命令

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 `src-tauri/src/commands.rs` 末尾追加 4 个新命令**

```rust
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
    let opt = if trimmed.is_empty() { None } else { Some(trimmed.as_str()) };
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
    (7890, "http", true),      // Clash mixed
    (7891, "socks5", false),   // Clash socks
    (1080, "socks5", false),
    (8080, "http", false),
    (8888, "http", false),
    (3128, "http", false),
    (10808, "socks5", false),  // V2Ray socks
    (10809, "http", false),    // V2Ray http
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
```

- [ ] **Step 2: 在 `src-tauri/src/lib.rs` 的 `invoke_handler!` 宏中注册 4 个新命令**

找到 `tauri::generate_handler![...]`，在末尾加：

```rust
            commands::get_global_proxy_url,
            commands::set_global_proxy_url,
            commands::test_proxy_url,
            commands::scan_local_proxies,
```

最终列表样子：
```rust
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
            commands::get_global_proxy_url,
            commands::set_global_proxy_url,
            commands::test_proxy_url,
            commands::scan_local_proxies,
        ])
```

- [ ] **Step 3: 编译**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo build 2>&1 | tail -5
```

预期：成功。

- [ ] **Step 4: 跑全部测试**

```bash
cargo test 2>&1 | tail -10
```

预期：所有测试通过。

- [ ] **Step 5: 提交**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy
git add -A && git commit -m "feat(backend): add commands for global proxy get/set/test/scan

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 3：前端 — 类型、API、Logo

### Task 6：types + api 扩展

**Files:**
- Modify: `src/types.ts`
- Modify: `src/lib/api.ts`

- [ ] **Step 1: 修改 `src/types.ts`，在 `GlobalConfig` interface 末尾加字段，并新增两个 interface**

找到 `interface GlobalConfig { ... }`，在 `autoStartServer: boolean;` 后加：
```ts
  proxyUrl: string;
```

在文件末尾增加：
```ts
export interface ProxyTestResult {
  success: boolean;
  latencyMs: number;
  error?: string;
}

export interface DetectedProxy {
  url: string;
  proxyType: string;
  port: number;
}
```

- [ ] **Step 2: 修改 `src/lib/api.ts`，在 `api` 对象末尾加 4 个方法**

替换文件为：
```ts
import { invoke } from "@tauri-apps/api/core";
import type {
  DetectedProxy,
  Endpoint,
  GlobalConfig,
  InitPayload,
  ProxyTestResult,
  ServerStatus,
} from "@/types";

export const api = {
  initData: () => invoke<InitPayload>("init_data"),
  listEndpoints: () => invoke<Endpoint[]>("list_endpoints"),
  saveEndpoint: (endpoint: Endpoint) =>
    invoke<Endpoint>("save_endpoint", { endpoint }),
  deleteEndpoint: (id: string) => invoke<void>("delete_endpoint", { id }),
  toggleEndpoint: (id: string, enabled: boolean) =>
    invoke<void>("toggle_endpoint", { id, enabled }),
  getGlobalConfig: () => invoke<GlobalConfig>("get_global_config"),
  applyGlobalConfig: (config: GlobalConfig) =>
    invoke<ServerStatus>("apply_global_config", { new: config }),
  serverStatus: () => invoke<ServerStatus>("server_status"),
  startServer: () => invoke<ServerStatus>("start_server"),
  stopServer: () => invoke<ServerStatus>("stop_server"),
  openConfigDir: () => invoke<void>("open_config_dir"),
  getGlobalProxyUrl: () => invoke<string>("get_global_proxy_url"),
  setGlobalProxyUrl: (url: string) =>
    invoke<void>("set_global_proxy_url", { url }),
  testProxyUrl: (url: string) =>
    invoke<ProxyTestResult>("test_proxy_url", { url }),
  scanLocalProxies: () => invoke<DetectedProxy[]>("scan_local_proxies"),
};

export const EVENTS = {
  STATUS_CHANGED: "proxy://status-changed",
  ENDPOINTS_CHANGED: "proxy://endpoints-changed",
  CONFIG_ERROR: "proxy://config-error",
} as const;
```

- [ ] **Step 3: typecheck**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy
pnpm typecheck 2>&1 | tail -5
```

预期：通过。

- [ ] **Step 4: 提交**

```bash
git add -A && git commit -m "feat(frontend): add types and api for global proxy

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 7：BrandLogo 组件

**Files:**
- Create: `src/components/BrandLogo.tsx`

- [ ] **Step 1: 创建 `src/components/BrandLogo.tsx`**

```tsx
interface Props {
  className?: string;
}

export function BrandLogo({ className }: Props) {
  return (
    <svg
      viewBox="0 0 32 32"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <defs>
        <linearGradient id="brand-g" x1="0" y1="0" x2="32" y2="32">
          <stop offset="0%" stopColor="#0A84FF" />
          <stop offset="100%" stopColor="#7C3AED" />
        </linearGradient>
      </defs>
      <rect x="2" y="2" width="28" height="28" rx="7" fill="url(#brand-g)" />
      <path
        d="M11 11 L7 16 L11 21"
        stroke="white"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        fill="none"
      />
      <path
        d="M14 11 L10 16 L14 21"
        stroke="white"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        fill="none"
        opacity="0.5"
      />
      <path
        d="M22 10 L13 22"
        stroke="white"
        strokeWidth="2.2"
        strokeLinecap="round"
        opacity="0.85"
      />
      <path
        d="M21 11 L25 16 L21 21"
        stroke="white"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        fill="none"
        opacity="0.5"
      />
      <path
        d="M24 11 L28 16 L24 21"
        stroke="white"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        fill="none"
      />
    </svg>
  );
}
```

- [ ] **Step 2: typecheck**

```bash
pnpm typecheck 2>&1 | tail -3
```

预期：通过。

- [ ] **Step 3: 提交**

```bash
git add -A && git commit -m "feat(frontend): add BrandLogo component

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 4：前端 — Header 重构

### Task 8：精简 ServerToggle、删除 ServerStatusBadge 在 Header 中的使用

**Files:**
- Modify: `src/components/ServerToggle.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: 重写 `src/components/ServerToggle.tsx`，删 label，加 tooltip**

```tsx
import { toast } from "sonner";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/api";
import type { ServerStatus } from "@/types";

export function ServerToggle({ status }: { status: ServerStatus }) {
  const handleChange = async (checked: boolean) => {
    try {
      if (checked) {
        await api.startServer();
        toast.success("服务已启动");
      } else {
        await api.stopServer();
        toast.success("服务已停止");
      }
    } catch (e: unknown) {
      toast.error(
        typeof e === "string" ? e : (e as Error)?.message ?? "操作失败",
      );
    }
  };

  const tooltip = status.running
    ? `运行中 ${status.listenAddress}:${status.listenPort}`
    : "已停止";

  return (
    <Switch
      checked={status.running}
      onCheckedChange={handleChange}
      title={tooltip}
    />
  );
}
```

- [ ] **Step 2: 重写 `src/App.tsx` 的 header 部分**

整文件替换：

```tsx
import * as React from "react";
import { Plus, Settings as SettingsIcon } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { useTheme } from "@/components/theme-provider";
import { api, EVENTS } from "@/lib/api";
import { isMac } from "@/lib/platform";
import type { Endpoint, GlobalConfig, ServerStatus } from "@/types";

import { BrandLogo } from "@/components/BrandLogo";
import { ServerToggle } from "@/components/ServerToggle";
import { EndpointList } from "@/components/endpoints/EndpointList";
import { AddEndpointDialog } from "@/components/endpoints/AddEndpointDialog";
import { EditEndpointDialog } from "@/components/endpoints/EditEndpointDialog";
import { SettingsPanel } from "@/components/settings/SettingsPanel";

const DRAG_BAR = isMac() ? 28 : 0;
const HEADER_H = 64;

export default function App() {
  const { setTheme } = useTheme();
  const [endpoints, setEndpoints] = React.useState<Endpoint[]>([]);
  const [global, setGlobal] = React.useState<GlobalConfig | null>(null);
  const [status, setStatus] = React.useState<ServerStatus>({ running: false });
  const [showAdd, setShowAdd] = React.useState(false);
  const [editing, setEditing] = React.useState<Endpoint | null>(null);
  const [showSettings, setShowSettings] = React.useState(false);

  const refresh = React.useCallback(async () => {
    const data = await api.initData();
    setEndpoints(data.endpoints);
    setGlobal(data.global);
    setStatus(data.status);
    setTheme(data.global.theme);
  }, [setTheme]);

  React.useEffect(() => {
    refresh().catch((e) => toast.error(`初始化失败：${e}`));
    getCurrentWindow()
      .show()
      .catch(() => undefined);
  }, [refresh]);

  React.useEffect(() => {
    const unlisten1 = listen<ServerStatus>(EVENTS.STATUS_CHANGED, (e) =>
      setStatus(e.payload),
    );
    const unlisten2 = listen<Endpoint[]>(EVENTS.ENDPOINTS_CHANGED, (e) =>
      setEndpoints(e.payload),
    );
    const unlisten3 = listen<{ message: string }>(EVENTS.CONFIG_ERROR, (e) =>
      toast.error(e.payload.message),
    );
    return () => {
      unlisten1.then((f) => f());
      unlisten2.then((f) => f());
      unlisten3.then((f) => f());
    };
  }, []);

  if (!global) {
    return (
      <div className="flex h-screen items-center justify-center text-muted-foreground">
        加载中…
      </div>
    );
  }

  return (
    <div
      className="flex flex-col h-screen overflow-hidden bg-background text-foreground selection:bg-primary/30 pb-4"
      style={{ paddingTop: DRAG_BAR + HEADER_H }}
    >
      {DRAG_BAR > 0 && (
        <div
          data-tauri-drag-region
          style={{ height: DRAG_BAR }}
          className="fixed left-0 right-0 top-0 z-[70]"
        />
      )}

      <header
        data-tauri-drag-region
        className="fixed left-0 right-0 z-50 bg-background/80 backdrop-blur-md"
        style={{ top: DRAG_BAR, height: HEADER_H }}
      >
        <div className="flex h-full items-center justify-between gap-2 px-6">
          <div className="flex items-center gap-2" data-tauri-no-drag>
            <BrandLogo className="h-7 w-7" />
            <span className="text-xl font-semibold text-blue-500 dark:text-blue-400">
              API 代理
            </span>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={() => setShowSettings(true)}
              title="设置"
            >
              <SettingsIcon className="h-4 w-4" />
            </Button>
          </div>
          <div className="flex items-center gap-3" data-tauri-no-drag>
            <ServerToggle status={status} />
            <Button
              onClick={() => setShowAdd(true)}
              size="icon"
              className="ml-2 bg-orange-500 hover:bg-orange-600 dark:bg-orange-500 dark:hover:bg-orange-600 text-white shadow-lg shadow-orange-500/30 dark:shadow-orange-500/40 rounded-full w-8 h-8"
            >
              <Plus className="w-5 h-5" />
            </Button>
          </div>
        </div>
      </header>

      <main className="flex-1 min-h-0 flex flex-col overflow-y-auto animate-fade-in">
        <EndpointList
          endpoints={endpoints}
          status={status}
          listenAddress={status.listenAddress ?? global.listenAddress}
          listenPort={status.listenPort ?? global.listenPort}
          onEdit={setEditing}
          onAdd={() => setShowAdd(true)}
        />
      </main>

      <AddEndpointDialog
        open={showAdd}
        onClose={() => setShowAdd(false)}
        onSaved={() => setShowAdd(false)}
      />
      {editing && (
        <EditEndpointDialog
          endpoint={editing}
          onClose={() => setEditing(null)}
          onSaved={() => setEditing(null)}
        />
      )}
      <SettingsPanel
        open={showSettings}
        global={global}
        onClose={() => setShowSettings(false)}
        onSaved={(g) => {
          setGlobal(g);
          setTheme(g.theme);
        }}
      />
    </div>
  );
}
```

变化点：
- import：去掉 `ModeToggle`、`ServerStatusBadge`，加 `BrandLogo`
- header 左侧：BrandLogo + 标题 + Settings
- header 右侧：只剩 ServerToggle + Add（加 orange-500 圆形 styling）

- [ ] **Step 3: 删除已不被引用的 `src/components/ServerStatusBadge.tsx`**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy
rm src/components/ServerStatusBadge.tsx
```

- [ ] **Step 4: typecheck**

```bash
pnpm typecheck 2>&1 | tail -5
```

预期：通过。

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(frontend): restructure header with logo, simplify ServerToggle

- Remove ServerStatusBadge from header
- Remove ModeToggle from header (kept in Settings panel)
- ServerToggle: drop label, add tooltip with listen address
- Add button now orange rounded-full per spec

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 5：前端 — 出站代理 UI

### Task 9：GlobalProxySettings 组件 + 接入 SettingsPanel

**Files:**
- Create: `src/components/settings/GlobalProxySettings.tsx`
- Modify: `src/components/settings/SettingsPanel.tsx`

- [ ] **Step 1: 创建 `src/components/settings/GlobalProxySettings.tsx`**

```tsx
import * as React from "react";
import { Eye, EyeOff, Loader2, Search, TestTube2, X } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { api } from "@/lib/api";
import type { DetectedProxy } from "@/types";

function extractAuth(url: string): {
  baseUrl: string;
  username: string;
  password: string;
} {
  if (!url.trim()) return { baseUrl: "", username: "", password: "" };
  try {
    const parsed = new URL(url);
    const username = decodeURIComponent(parsed.username || "");
    const password = decodeURIComponent(parsed.password || "");
    parsed.username = "";
    parsed.password = "";
    return { baseUrl: parsed.toString(), username, password };
  } catch {
    return { baseUrl: url, username: "", password: "" };
  }
}

function mergeAuth(baseUrl: string, username: string, password: string): string {
  if (!baseUrl.trim()) return "";
  if (!username.trim()) return baseUrl;
  try {
    const parsed = new URL(baseUrl);
    parsed.username = username.trim();
    if (password) parsed.password = password;
    return parsed.toString();
  } catch {
    return baseUrl;
  }
}

export function GlobalProxySettings() {
  const [saved, setSaved] = React.useState<string | null>(null);
  const [url, setUrl] = React.useState("");
  const [username, setUsername] = React.useState("");
  const [password, setPassword] = React.useState("");
  const [showPassword, setShowPassword] = React.useState(false);
  const [dirty, setDirty] = React.useState(false);
  const [detected, setDetected] = React.useState<DetectedProxy[]>([]);
  const [saving, setSaving] = React.useState(false);
  const [scanning, setScanning] = React.useState(false);
  const [testing, setTesting] = React.useState(false);

  const fullUrl = React.useMemo(
    () => mergeAuth(url, username, password),
    [url, username, password],
  );

  React.useEffect(() => {
    api.getGlobalProxyUrl().then((u) => {
      setSaved(u);
      const { baseUrl, username: usr, password: pwd } = extractAuth(u);
      setUrl(baseUrl);
      setUsername(usr);
      setPassword(pwd);
      setDirty(false);
    });
  }, []);

  const handleSave = async () => {
    setSaving(true);
    try {
      await api.setGlobalProxyUrl(fullUrl);
      setSaved(fullUrl);
      setDirty(false);
      toast.success(fullUrl ? "出站代理已应用" : "已切换为直连");
    } catch (e: unknown) {
      toast.error(typeof e === "string" ? e : (e as Error)?.message ?? "保存失败");
    } finally {
      setSaving(false);
    }
  };

  const handleTest = async () => {
    if (!fullUrl) return;
    setTesting(true);
    try {
      const res = await api.testProxyUrl(fullUrl);
      if (res.success) {
        toast.success(`代理可用 · ${res.latencyMs}ms`);
      } else {
        toast.error(`代理不可用：${res.error ?? "未知错误"}`);
      }
    } catch (e: unknown) {
      toast.error(typeof e === "string" ? e : (e as Error)?.message ?? "测试失败");
    } finally {
      setTesting(false);
    }
  };

  const handleScan = async () => {
    setScanning(true);
    try {
      const result = await api.scanLocalProxies();
      setDetected(result);
      if (result.length === 0) toast.info("未发现本机代理端口");
    } finally {
      setScanning(false);
    }
  };

  const handleSelect = (proxyUrl: string) => {
    const { baseUrl, username: u, password: p } = extractAuth(proxyUrl);
    setUrl(baseUrl);
    setUsername(u);
    setPassword(p);
    setDirty(true);
    setDetected([]);
  };

  const handleClear = () => {
    setUrl("");
    setUsername("");
    setPassword("");
    setDirty(true);
  };

  const isUnchangedFromSaved = saved === fullUrl;

  return (
    <section className="rounded-xl border p-4 space-y-3">
      <div className="space-y-1">
        <Label className="text-base font-semibold">出站代理</Label>
        <p className="text-sm text-muted-foreground">
          让本工具发往上游 API 的请求经由 HTTP 或 SOCKS 代理。留空表示直连。
        </p>
      </div>

      <div className="flex gap-2">
        <Input
          placeholder="http://127.0.0.1:7890 / socks5://127.0.0.1:1080"
          value={url}
          onChange={(e) => {
            setUrl(e.target.value);
            setDirty(true);
          }}
          className="font-mono text-sm flex-1"
        />
        <Button
          variant="outline"
          size="icon"
          disabled={scanning}
          onClick={handleScan}
          title="扫描本机代理"
        >
          {scanning ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Search className="h-4 w-4" />
          )}
        </Button>
        <Button
          variant="outline"
          size="icon"
          disabled={!fullUrl || testing}
          onClick={handleTest}
          title="测试连通"
        >
          {testing ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <TestTube2 className="h-4 w-4" />
          )}
        </Button>
        <Button
          variant="outline"
          size="icon"
          disabled={!url && !username && !password}
          onClick={handleClear}
          title="清空"
        >
          <X className="h-4 w-4" />
        </Button>
        <Button
          onClick={handleSave}
          disabled={!dirty || saving || isUnchangedFromSaved}
          size="sm"
        >
          {saving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
          保存
        </Button>
      </div>

      <div className="flex gap-2">
        <Input
          placeholder="用户名（可选）"
          value={username}
          onChange={(e) => {
            setUsername(e.target.value);
            setDirty(true);
          }}
          className="font-mono text-sm flex-1"
        />
        <div className="relative flex-1">
          <Input
            type={showPassword ? "text" : "password"}
            placeholder="密码（可选）"
            value={password}
            onChange={(e) => {
              setPassword(e.target.value);
              setDirty(true);
            }}
            className="font-mono text-sm pr-10"
          />
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="absolute right-0 top-0 h-full px-3 hover:bg-transparent"
            onClick={() => setShowPassword(!showPassword)}
            tabIndex={-1}
          >
            {showPassword ? (
              <EyeOff className="h-4 w-4 text-muted-foreground" />
            ) : (
              <Eye className="h-4 w-4 text-muted-foreground" />
            )}
          </Button>
        </div>
      </div>

      {detected.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {detected.map((p) => (
            <Button
              key={p.url}
              variant="secondary"
              size="sm"
              onClick={() => handleSelect(p.url)}
              className="font-mono text-xs"
            >
              {p.url}
            </Button>
          ))}
        </div>
      )}
    </section>
  );
}
```

- [ ] **Step 2: 修改 `src/components/settings/SettingsPanel.tsx`，在「代理监听」与「行为」之间插入 `<GlobalProxySettings />`**

找到 `</section>` 后紧接 `<section className="rounded-xl border p-4 space-y-4">\n          <Label className="text-base font-semibold">行为</Label>` 的位置，在前面插入：

```tsx
        <GlobalProxySettings />
```

并在文件顶部 import：
```tsx
import { GlobalProxySettings } from "./GlobalProxySettings";
```

- [ ] **Step 3: typecheck**

```bash
pnpm typecheck 2>&1 | tail -3
```

预期：通过。

- [ ] **Step 4: 提交**

```bash
git add -A && git commit -m "feat(frontend): add GlobalProxySettings to SettingsPanel

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 6：App icon 生成

### Task 10：生成 1024×1024 PNG 源 + tauri icon 全套

**Files:**
- Create: `assets/icon-source.svg`
- Create: `assets/icon-1024.png`
- Modify: `src-tauri/icons/*`（由 tauri icon 自动生成全套）

- [ ] **Step 1: 创建 `assets/icon-source.svg`**

```bash
mkdir -p /Users/fanjunjie/Documents/repositories/personal/apiproxy/assets
```

```svg
<svg viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg" width="1024" height="1024">
  <defs>
    <linearGradient id="g" x1="0" y1="0" x2="1024" y2="1024" gradientUnits="userSpaceOnUse">
      <stop offset="0%" stop-color="#0A84FF"/>
      <stop offset="100%" stop-color="#7C3AED"/>
    </linearGradient>
  </defs>
  <rect x="64" y="64" width="896" height="896" rx="224" fill="url(#g)"/>
  <g stroke="white" stroke-linecap="round" stroke-linejoin="round" fill="none">
    <path d="M352 352 L224 512 L352 672" stroke-width="64"/>
    <path d="M448 352 L320 512 L448 672" stroke-width="64" opacity="0.5"/>
    <path d="M704 320 L416 704" stroke-width="72" opacity="0.85"/>
    <path d="M672 352 L800 512 L672 672" stroke-width="64" opacity="0.5"/>
    <path d="M768 352 L896 512 L768 672" stroke-width="64"/>
  </g>
</svg>
```

写入 `/Users/fanjunjie/Documents/repositories/personal/apiproxy/assets/icon-source.svg`。

- [ ] **Step 2: 把 SVG 转换为 1024 PNG（用 Chrome DevTools MCP）**

如果环境里 `rsvg-convert` 或 ImageMagick `convert` 命令可用：
```bash
rsvg-convert -w 1024 -h 1024 \
  /Users/fanjunjie/Documents/repositories/personal/apiproxy/assets/icon-source.svg \
  -o /Users/fanjunjie/Documents/repositories/personal/apiproxy/assets/icon-1024.png
```

否则，用以下 Python 脚本（直接用 PIL 几何绘制，不依赖 SVG 渲染器）：

```bash
python3 << 'EOF'
from PIL import Image, ImageDraw

W = H = 1024
img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

# 圆角方形背景（蓝→紫渐变近似：先纯色，再叠对角线渐变）
import math
for y in range(H):
    for x in range(W):
        t = (x + y) / (W + H)
        r = int(0x0A * (1 - t) + 0x7C * t)
        g = int(0x84 * (1 - t) + 0x3A * t)
        b = int(0xFF * (1 - t) + 0xED * t)
        img.putpixel((x, y), (r, g, b, 255))

# 切出圆角
mask = Image.new("L", (W, H), 0)
md = ImageDraw.Draw(mask)
md.rounded_rectangle([(64, 64), (W - 64, H - 64)], radius=224, fill=255)
out = Image.new("RGBA", (W, H), (0, 0, 0, 0))
out.paste(img, (0, 0), mask)

d = ImageDraw.Draw(out)

def stroke_path(points, width, opacity=255):
    color = (255, 255, 255, opacity)
    d.line(points, fill=color, width=width, joint="curve")
    # 端点圆点
    half = width // 2
    for (x, y) in points:
        d.ellipse((x - half, y - half, x + half, y + half), fill=color)

# 五条线 (左 « + 中 / + 右 »)
stroke_path([(352, 352), (224, 512), (352, 672)], 64, 255)
stroke_path([(448, 352), (320, 512), (448, 672)], 64, 128)
stroke_path([(704, 320), (416, 704)],            72, 217)
stroke_path([(672, 352), (800, 512), (672, 672)], 64, 128)
stroke_path([(768, 352), (896, 512), (768, 672)], 64, 255)

out.save("/Users/fanjunjie/Documents/repositories/personal/apiproxy/assets/icon-1024.png")
print("OK 1024 PNG written")
EOF
```

预期：`assets/icon-1024.png` 1024×1024 文件生成成功。

- [ ] **Step 3: 用 tauri-cli 生成全套 icon**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy
pnpm tauri icon assets/icon-1024.png
```

预期：`src-tauri/icons/` 目录被覆盖，生成 32x32.png / 128x128.png / 128x128@2x.png / icon.icns / icon.ico / Square*.png / StoreLogo.png 等全套。

- [ ] **Step 4: 验证 cargo 仍能编译**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo build 2>&1 | tail -3
```

预期：成功（不需要重建大量东西，只是嵌入新 icon）。

- [ ] **Step 5: 提交**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy
git add -A && git commit -m "feat(brand): add app icon with «/» dual-arrow logo

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 7：清理参考项目命名残留

### Task 11：搜索并替换 cc-switch 字样

**Files:**
- 检查全部源代码与文档（排除 `docs/superpowers/`、`docs/superpowers/specs/`、`docs/superpowers/plans/` 中的设计文档）

- [ ] **Step 1: 搜索匹配项**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy
rg -i 'cc[-_ ]?switch|参考cc-switch|ccswitch' \
  --glob '!docs/superpowers/**' \
  --glob '!node_modules/**' \
  --glob '!target/**' \
  --glob '!pnpm-lock.yaml' \
  --glob '!dist/**' \
  -n 2>&1
```

预期输出：识别出所有需要清理的字串位置（应该非常少，可能 0 个，因为源代码里我们没引用过）。

- [ ] **Step 2: 如有命中，逐处替换为通用术语**

替换规则：
- `cc-switch` → 视上下文删除或改为 `参考实现` / `参考项目`
- `参考 cc-switch` / `参考cc-switch` → 删除该整句或改为 `参考实现`
- 文件名、类名、变量名包含 `cc-switch` 或 `ccswitch` 的：直接重命名（应该没有）

- [ ] **Step 3: 重新搜索确认零命中**

```bash
rg -i 'cc[-_ ]?switch|ccswitch' \
  --glob '!docs/superpowers/**' \
  --glob '!node_modules/**' \
  --glob '!target/**' \
  --glob '!pnpm-lock.yaml' \
  --glob '!dist/**' \
  2>&1 | wc -l
```

预期：`0`

- [ ] **Step 4: 提交（如有改动）**

```bash
git add -A
if ! git diff --cached --quiet; then
  git commit -m "chore: remove references to reference project from source code

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
fi
```

---

## Phase 8：验收

### Task 12：全量验证 + 手测

- [ ] **Step 1: 全量后端测试**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo test 2>&1 | tail -10
```

预期：38 个测试通过（29 原有 + 9 个 http_client 新测试）。

- [ ] **Step 2: 前端 typecheck**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy
pnpm typecheck 2>&1 | tail -3
```

预期：无错误。

- [ ] **Step 3: 前端构建**

```bash
pnpm build:renderer 2>&1 | tail -5
```

预期：构建成功。

- [ ] **Step 4: 启动 dev 模式**

```bash
pnpm tauri dev > /tmp/apiproxy-v2-dev.log 2>&1 &
sleep 30
```

- [ ] **Step 5: 手动验证清单**

```
□ 1. 窗口顶部：左 Logo+「API 代理」+齿轮，右 Switch+橙色 + 按钮，无其它文字
□ 2. 鼠标悬停 Switch，tooltip 显示「运行中 0.0.0.0:8118」
□ 3. App Dock/任务栏图标：蓝紫渐变圆角方+白色 «/»
□ 4. 点齿轮打开设置，滚动应能看到「出站代理」章节
□ 5. 点扫描按钮，如本机有 Clash/V2Ray 应出现端口按钮
□ 6. 输入一个无效端口的 socks5://127.0.0.1:9999，点测试，应 toast 提示失败
□ 7. 输入有效代理 → 保存 → toast「出站代理已应用」
□ 8. 清空 + 保存 → toast「已切换为直连」
□ 9. 添加一个 /cc → upstream，curl http://127.0.0.1:8118/cc/ 看到上游响应
□ 10. 全局 grep 没有 cc-switch 字串残留
```

- [ ] **Step 6: 杀掉 dev 进程**

```bash
pkill -f "target/debug/apiproxy" || true
pkill -f "tauri.js dev" || true
pkill -f "vite/bin/vite.js" || true
```

- [ ] **Step 7: 完成提交（手测如发现修复，每个修复一个提交）**

```bash
git status   # 应该 clean
```

---

## 完成

至此 v2 交付：
- ✅ 出站代理（HTTP/HTTPS/SOCKS5）支持
- ✅ 顶部精简（Logo + 标题 + 设置 / Switch + 添加）
- ✅ 添加按钮橙色圆形
- ✅ 设置面板内「出站代理」章节（URL/扫描/测试/清空/保存/用户名密码）
- ✅ `«/»` 双向箭头品牌 logo + 全套 App icon
- ✅ 源码无参考项目命名残留
