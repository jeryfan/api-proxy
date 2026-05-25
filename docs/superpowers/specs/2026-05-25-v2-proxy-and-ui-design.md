# v2 设计：全局出站代理 + 顶部重构 + Logo

**日期**：2026-05-25
**项目**：apiproxy（API 代理）
**v1 spec**：[2026-05-25-apiproxy-design.md](2026-05-25-apiproxy-design.md)

## 1. 目标

在已交付的 MVP 之上做四件事：

1. **全局出站代理**：让本工具发往上游 API 的请求可以经由用户配置的 HTTP/SOCKS 代理（典型场景：Clash / 公司 VPN）。
2. **顶部 Header 重构**：移除冗余信息，只保留必要按钮。
3. **logo 设计**：用 `«/»` 双向箭头符号设计一个简洁可辨的桌面图标 + 顶栏内嵌 SVG。
4. **清理代码内对参考项目的命名残留**：源码、注释、类名、字符串不应出现该名称。

## 2. 决策摘要

| 项 | 决策 |
|---|---|
| 出站代理协议 | http / https / socks5 / socks5h |
| 代理认证 | 编码在 URL 内（`user:pass@host:port`），前端拆分输入框 |
| 配置存储 | `GlobalConfig.proxyUrl: string`（默认 `""` 表示直连） |
| 客户端切换 | 后端持有 `OnceCell<RwLock<reqwest::Client>>` 全局客户端，切换代理时热更新 |
| ProxyManager 改造 | 删除自有 client，每请求时从 `http_client::get()` 取最新 |
| 出站代理 UI 入口 | 在 Settings 面板新增「出站代理」章节 |
| 端口扫描清单 | 7890, 7891, 1080, 8080, 8888, 3128, 10808, 10809（含 mixed 模式同时给 http/socks5） |
| 测试目标 URL | `https://httpbin.org/get` → 失败回退 `https://www.google.com` → `https://api.anthropic.com` |
| 顶部布局 | 左侧 Logo + 标题 + Settings；右侧 服务 Switch + 添加按钮 |
| 顶部去除 | 状态文字徽章、监听地址显示、主题切换按钮 |
| 主题切换迁移 | 进入 Settings 面板的「外观」章节（已有，删顶部即可） |
| 添加按钮配色 | 橙色圆形：`bg-orange-500 hover:bg-orange-600 text-white shadow-lg shadow-orange-500/30 rounded-full w-8 h-8` |
| Logo 主体 | `«/»` 双向箭头，蓝紫渐变圆角方块 + 白色描边 |
| Logo 交付 | SVG 源 + 1024/512/256/128/64/32 多档 PNG |

## 3. 后端架构

### 3.1 新增模块 `src-tauri/src/proxy/http_client.rs`

```rust
use std::sync::{OnceLock, RwLock};
use std::time::Duration;
use reqwest::Client;
use url::Url;

static GLOBAL_CLIENT: OnceLock<RwLock<Client>> = OnceLock::new();
static CURRENT_PROXY_URL: OnceLock<RwLock<Option<String>>> = OnceLock::new();

pub fn init(proxy_url: Option<&str>) -> Result<(), String>;
pub fn validate_proxy(proxy_url: Option<&str>) -> Result<(), String>;
pub fn apply_proxy(proxy_url: Option<&str>) -> Result<(), String>;
pub fn get() -> Client;
pub fn get_current_proxy_url() -> Option<String>;
pub fn mask_url(url: &str) -> String;            // 用于日志，把 user:pass 替换为 user:***
fn build_client(proxy_url: Option<&str>) -> Result<Client, String>;
```

`build_client` 规则：
- `proxy_url` 为 `None`/空 → 用 reqwest 默认（跟随系统环境变量代理）
- 非空 → `url::Url::parse` 校验 + 限定 scheme ∈ {http, https, socks5, socks5h} + `reqwest::Proxy::all(url)` 构造
- 通用配置：`timeout(600s) connect_timeout(30s) pool_max_idle_per_host(10) tcp_keepalive(60s)`

注：reqwest 需要开启 `socks` feature 才支持 socks5。Cargo.toml 现在的 `["rustls-tls", "stream", "json"]` 需要加 `"socks"`。

### 3.2 GlobalConfig 字段增补

```rust
pub struct GlobalConfig {
    // ... 原字段
    pub proxy_url: String,           // 新增；默认 ""
}
```

### 3.3 ProxyManager 改造

- 删除字段 `client: reqwest::Client`
- `handler.rs` 调用 `crate::proxy::http_client::get()` 获取当前快照 client
- ProxyManager 启停逻辑不变；代理切换不重启 server

### 3.4 新增 Tauri commands

文件：`src-tauri/src/commands.rs`

