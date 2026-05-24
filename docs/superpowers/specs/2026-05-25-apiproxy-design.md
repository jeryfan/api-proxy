# API 代理工具设计文档

**日期**：2026-05-25
**项目**：apiproxy（中文显示名「API 代理」）

## 1. 目标

一个本地 Tauri 桌面工具，作为「单端口 + 路径前缀路由」的 HTTP 反向代理：

- 用户在 UI 中配置端点（path → upstreamUrl），收到 `/cc/...` 请求转发到 `https://api.moonshot.com/v1/...`。
- 支持对请求头、查询参数、JSON 请求体进行 set/add/remove 类型的修改。
- 响应原样透传（含 SSE 流式）。
- 类 cc-switch 视觉风格，中文界面，无多语言。

## 2. 技术栈

| 层 | 选型 |
|---|---|
| 桌面壳 | Tauri 2 |
| 后端 (Rust) | axum 0.7、reqwest 0.12、tokio、serde、serde_json、arc-swap、uuid、url |
| Tauri 插件 | tauri-plugin-store、tauri-plugin-single-instance、tauri-plugin-clipboard-manager、tauri-plugin-dialog |
| 前端框架 | React 18 + TypeScript + Vite 8 |
| 包管理器 | pnpm |
| 样式 | Tailwind CSS 3.4（照搬 cc-switch 主题）+ shadcn 风格组件 |
| 表单 | react-hook-form + zod |
| 动画 | framer-motion |
| 状态管理 | 本地 useState + Tauri events；不引 redux/zustand |
| HTTP 调用前端→后端 | `@tauri-apps/api` invoke |
| 图标 | lucide-react |
| Toast | sonner |

## 3. 关键设计决策（已与用户确认）

| 决策 | 选择 |
|---|---|
| 监听模型 | 单端口 + 路径前缀路由 |
| 请求体修改 | JSON 字段合并/覆盖（非 JSON 跳过） |
| 响应处理 | 完全不修改（含响应头） |
| 协议支持 | HTTP + SSE（不支持 WebSocket） |
| 服务启停粒度 | 全局服务开关 + 端点级别启用标记 |
| 日志/活动记录 | 不记录 |
| 默认监听地址 | `0.0.0.0` |
| 系统托盘 | 需要 |
| UI 主题 | light / dark / system 三档 |
| 变量插值 | 不支持 |
| path 冲突 | 保存时报错 |
| 设置页 | 独立全屏面板 |
| 路径前缀处理 | 默认剥离（端点可关闭） |
| 配置存储 | Tauri Store 插件，明文 JSON |
| 表单字段 | 必选字段 + 端点描述（其他暂不做） |
| 启动时服务行为 | 总是自动启动 |
| 后端代理框架 | axum + reqwest |
| 配置变更策略 | 端点 CRUD 热应用，仅监听端口/地址变化时优雅重启 |

## 4. 项目目录结构