```rust
#[tauri::command]
pub fn get_global_proxy_url(state: State<'_, AppState>) -> String;

#[tauri::command]
pub async fn set_global_proxy_url(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    url: String,
) -> Result<(), String>;
//  顺序：validate -> 写 store -> apply

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyTestResult {
    pub success: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn test_proxy_url(url: String) -> Result<ProxyTestResult, String>;
//  建临时 client 经代理 HEAD 多个测试 URL，任一成功即成功

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedProxy {
    pub url: String,
    pub proxy_type: String,  // "http" / "socks5"
    pub port: u16,
}

#[tauri::command]
pub async fn scan_local_proxies() -> Vec<DetectedProxy>;
//  tokio::spawn_blocking 中 TCP connect_timeout 100ms 扫常见端口
```

### 3.5 启动流程修改

`lib.rs` `setup` 中：
```rust
let g = store.global();
http_client::init(if g.proxy_url.is_empty() { None } else { Some(&g.proxy_url) })?;
```

### 3.6 测试

新增到 `proxy/transform.rs` 同级或自己模块：
- `validate_proxy_accepts_http` / `validate_proxy_accepts_socks5` / `validate_proxy_rejects_ftp` / `validate_proxy_rejects_bad_url`
- `mask_url_strips_password`

集成测试 `tests/proxy_global_test.rs`：起一个本地 fake HTTP CONNECT 代理（用 [`tokio::net::TcpListener`] 手写 CONNECT handshake），验证 `reqwest::Proxy::all(this_proxy)` 能把请求经它送出。

## 4. 前端

### 4.1 `src/types.ts` 增补

```ts
interface GlobalConfig {
  // ...
  proxyUrl: string;
}

export interface ProxyTestResult { success: boolean; latencyMs: number; error?: string; }
export interface DetectedProxy { url: string; proxyType: string; port: number; }
```

### 4.2 `src/lib/api.ts` 增补

```ts
getGlobalProxyUrl: () => invoke<string>("get_global_proxy_url"),
setGlobalProxyUrl: (url: string) => invoke<void>("set_global_proxy_url", { url }),
testProxyUrl: (url: string) => invoke<ProxyTestResult>("test_proxy_url", { url }),
scanLocalProxies: () => invoke<DetectedProxy[]>("scan_local_proxies"),
```

### 4.3 `SettingsPanel.tsx` 新增「出站代理」章节

放在「代理监听」之下、「行为」之上，内部三行布局：

```
┌─ 出站代理 ───────────────────────────────────────────────────┐
│ 此设置会让本工具发往上游 API 的请求经由所配置的 HTTP / SOCKS 代理。 │
│ ┌───────────────────────────────┬──┬──┬──┬──┐               │
│ │ http://127.0.0.1:7890 / ... │🔍│🧪│✕ │保存│              │
│ └───────────────────────────────┴──┴──┴──┴──┘               │
│ ┌──────────────┬──────────────────────┐                     │
│ │ 用户名（可选） │ 密码（可选）  [👁]    │                     │
│ └──────────────┴──────────────────────┘                     │
│ ┌─ 扫描结果 (如有) ──────────────────────────┐               │
│ │ [http://127.0.0.1:7890] [socks5://...]      │               │
│ └────────────────────────────────────────────┘               │
└────────────────────────────────────────────────────────────┘
```

行为：
- 输入 URL → 状态变 dirty → 保存按钮可用
- 点扫描 → `scan_local_proxies` → 渲染结果按钮，点一下填入 URL
- 点测试 → `test_proxy_url(fullUrl)` → toast 显示 `延迟 X ms` 或 `失败：...`
- 点清除 → 三个 input 都清空，dirty = true
- 点保存 → `set_global_proxy_url(fullUrl)`，成功 toast 「已应用」/「已切回直连」

实现成独立组件 `src/components/settings/GlobalProxySettings.tsx`，`SettingsPanel.tsx` 引用之。

### 4.4 顶部 Header 重构

`App.tsx` 顶部区域改为：

```tsx
<header className="fixed left-0 right-0 z-50 bg-background/80 backdrop-blur-md"
        style={{ top: DRAG_BAR, height: HEADER_H }}
        data-tauri-drag-region>
  <div className="flex h-full items-center justify-between gap-2 px-6">
    {/* 左侧：Logo + 标题 + Settings */}
    <div className="flex items-center gap-2" data-tauri-no-drag>
      <BrandLogo className="h-7 w-7" />
      <span className="text-xl font-semibold text-blue-500 dark:text-blue-400">
        API 代理
      </span>
      <Button variant="ghost" size="icon" className="h-8 w-8"
              onClick={() => setShowSettings(true)} title="设置">
        <SettingsIcon className="h-4 w-4" />
      </Button>
    </div>

    {/* 右侧：服务 Switch + 添加按钮 */}
    <div className="flex items-center gap-3" data-tauri-no-drag>
      <ServerToggle status={status} />
      <Button onClick={() => setShowAdd(true)} size="icon"
              className="ml-2 bg-orange-500 hover:bg-orange-600 dark:bg-orange-500 dark:hover:bg-orange-600 text-white shadow-lg shadow-orange-500/30 dark:shadow-orange-500/40 rounded-full w-8 h-8">
        <Plus className="w-5 h-5" />
      </Button>
    </div>
  </div>
</header>
```

被移除：`ServerStatusBadge`、`ModeToggle`、对监听地址的文本展示。