```
apiproxy/
├─ package.json
├─ pnpm-lock.yaml
├─ pnpm-workspace.yaml
├─ tsconfig.json
├─ tsconfig.node.json
├─ vite.config.ts            # root: "src", outDir: "../dist"
├─ tailwind.config.cjs       # 照搬 cc-switch
├─ postcss.config.cjs
├─ rust-toolchain.toml
├─ .gitignore
├─ .node-version
├─ README.md
├─ docs/superpowers/specs/2026-05-25-apiproxy-design.md
├─ src/
│  ├─ index.html
│  ├─ index.css
│  ├─ main.tsx
│  ├─ App.tsx
│  ├─ types.ts
│  ├─ lib/
│  │  ├─ utils.ts
│  │  ├─ platform.ts
│  │  ├─ api.ts             # invoke 封装
│  │  └─ schemas.ts         # zod schemas
│  ├─ components/
│  │  ├─ ui/                # 通用基础组件（button/input/textarea/dialog/select/switch/label/form/badge/sonner）
│  │  ├─ common/
│  │  │  └─ FullScreenPanel.tsx
│  │  ├─ endpoints/
│  │  │  ├─ EndpointList.tsx
│  │  │  ├─ EndpointCard.tsx
│  │  │  ├─ EndpointActions.tsx
│  │  │  ├─ AddEndpointDialog.tsx
│  │  │  ├─ EditEndpointDialog.tsx
│  │  │  ├─ EndpointEmptyState.tsx
│  │  │  └─ forms/
│  │  │     ├─ EndpointForm.tsx
│  │  │     ├─ RulesField.tsx       # HeaderRule / QueryRule 通用
│  │  │     └─ BodyMergeField.tsx
│  │  ├─ settings/
│  │  │  └─ SettingsPanel.tsx
│  │  ├─ ServerToggle.tsx
│  │  ├─ ServerStatusBadge.tsx
│  │  ├─ theme-provider.tsx
│  │  ├─ mode-toggle.tsx
│  │  └─ ConfirmDialog.tsx
│  └─ assets/
└─ src-tauri/
   ├─ Cargo.toml
   ├─ tauri.conf.json
   ├─ build.rs
   ├─ icons/
   ├─ capabilities/default.json
   └─ src/
      ├─ main.rs
      ├─ lib.rs
      ├─ commands.rs
      ├─ config/
      │  ├─ mod.rs           # 类型与校验
      │  └─ store.rs         # Tauri Store 读写封装
      ├─ proxy/
      │  ├─ mod.rs           # ProxyManager（启停/热切换）
      │  ├─ server.rs        # axum app 构建 & serve
      │  ├─ handler.rs       # 通用反向代理 handler
      │  └─ transform.rs     # 请求修改纯函数（高度可测）
      ├─ tray.rs
      └─ events.rs           # 事件名常量
```

## 5. 数据模型

### 5.1 Endpoint（端点配置，持久化）

```typescript
type RuleAction = 'set' | 'add' | 'remove';

interface HeaderRule { action: RuleAction; key: string; value: string; }
interface QueryRule  { action: RuleAction; key: string; value: string; }

interface Endpoint {
  id: string;                 // UUID v4
  name: string;               // 显示名，必填，≤ 60 字符
  description?: string;       // 可选副标题
  enabled: boolean;
  path: string;               // 以 "/" 开头，正则 ^/[A-Za-z0-9._\-/]+$；不能为 "/"
  upstreamUrl: string;        // 必须是有效 URL，scheme ∈ {http, https}
  stripPrefix: boolean;       // 默认 true
  headerRules: HeaderRule[];
  queryRules: QueryRule[];
  bodyMerge?: string;         // 合法 JSON 对象的字符串；非空时启用
  createdAt: number;
  updatedAt: number;
}
```

`RuleAction` 语义：

- `set`：不存在则添加，存在则覆盖（单值替换）
- `add`：保留原值并追加（用于多值场景）
- `remove`：删除该 key（value 字段忽略）

### 5.2 GlobalConfig（全局配置，持久化）

```typescript
interface GlobalConfig {
  listenAddress: string;       // 默认 "0.0.0.0"
  listenPort: number;          // 默认 8118，1..=65535
  requestTimeoutSecs: number;  // 默认 60，1..=600
  theme: 'light' | 'dark' | 'system';  // 默认 'system'
  closeToTray: boolean;        // 默认 true
  autoStartServer: boolean;    // 默认 true
}
```

### 5.3 ServerStatus（运行时状态，不持久化）

```typescript
interface ServerStatus {
  running: boolean;
  listenAddress?: string;
  listenPort?: number;
  startedAt?: number;
  lastError?: string;
}
```

### 5.4 Store 文件 `config.json` 结构

```json
{
  "global": { ... },
  "endpoints": [ ... ]
}
```

存放位置由 Tauri Store 决定（macOS `~/Library/Application Support/com.apiproxy.app/`，
Windows `%APPDATA%\com.apiproxy.app\`，Linux `~/.config/com.apiproxy.app/`）。

## 6. 后端架构

### 6.1 ProxyManager

```rust
pub struct ProxyManager {
    routes: Arc<ArcSwap<Vec<Endpoint>>>,   // 热切换路由表
    timeout_secs: Arc<AtomicU64>,          // 热切换超时
    handle: Mutex<Option<ServerHandle>>,   // 当前运行的 server
    http_client: reqwest::Client,
}