### 4.5 `ServerToggle` 简化

- 删掉外面的「服务」label
- 在 Switch 自身添加 tooltip（`title` 属性）：`运行中 0.0.0.0:8118` / `已停止`

`ServerStatusBadge.tsx` 删除。

### 4.6 `ModeToggle` 处理

- 顶部不再使用 ModeToggle
- 文件保留备用，但不在 App 引入
- Settings 面板「外观」章节已经有主题 Select，作为主入口

### 4.7 BrandLogo 组件

`src/components/BrandLogo.tsx`：纯 SVG React 组件，接受 `className`：

```tsx
export function BrandLogo({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg"
         className={className}>
      <rect x="2" y="2" width="28" height="28" rx="7" fill="url(#brand-g)"/>
      <defs>
        <linearGradient id="brand-g" x1="0" y1="0" x2="32" y2="32">
          <stop offset="0%" stopColor="#0A84FF"/>
          <stop offset="100%" stopColor="#7C3AED"/>
        </linearGradient>
      </defs>
      <path d="M11 11 L7 16 L11 21" stroke="white" strokeWidth="2"
            strokeLinecap="round" strokeLinejoin="round" fill="none"/>
      <path d="M14 11 L10 16 L14 21" stroke="white" strokeWidth="2"
            strokeLinecap="round" strokeLinejoin="round" fill="none" opacity="0.5"/>
      <path d="M22 10 L13 22" stroke="white" strokeWidth="2.2"
            strokeLinecap="round" opacity="0.85"/>
      <path d="M21 11 L25 16 L21 21" stroke="white" strokeWidth="2"
            strokeLinecap="round" strokeLinejoin="round" fill="none" opacity="0.5"/>
      <path d="M24 11 L28 16 L24 21" stroke="white" strokeWidth="2"
            strokeLinecap="round" strokeLinejoin="round" fill="none"/>
    </svg>
  );
}
```

## 5. App 图标

`src-tauri/icons/`：

- 用 Python（PIL）或纯 Rust 写一个生成脚本，把 SVG 渲染为：
  - `icon.png` 1024×1024
  - `icon@2x.png` 2048×2048（可选）
  - `32x32.png` / `128x128.png` / `128x128@2x.png` / `Square*.png` / `StoreLogo.png`（Windows/macOS Tauri 标准）
- 使用 `@tauri-apps/cli` 的 `tauri icon` 命令最简单：`pnpm tauri icon path/to/source-1024.png` 自动生成全套
- 我们生成一个 1024×1024 PNG 源（参照 BrandLogo 的几何）然后用上述命令

落地步骤：
1. 用一段 Python 把 SVG 渲染成 1024×1024 PNG（用 `cairosvg` 或简单地用纯几何写）
2. 跑 `pnpm tauri icon` 生成全套

## 6. 清理参考项目命名

执行：
```
rg -i "cc-switch|cc_switch|ccswitch" --type-add 'rs:*.rs' -t rs -t ts -t tsx -t css -t json -t toml \
  --glob '!docs/**' --glob '!CHANGELOG*' --glob '!node_modules/**' .
```

修复每一处。注释/字符串/类名/包名都要看。

排除：`docs/superpowers/` 下的历史 spec 可以保留（明确为「参考的项目」上下文）。

## 7. 数据迁移

`GlobalConfig` 增字段 `proxyUrl`，对老配置文件做向后兼容：`#[serde(default)]` 让旧配置文件加载时默认空字符串，无需破坏。

## 8. 风险

- reqwest 加 `socks` feature 会拉 `socks` crate → bundle 体积 +约 200KB（可接受）
- `http_client` 模块改 ProxyManager 接口；handler 每请求都 `get()` 拷贝 client（reqwest::Client 内部是 `Arc`，clone 廉价，不是性能问题）
- 顶部去掉状态文字后，"服务正在运行 0.0.0.0:8118" 信息要么去 Settings 面板看，要么 tooltip。tooltip 不够发现，但已经够用（如有反馈再添）

## 9. 非目标

- 系统代理自动检测（macOS scutil / Windows registry）— 仅做 TCP 端口扫描
- 按规则的智能分流（per-endpoint 配独立代理）— 全局一套就够
- 代理协议鉴权除 user/pass 外的（如 socks5 password、TLS 客户端证书）

## 10. 验收清单

- [ ] 设置面板可输入代理 URL，扫描出本机 Clash 端口
- [ ] 测试按钮显示延迟数字
- [ ] 保存后，curl 经本工具发的请求经过代理（用 mitmproxy/wireshark 验证）
- [ ] 清除按钮 + 保存 = 切回直连，下次请求直接走默认
- [ ] 顶部干净：左 Logo+标题+设置，右 Switch+添加
- [ ] App dock 图标显示 `«/»` 蓝紫渐变
- [ ] `rg cc-switch -t rs -t ts -t tsx -t css -t json --glob '!docs/**'` 无命中
- [ ] 后端 29 + 新增 ~6 个测试全部通过
- [ ] `pnpm typecheck` 通过
- [ ] `pnpm tauri dev` 启动成功