struct ServerHandle {
    shutdown: tokio::sync::oneshot::Sender<()>,
    join: tokio::task::JoinHandle<()>,
    address: String,
    port: u16,
}
```

API：
- `start(address, port) -> Result<(), ProxyError>` — 绑定 listener，启动 server
- `stop() -> Result<(), ProxyError>` — graceful shutdown
- `replace_routes(endpoints)` — 原子替换路由表
- `replace_timeout(secs)` — 替换超时
- `status() -> ServerStatus`

### 6.2 请求生命周期

1. axum 收到任意方法/路径的请求 `Req`
2. 读 `routes.load_full()` 取当前路由表快照
3. 路由匹配：在所有 `enabled === true` 的端点中，按 `endpoint.path` 长度**降序**找第一个满足以下条件的端点：
   - `req.uri.path()` 等于 `endpoint.path`，**或**
   - `req.uri.path()` 以 `endpoint.path + "/"` 开头
   这样可以避免 `/cc` 匹配到 `/ccfoo`。
   未匹配 → `404 {"error": "no endpoint matched", "path": "..."}`
4. 构造上游 URL：
   - `base = parse(endpoint.upstreamUrl)`
   - `remaining = if endpoint.stripPrefix { req.path.strip_prefix(endpoint.path) } else { req.path }`
   - 处理空尾路径：剥离后为空时不附加任何 path（直接用 base.path），保留 base 自带 path
   - `upstream_url = base.join_segment(remaining)`
   - 把 `req.uri.query()` 透传挂上
5. 应用 `queryRules`：在 upstream_url 的 query 上做 set/add/remove
6. 拷贝请求头 → 移除 hop-by-hop（Connection、Keep-Alive、Proxy-*、Transfer-Encoding、TE、Trailer、Upgrade、Proxy-Authorization、Proxy-Authenticate）→ 覆盖 `Host` 为 upstream host
7. 应用 `headerRules`
8. 处理请求体：
   - `bodyMerge` 为空 → reqwest 端用 stream body 直接转发
   - `bodyMerge` 非空：
     - 读全 body 字节
     - body 非空且 `Content-Type` 含 `application/json`：解析为 `serde_json::Value`，与 `bodyMerge` 做**深合并**（patch 覆盖原值；对象字段递归合并；数组直接替换）
     - 重新序列化为 JSON bytes，更新 `Content-Length`
     - 不是 JSON 或解析失败 → 原样转发并加响应头 `X-ApiProxy-Warn: body-merge-skipped`
9. `reqwest::Client::request(...).timeout(...).send().await`
10. 透传 Response：status、headers（去 hop-by-hop）、body 使用 `bytes_stream()` 转 axum `Body::from_stream`
11. 错误：
    - 连接失败/超时 → `502 {"error": "...", "code": "UPSTREAM_TIMEOUT" | "UPSTREAM_CONNECT" | "UPSTREAM_TLS"}`

### 6.3 关键纯函数（在 `proxy/transform.rs`）

```rust
pub fn match_endpoint<'a>(
    routes: &'a [Endpoint],
    req_path: &str,
) -> Option<&'a Endpoint>;

pub fn build_upstream_url(
    endpoint: &Endpoint,
    req_path: &str,
    req_query: Option<&str>,
) -> Result<Url, TransformError>;

pub fn apply_query_rules(url: &mut Url, rules: &[QueryRule]);

pub fn apply_header_rules(headers: &mut HeaderMap, rules: &[HeaderRule]);

pub fn strip_hop_by_hop(headers: &mut HeaderMap);

pub fn merge_json_body(
    original: &[u8],
    patch: &serde_json::Value,
) -> Result<Vec<u8>, TransformError>;
```

这些函数无 IO、无异步，单元测试覆盖。

### 6.4 Tauri Commands

```rust
#[tauri::command] fn init_data(state) -> InitPayload;    // { global, endpoints, status }
#[tauri::command] fn list_endpoints(state) -> Vec<Endpoint>;
#[tauri::command] fn save_endpoint(state, endpoint: Endpoint) -> Result<Endpoint, String>;
#[tauri::command] fn delete_endpoint(state, id: String) -> Result<(), String>;
#[tauri::command] fn toggle_endpoint(state, id: String, enabled: bool) -> Result<(), String>;
#[tauri::command] fn get_global_config(state) -> GlobalConfig;
#[tauri::command] fn apply_global_config(state, app, config: GlobalConfig) -> Result<ServerStatus, String>;
#[tauri::command] fn server_status(state) -> ServerStatus;
#[tauri::command] fn start_server(state, app) -> Result<ServerStatus, String>;
#[tauri::command] fn stop_server(state) -> Result<ServerStatus, String>;
#[tauri::command] fn open_config_dir(app) -> Result<(), String>;
```

### 6.5 事件（后端 emit → 前端订阅）

```
proxy://status-changed    -> ServerStatus
proxy://config-error      -> { message: string }
proxy://endpoints-changed -> Vec<Endpoint>
```

### 6.6 校验规则（前后端均执行）

- `name`：非空，≤ 60 字符
- `path`：以 `/` 开头，正则 `^/[A-Za-z0-9._\-/]+$`，不能等于 `/`；保存时归一化：去掉末尾的 `/`（除非 path 本身是 `/`，但这被禁止），相邻 `//` 合并为单个 `/`
- `upstreamUrl`：可解析为 `Url`，scheme 在 `{http, https}` 内
- `bodyMerge`：若非空必须解析为 `serde_json::Value::Object`
- 全局校验：所有 `enabled === true` 的端点 path 唯一；冲突时报错并指出冲突对象

## 7. 前端架构

### 7.1 顶部 Header（fixed, h-64, backdrop-blur）

| 区位 | 内容 |
|---|---|
| 最左 | 标题「API 代理」（`text-xl font-semibold text-blue-500`），非首页变 `<返回 + 子页标题>` |
| 标题右 | 设置按钮（Settings 图标） |
| 中部偏右 | `ServerStatusBadge`（运行中/已停止）+ 监听地址显示（如 `0.0.0.0:8118`，点击复制） |
| 右侧 | `ServerToggle`（全局服务开关 Switch）+ `ModeToggle`（主题切换） + `添加` 按钮（Plus 图标） |

### 7.2 主区

- 首页：`EndpointList`
  - 容器：`<main class="flex-1 ... animate-fade-in"> <div class="mt-4 space-y-4 px-6"> ... </div> </main>`
  - 列表为空 → `EndpointEmptyState`（虚线圆框 + Route 图标 + 创建按钮）
  - 列表非空 → 每个端点一张 `EndpointCard`

### 7.3 EndpointCard 视觉

```
┌────────────────────────────────────────────────────────────────────────┐
│ [⋮⋮] [📦Route]  端点名称  [已启用]               [▶/⏸][📋][✏][🗑]│
│              /cc → https://api.moonshot.com/v1                        │
│              描述（如有，灰色小字）                                     │
└────────────────────────────────────────────────────────────────────────┘
```

- 容器类（照搬 ProviderCard）：`relative overflow-hidden rounded-xl border p-4 transition-all bg-card group hover:border-border-active hover:shadow-sm`
- 启用状态：左边加 emerald 渐变光晕层
- 拖拽柄占位（GripVertical）— **本期不实现拖拽排序**，仅视觉占位（便于将来补）
- 图标方块：`h-8 w-8 rounded-lg bg-muted` 内放 lucide `Route` 图标 20px
- 标题：`text-base font-semibold leading-none` + 状态徽章（启用 violet/停用 slate；服务未运行时再加灰色 `服务未运行` 徽章）
- 副标题（mono）：`<span class="text-blue-500">{path}</span>` + ` → ` + `<span class="truncate">{upstreamUrl}</span>`
- 描述行：`text-sm text-muted-foreground`（无描述时不渲染）
- 右侧操作按钮组（`group-hover:opacity-100`，默认隐藏）：
  - 启用/停用切换（Play / Pause）
  - 复制端点对外 URL（点击复制 `http://localhost:8118/cc`）
  - 编辑
  - 删除（hover 红色，触发 ConfirmDialog）

### 7.4 添加 / 编辑端点（FullScreenPanel）

`EndpointForm` 用 react-hook-form + zodResolver，分章节：

1. **基本信息**
   - 名称（Input，必填）
   - 描述（Textarea，可选）
   - 启用（Switch，默认 true）
2. **路由**
   - 路径（Input，前面 readonly 显示 `/`）
   - 上游地址（Input，placeholder `https://api.moonshot.com/v1`）
   - 剥离路径前缀（Switch，默认 true）
3. **请求头规则**（`RulesField` 复用）
   - 动态数组（react-hook-form `useFieldArray`），每行：
     - Action（Select：set / add / remove）
     - Key（Input）
     - Value（Input；action=remove 时禁用并淡化）
     - 删除按钮（Trash2）
   - 底部「+ 添加规则」按钮
4. **查询参数规则**（`RulesField` 同上）
5. **请求体合并 (JSON)**
   - Textarea（`min-h-[160px] font-mono text-sm`）
   - 实时 zod 校验：必须为对象；错误时输入框红边 + 下方 `text-destructive text-sm` 错误说明

Footer：
- 添加场景：`[取消] [+ 添加]`
- 编辑场景：`[取消] [💾 保存]`

提交逻辑：
1. zod 通过
2. `await invoke('save_endpoint', { endpoint })`
3. 成功 → 关面板 + toast 「已保存」
4. 失败（如 path 冲突）→ 不关面板 + toast 错误

### 7.5 设置面板（FullScreenPanel）

字段：
- 监听地址（Input，默认 `0.0.0.0`）
- 监听端口（NumberInput，1-65535）
- 请求超时秒数（NumberInput，1-600）
- 关闭窗口最小化到托盘（Switch）
- 启动时自动启动代理（Switch）
- 主题（分段控件：light / dark / system）

Footer：`[取消] [应用]`。

应用逻辑：
1. 主题改动立即生效（写 localStorage、改 html class）
2. 其余字段调 `apply_global_config`
3. 端口/地址改动 → 后端优雅重启 server；UI 显示 `服务已重启于 X:Y` toast
4. 端口绑定失败 → 旧 listener 保持，UI 显示错误

### 7.6 全局服务开关交互

`ServerToggle`（Switch）：
- on → `invoke('start_server')`，失败 toast 错误
- off → `invoke('stop_server')`
- 后端 `proxy://status-changed` 事件订阅以保持状态同步

### 7.7 Toast / 通知

sonner 的 Toaster，位置 `top-center`、richColors、duration 2000ms。

### 7.8 Tauri 事件订阅

`App.tsx` mount 时挂三个 listener：
- `proxy://status-changed` → 更新本地状态
- `proxy://endpoints-changed` → React 端刷新（如另一处改了）
- `proxy://config-error` → toast 错误

## 8. 托盘与窗口

### 8.1 Tauri 窗口配置

```json
{
  "title": "",
  "titleBarStyle": "Overlay",
  "width": 1000,
  "height": 650,
  "minWidth": 900,
  "minHeight": 600,
  "visible": false,
  "resizable": true,
  "center": true
}
```

前端 ready 后通过 `getCurrentWindow().show()` 显示（无白屏闪烁）。

### 8.2 托盘菜单

| 项 | 行为 |
|---|---|
| `🟢 运行中：0.0.0.0:8118` / `⚫ 已停止`（disabled 显示） | — |
| `启动代理` / `停止代理`（根据状态切换） | `start_server` / `stop_server` |
| `显示主窗口` | `show().setFocus()` |
| ──分隔── | |
| `退出` | `app.exit(0)` |

托盘单击：macOS 弹菜单（标准）；Windows/Linux 单击 = 显示主窗口。

### 8.3 启动流程

```
1. tauri-plugin-single-instance 检查（已运行则 focus）
2. 加载 GlobalConfig & Endpoints
3. 创建 ProxyManager
4. 若 autoStartServer → ProxyManager.start()，失败记录 lastError 不阻塞
5. 创建托盘
6. 注册命令、事件
7. 显示窗口（前端 ready 后）
```

### 8.4 关闭按钮

若 `global.closeToTray === true` → 拦截 close 改为 `hide`。
否则正常退出（同时 `manager.stop()`）。

## 9. 错误处理

### 9.1 错误类型

```rust
pub enum ConfigError {
    InvalidPath(String),
    InvalidUrl(String),
    InvalidBodyMerge(String),
    DuplicatePath { conflict_with: String },
    NotFound(String),
    Io(String),
}

pub enum ProxyError {
    Bind { address: String, port: u16, message: String },
    NotRunning,
    AlreadyRunning,
    Upstream(String),
}
```

均实现 `Display`，命令返回 `Result<T, String>` 给前端。

### 9.2 代理响应错误格式

```json
{
  "error": "human-readable description",
  "code": "NO_ENDPOINT_MATCHED | UPSTREAM_TIMEOUT | UPSTREAM_CONNECT | UPSTREAM_TLS | BODY_MERGE_FAILED",
  "endpoint": "/cc",
  "upstream": "https://api.moonshot.com/v1"
}
```

## 10. 测试

### 10.1 后端单元测试（`proxy/transform.rs` 同文件 `#[cfg(test)] mod tests`）

覆盖：
- `match_endpoint`：单一匹配、长前缀优先、不匹配、禁用端点忽略
- `build_upstream_url`：剥离/不剥离、查询透传、嵌套子路径
- `apply_query_rules`：set/add/remove 各场景
- `apply_header_rules`：同上
- `strip_hop_by_hop`：完整列表清理
- `merge_json_body`：对象深合并、数组替换、非 JSON 跳过、空 body

### 10.2 后端集成测试（`src-tauri/tests/proxy_integration.rs`）

- 启动一个 fake upstream（`hyper` 直接起一个简单 server 或 `httpmock`）
- 启动 ProxyManager 在随机端口
- 验证：基础转发、SSE 流式响应、超时、未匹配 404、headerRules 生效、JSON merge 生效

### 10.3 前端

只跑 `pnpm typecheck`，本期不写组件测试。

### 10.4 手动验证

- macOS 下打开应用、看到主页（暂为空状态）
- 点击「+ 添加」创建一个端点（path=/cc，upstream=https://api.moonshot.com/v1）
- 顶部全局开关绿色，监听地址显示 `0.0.0.0:8118`
- 在终端 `curl http://localhost:8118/cc/foo`，看到 upstream 实际响应或错误
- 关闭窗口 → 托盘存在；托盘菜单可启停服务

## 11. 构建与发布

- `pnpm install`
- 开发：`pnpm tauri dev`
- 打包：`pnpm tauri build` 产出 macOS `.dmg` / Windows `.msi` / Linux `.AppImage` + `.deb`
- 不集成 updater（保留极简）

## 12. 非目标（明确不做）

- 多语言（仅中文）
- 拖拽排序（卡片仅保留拖拽柄占位以备将来）
- 持久化请求日志/活动记录
- 响应修改
- WebSocket 升级转发
- 端点级独立监听端口
- 变量插值（如 `${env.X}`、`${req.header.X}`）
- API key 写入系统 keychain
- TLS 证书校验跳过开关
- 多语言、updater、CI、e2e 测试

## 13. 后续可扩展点

- 拖拽排序（已留 GripVertical 占位）
- 请求活动历史（内存或 SQLite）
- 端点级别请求超时覆盖全局
- HTTP 方法白名单过滤
- 上游连接池调整
- mTLS 客户端证书
- 多端点路径冲突自动建议改名
