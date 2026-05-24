# API 代理工具 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建一个 Tauri 2 + Rust + React 桌面工具，以「单端口 + 路径前缀路由」的反向代理形式转发 HTTP/SSE 请求，支持请求头/查询参数/JSON 请求体修改。

**Architecture:**
- Rust 后端：axum 0.7（监听本地端口）+ reqwest 0.12（请求上游），用 `ArcSwap<Vec<Endpoint>>` 热切换路由表；ProxyManager 仅在监听端口/地址变化时优雅重启 listener。
- 前端：React 18 + Vite 8 + Tailwind 3.4（完全照搬 cc-switch 主题），通过 Tauri commands 读写配置、订阅事件感知服务状态变化。
- 配置存于 Tauri Store 插件 JSON 文件；系统托盘可启停服务。

**Tech Stack:** Tauri 2, axum 0.7, reqwest 0.12, tokio, serde, arc-swap, React 18, Vite 8, TypeScript, Tailwind CSS 3.4, Radix UI, react-hook-form, zod, lucide-react, sonner, framer-motion, pnpm。

**Spec:** [/docs/superpowers/specs/2026-05-25-apiproxy-design.md](../specs/2026-05-25-apiproxy-design.md)

---

## Phase 0：脚手架

### Task 1：项目根配置文件

**Files:**
- Create: `package.json`
- Create: `pnpm-workspace.yaml`
- Create: `.gitignore`
- Create: `.node-version`
- Create: `rust-toolchain.toml`

- [ ] **Step 1: 创建 `package.json`**

```json
{
  "name": "apiproxy",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "pnpm tauri dev",
    "build": "pnpm tauri build",
    "dev:renderer": "vite",
    "build:renderer": "vite build",
    "preview": "vite preview",
    "typecheck": "tsc --noEmit",
    "tauri": "tauri"
  },
  "dependencies": {
    "@radix-ui/react-dialog": "^1.1.6",
    "@radix-ui/react-label": "^2.1.2",
    "@radix-ui/react-select": "^2.1.6",
    "@radix-ui/react-slot": "^1.1.2",
    "@radix-ui/react-switch": "^1.1.3",
    "@radix-ui/react-visually-hidden": "^1.1.2",
    "@tauri-apps/api": "^2.8.0",
    "@tauri-apps/plugin-clipboard-manager": "^2.2.2",
    "@tauri-apps/plugin-dialog": "^2.2.0",
    "@tauri-apps/plugin-store": "^2.2.0",
    "class-variance-authority": "^0.7.1",
    "clsx": "^2.1.1",
    "framer-motion": "^12.0.0",
    "lucide-react": "^0.542.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "react-hook-form": "^7.65.0",
    "@hookform/resolvers": "^3.10.0",
    "sonner": "^2.0.0",
    "tailwind-merge": "^2.6.0",
    "zod": "^3.24.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.8.4",
    "@types/node": "^22.10.0",
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.4",
    "autoprefixer": "^10.4.20",
    "postcss": "^8.5.0",
    "tailwindcss": "^3.4.17",
    "tailwindcss-animate": "^1.0.7",
    "typescript": "^5.6.0",
    "vite": "^8.0.0"
  }
}
```

- [ ] **Step 2: 创建 `pnpm-workspace.yaml`**

```yaml
packages: []
```

- [ ] **Step 3: 创建 `.gitignore`**

```
node_modules
dist
dist-ssr
.DS_Store
.vscode/*
!.vscode/extensions.json
*.log
.env
.env.local
# rust
src-tauri/target
# tauri
src-tauri/Cargo.lock
*.swp
```

- [ ] **Step 4: 创建 `.node-version`**

```
20.18.0
```

- [ ] **Step 5: 创建 `rust-toolchain.toml`**

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

- [ ] **Step 6: 提交**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy
git init
git add -A
git commit -m "chore: bootstrap project root config files"
```

---

### Task 2：TypeScript / Vite / Tailwind 配置

**Files:**
- Create: `tsconfig.json`
- Create: `tsconfig.node.json`
- Create: `vite.config.ts`
- Create: `tailwind.config.cjs`
- Create: `postcss.config.cjs`

- [ ] **Step 1: 创建 `tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": false,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "baseUrl": ".",
    "paths": { "@/*": ["src/*"] }
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

- [ ] **Step 2: 创建 `tsconfig.node.json`**

```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true,
    "strict": true
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 3: 创建 `vite.config.ts`**

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

export default defineConfig({
  root: "src",
  base: "./",
  plugins: [react()],
  resolve: {
    alias: { "@": resolve(__dirname, "src") },
  },
  build: {
    outDir: "../dist",
    emptyOutDir: true,
    target: "es2020",
  },
  server: {
    port: 3000,
    strictPort: true,
    host: false,
  },
  envPrefix: ["VITE_", "TAURI_"],
  clearScreen: false,
});
```

- [ ] **Step 4: 创建 `tailwind.config.cjs`**（照搬 cc-switch 主题）

```js
/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  darkMode: ["selector", ".dark"],
  theme: {
    extend: {
      colors: {
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        card: { DEFAULT: "hsl(var(--card))", foreground: "hsl(var(--card-foreground))" },
        popover: { DEFAULT: "hsl(var(--popover))", foreground: "hsl(var(--popover-foreground))" },
        primary: { DEFAULT: "hsl(var(--primary))", foreground: "hsl(var(--primary-foreground))" },
        secondary: { DEFAULT: "hsl(var(--secondary))", foreground: "hsl(var(--secondary-foreground))" },
        muted: { DEFAULT: "hsl(var(--muted))", foreground: "hsl(var(--muted-foreground))" },
        accent: { DEFAULT: "hsl(var(--accent))", foreground: "hsl(var(--accent-foreground))" },
        destructive: { DEFAULT: "hsl(var(--destructive))", foreground: "hsl(var(--destructive-foreground))" },
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        blue: { 400: "#409CFF", 500: "#0A84FF", 600: "#0060DF" },
        gray: {
          50: "#fafafa", 100: "#f4f4f5", 200: "#e4e4e7", 300: "#d4d4d8",
          400: "#a1a1aa", 500: "#71717a", 600: "#636366", 700: "#48484A",
          800: "#3A3A3C", 900: "#2C2C2E", 950: "#1C1C1E",
        },
        green: { 100: "#d1fae5", 500: "#10b981" },
        red: { 100: "#fee2e2", 500: "#ef4444" },
        amber: { 100: "#fef3c7", 500: "#f59e0b" },
      },
      boxShadow: {
        sm: "0 1px 2px 0 rgb(0 0 0 / 0.05)",
        md: "0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)",
        lg: "0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)",
      },
      borderRadius: {
        sm: "0.375rem",
        md: "0.5rem",
        lg: "0.75rem",
        xl: "0.875rem",
      },
      fontFamily: {
        sans: [
          "-apple-system", "BlinkMacSystemFont", '"Segoe UI"', "Roboto",
          '"Helvetica Neue"', "Arial", "sans-serif",
        ],
        mono: [
          "ui-monospace", "SFMono-Regular", '"SF Mono"', "Consolas",
          '"Liberation Mono"', "Menlo", "monospace",
        ],
      },
      animation: {
        "fade-in": "fadeIn 0.5s ease-out",
        "slide-up": "slideUp 0.5s ease-out",
      },
      keyframes: {
        fadeIn: { "0%": { opacity: "0" }, "100%": { opacity: "1" } },
        slideUp: {
          "0%": { transform: "translateY(20px)", opacity: "0" },
          "100%": { transform: "translateY(0)", opacity: "1" },
        },
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
};
```

- [ ] **Step 5: 创建 `postcss.config.cjs`**

```js
module.exports = {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
};
```

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "chore: add TypeScript, Vite, Tailwind config"
```

---

### Task 3：前端入口与全局 CSS

**Files:**
- Create: `src/index.html`
- Create: `src/index.css`
- Create: `src/main.tsx`
- Create: `src/App.tsx`
- Create: `src/vite-env.d.ts`

- [ ] **Step 1: 创建 `src/index.html`**

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>API 代理</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 2: 创建 `src/index.css`**（沿用 cc-switch 全局 CSS 变量）

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  :root {
    --background: 0 0% 100%;
    --foreground: 240 10% 3.9%;
    --card: 0 0% 100%;
    --card-foreground: 240 10% 3.9%;
    --popover: 0 0% 100%;
    --popover-foreground: 240 10% 3.9%;
    --primary: 210 100% 56%;
    --primary-foreground: 0 0% 98%;
    --secondary: 240 4.8% 95.9%;
    --secondary-foreground: 240 5.9% 10%;
    --muted: 240 4.8% 95.9%;
    --muted-foreground: 240 3.8% 46.1%;
    --accent: 240 4.8% 95.9%;
    --accent-foreground: 240 5.9% 10%;
    --destructive: 0 84.2% 60.2%;
    --destructive-foreground: 0 0% 98%;
    --border: 240 5.9% 90%;
    --input: 240 5.9% 90%;
    --ring: 210 100% 56%;
  }

  .dark {
    --background: 240 5% 12%;
    --foreground: 0 0% 98%;
    --card: 240 5% 16%;
    --card-foreground: 0 0% 98%;
    --popover: 240 5% 16%;
    --popover-foreground: 0 0% 98%;
    --primary: 210 100% 56%;
    --primary-foreground: 0 0% 98%;
    --secondary: 240 3.7% 22%;
    --secondary-foreground: 0 0% 98%;
    --muted: 240 3.7% 22%;
    --muted-foreground: 240 5% 64.9%;
    --accent: 240 3.7% 22%;
    --accent-foreground: 0 0% 98%;
    --destructive: 0 72.2% 50.6%;
    --destructive-foreground: 0 0% 98%;
    --border: 240 5% 24%;
    --input: 240 5% 24%;
    --ring: 210 100% 56%;
  }

  * {
    @apply border-border;
    scrollbar-width: none;
  }
  *::-webkit-scrollbar {
    display: none;
  }
  html,
  body {
    @apply m-0 p-0 text-sm;
    background-color: hsl(var(--background));
    color: hsl(var(--foreground));
    font-family:
      -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue",
      Arial, sans-serif;
    line-height: 1.5;
  }
  html.dark {
    color-scheme: dark;
  }
  *:focus-visible {
    outline: 2px solid hsl(var(--ring));
    outline-offset: 2px;
  }
  [data-tauri-drag-region] {
    -webkit-app-region: drag;
  }
  [data-tauri-no-drag],
  button,
  input,
  select,
  textarea,
  a {
    -webkit-app-region: no-drag;
  }
}

@layer utilities {
  .border-border-default {
    border-color: hsl(var(--border));
  }
  .border-border-active {
    border-color: hsl(var(--primary));
  }
  .border-border-hover {
    border-color: hsl(var(--primary) / 0.4);
  }
}
```

- [ ] **Step 3: 创建 `src/vite-env.d.ts`**

```ts
/// <reference types="vite/client" />
```

- [ ] **Step 4: 创建占位 `src/App.tsx`**

```tsx
export default function App() {
  return (
    <div className="flex h-screen items-center justify-center text-xl">
      API 代理
    </div>
  );
}
```

- [ ] **Step 5: 创建 `src/main.tsx`**

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "chore: add frontend entry, global CSS, placeholder App"
```

---

### Task 4：Tauri 后端脚手架

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/icons/icon.png`（占位）

- [ ] **Step 1: 创建 `src-tauri/Cargo.toml`**

```toml
[package]
name = "apiproxy"
version = "0.1.0"
description = "本地路由代理工具"
authors = ["you"]
edition = "2021"
rust-version = "1.77"

[lib]
name = "apiproxy_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon", "macos-private-api"] }
tauri-plugin-store = "2"
tauri-plugin-single-instance = "2"
tauri-plugin-clipboard-manager = "2"
tauri-plugin-dialog = "2"

serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
url = { version = "2", features = ["serde"] }
arc-swap = "1"

tokio = { version = "1", features = ["full"] }
axum = { version = "0.7", features = ["macros"] }
hyper = { version = "1", features = ["full"] }
hyper-util = { version = "0.1", features = ["tokio"] }
http = "1"
http-body-util = "0.1"
bytes = "1"
futures-util = "0.3"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream", "json"] }
tower = { version = "0.5", features = ["util"] }
tower-http = { version = "0.5", features = ["catch-panic"] }

thiserror = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util", "macros"] }

[profile.release]
opt-level = "s"
lto = "thin"
codegen-units = 1
strip = "symbols"
panic = "unwind"
```

- [ ] **Step 2: 创建 `src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 3: 创建 `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "API 代理",
  "version": "0.1.0",
  "identifier": "com.apiproxy.app",
  "build": {
    "beforeDevCommand": "pnpm run dev:renderer",
    "beforeBuildCommand": "pnpm run build:renderer",
    "devUrl": "http://localhost:3000",
    "frontendDist": "../dist"
  },
  "app": {
    "macOSPrivateApi": true,
    "windows": [
      {
        "label": "main",
        "title": "",
        "titleBarStyle": "Overlay",
        "width": 1000,
        "height": 650,
        "minWidth": 900,
        "minHeight": 600,
        "visible": false,
        "resizable": true,
        "fullscreen": false,
        "center": true
      }
    ],
    "security": {
      "csp": "default-src 'self'; img-src 'self' data: https: http:; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: http://ipc.localhost https: http:",
      "assetProtocol": { "enable": true, "scope": ["**"] }
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/icon.png"],
    "macOS": {
      "minimumSystemVersion": "12.0"
    }
  }
}
```

- [ ] **Step 4: 创建 `src-tauri/capabilities/default.json`**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default permissions",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-close",
    "core:window:allow-set-focus",
    "core:app:allow-version",
    "core:event:default",
    "store:default",
    "clipboard-manager:allow-write-text",
    "dialog:default"
  ]
}
```

- [ ] **Step 5: 占位图标** — 暂用空白 PNG（最终用户替换）

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
mkdir -p icons
# 用 1024×1024 透明 PNG 占位
python3 -c "
from struct import pack
import zlib, base64
W, H = 32, 32
raw = b''.join(b'\\x00' + b'\\x00\\x00\\x00\\x00' * W for _ in range(H))
def chunk(tp, data):
    body = tp + data
    return pack('>I', len(data)) + body + pack('>I', zlib.crc32(body) & 0xffffffff)
sig = b'\\x89PNG\\r\\n\\x1a\\n'
ihdr = pack('>IIBBBBB', W, H, 8, 6, 0, 0, 0)
idat = zlib.compress(raw)
png = sig + chunk(b'IHDR', ihdr) + chunk(b'IDAT', idat) + chunk(b'IEND', b'')
open('icons/icon.png','wb').write(png)
"
```

- [ ] **Step 6: 创建 `src-tauri/src/main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    apiproxy_lib::run()
}
```

- [ ] **Step 7: 创建 `src-tauri/src/lib.rs`**

```rust
use tauri::Manager;

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
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 8: 安装依赖（pnpm + cargo fetch）**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy
pnpm install
```

预期：安装成功、生成 `pnpm-lock.yaml`、`node_modules/`。

- [ ] **Step 9: 烟雾测试 `pnpm typecheck`**

```bash
pnpm typecheck
```

预期：无错误。

- [ ] **Step 10: 启动 dev 验证**

```bash
pnpm tauri dev
```

预期：窗口弹出，居中显示"API 代理"大字。手动 Ctrl-C 关闭。

- [ ] **Step 11: 提交**

```bash
git add -A
git commit -m "chore: bootstrap Tauri backend with placeholder window"
```

---

## Phase 1：Rust 后端 — 类型与纯函数

### Task 5：定义配置类型

**Files:**
- Create: `src-tauri/src/config/mod.rs`
- Modify: `src-tauri/src/lib.rs`（添加 `mod config;`）

- [ ] **Step 1: 创建 `src-tauri/src/config/mod.rs`**

```rust
pub mod store;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Set,
    Add,
    Remove,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rule {
    pub action: RuleAction,
    pub key: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub enabled: bool,
    pub path: String,
    pub upstream_url: String,
    #[serde(default = "default_true")]
    pub strip_prefix: bool,
    #[serde(default)]
    pub header_rules: Vec<Rule>,
    #[serde(default)]
    pub query_rules: Vec<Rule>,
    #[serde(default)]
    pub body_merge: String,
    pub created_at: i64,
    pub updated_at: i64,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalConfig {
    pub listen_address: String,
    pub listen_port: u16,
    pub request_timeout_secs: u64,
    pub theme: String,
    pub close_to_tray: bool,
    pub auto_start_server: bool,
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
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub running: bool,
    pub listen_address: Option<String>,
    pub listen_port: Option<u16>,
    pub started_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("名称不能为空且不超过 60 字符")]
    InvalidName,
    #[error("路径无效：{0}")]
    InvalidPath(String),
    #[error("上游地址无效：{0}")]
    InvalidUrl(String),
    #[error("请求体合并必须为合法的 JSON 对象：{0}")]
    InvalidBodyMerge(String),
    #[error("端点路径 '{path}' 与已有端点 '{conflict_with}' 冲突")]
    DuplicatePath { path: String, conflict_with: String },
    #[error("未找到端点：{0}")]
    NotFound(String),
}

impl Endpoint {
    pub fn normalize(&mut self) {
        if !self.path.starts_with('/') {
            self.path = format!("/{}", self.path);
        }
        while self.path.contains("//") {
            self.path = self.path.replace("//", "/");
        }
        if self.path.len() > 1 && self.path.ends_with('/') {
            self.path.pop();
        }
        self.name = self.name.trim().to_string();
        self.description = self.description.trim().to_string();
        self.upstream_url = self.upstream_url.trim().to_string();
        self.body_merge = self.body_merge.trim().to_string();
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.name.is_empty() || self.name.chars().count() > 60 {
            return Err(ConfigError::InvalidName);
        }
        if self.path.is_empty() || !self.path.starts_with('/') || self.path == "/" {
            return Err(ConfigError::InvalidPath(self.path.clone()));
        }
        if !self
            .path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '.'))
        {
            return Err(ConfigError::InvalidPath(self.path.clone()));
        }
        match Url::parse(&self.upstream_url) {
            Ok(u) if matches!(u.scheme(), "http" | "https") => {}
            _ => return Err(ConfigError::InvalidUrl(self.upstream_url.clone())),
        }
        if !self.body_merge.is_empty() {
            let v: serde_json::Value = serde_json::from_str(&self.body_merge)
                .map_err(|e| ConfigError::InvalidBodyMerge(e.to_string()))?;
            if !v.is_object() {
                return Err(ConfigError::InvalidBodyMerge("顶层必须为对象".into()));
            }
        }
        Ok(())
    }
}

pub fn validate_unique_paths(endpoints: &[Endpoint]) -> Result<(), ConfigError> {
    let mut seen: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for e in endpoints.iter().filter(|e| e.enabled) {
        if let Some(other) = seen.get(e.path.as_str()) {
            return Err(ConfigError::DuplicatePath {
                path: e.path.clone(),
                conflict_with: (*other).to_string(),
            });
        }
        seen.insert(e.path.as_str(), e.name.as_str());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ep(path: &str, enabled: bool) -> Endpoint {
        Endpoint {
            id: "id".into(),
            name: "ep".into(),
            description: "".into(),
            enabled,
            path: path.into(),
            upstream_url: "https://example.com".into(),
            strip_prefix: true,
            header_rules: vec![],
            query_rules: vec![],
            body_merge: "".into(),
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn normalize_strips_trailing_slash() {
        let mut e = ep("/cc/", true);
        e.normalize();
        assert_eq!(e.path, "/cc");
    }

    #[test]
    fn normalize_collapses_double_slash() {
        let mut e = ep("//cc//foo", true);
        e.normalize();
        assert_eq!(e.path, "/cc/foo");
    }

    #[test]
    fn validate_rejects_root_path() {
        let mut e = ep("/", true);
        assert!(matches!(e.validate(), Err(ConfigError::InvalidPath(_))));
        e.path = "".into();
        assert!(matches!(e.validate(), Err(ConfigError::InvalidPath(_))));
    }

    #[test]
    fn validate_rejects_invalid_chars() {
        let mut e = ep("/cc 1", true);
        assert!(matches!(e.validate(), Err(ConfigError::InvalidPath(_))));
    }

    #[test]
    fn validate_rejects_bad_url() {
        let mut e = ep("/cc", true);
        e.upstream_url = "ftp://x.com".into();
        assert!(matches!(e.validate(), Err(ConfigError::InvalidUrl(_))));
    }

    #[test]
    fn validate_body_merge_must_be_object() {
        let mut e = ep("/cc", true);
        e.body_merge = "[1,2]".into();
        assert!(matches!(e.validate(), Err(ConfigError::InvalidBodyMerge(_))));
        e.body_merge = "{\"a\":1}".into();
        assert!(e.validate().is_ok());
    }

    #[test]
    fn unique_paths_ignores_disabled() {
        let a = ep("/cc", true);
        let b = ep("/cc", false);
        assert!(validate_unique_paths(&[a, b]).is_ok());
    }

    #[test]
    fn unique_paths_detects_conflict() {
        let mut a = ep("/cc", true);
        a.name = "A".into();
        let mut b = ep("/cc", true);
        b.name = "B".into();
        assert!(matches!(
            validate_unique_paths(&[a, b]),
            Err(ConfigError::DuplicatePath { .. })
        ));
    }
}
```

- [ ] **Step 2: 创建占位 `src-tauri/src/config/store.rs`**

```rust
// 实际逻辑在 Task 6 实现
pub struct Placeholder;
```

- [ ] **Step 3: 更新 `src-tauri/src/lib.rs` 顶部添加模块声明**

```rust
mod config;
```

放在最顶部，紧跟 `use tauri::Manager;` 之前。

- [ ] **Step 4: 运行测试**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo test config:: -- --nocapture
```

预期：所有测试通过。

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(backend): add config types with validation and tests"
```

---

### Task 6：Tauri Store 读写封装

**Files:**
- Modify: `src-tauri/src/config/store.rs`

- [ ] **Step 1: 完整重写 `src-tauri/src/config/store.rs`**

```rust
use super::{Endpoint, GlobalConfig};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, Runtime, Wry};
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

    pub fn save_endpoints(
        &self,
        app: &AppHandle<Wry>,
        new: Vec<Endpoint>,
    ) -> Result<(), String> {
        {
            let mut s = self.inner.lock().unwrap();
            s.endpoints = new.clone();
        }
        let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
        store.set(KEY_ENDPOINTS, serde_json::to_value(&new).unwrap());
        store.save().map_err(|e| e.to_string())
    }
}
```

- [ ] **Step 2: 编译验证**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo build
```

预期：成功（无警告或仅有 dead-code 警告 OK）。

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "feat(backend): add ConfigStore with Tauri Store backing"
```

---

### Task 7：transform 纯函数（TDD）

**Files:**
- Create: `src-tauri/src/proxy/mod.rs`（占位）
- Create: `src-tauri/src/proxy/transform.rs`
- Modify: `src-tauri/src/lib.rs`（添加 `mod proxy;`）

- [ ] **Step 1: 创建 `src-tauri/src/proxy/mod.rs`** 占位

```rust
pub mod transform;
```

- [ ] **Step 2: 创建 `src-tauri/src/proxy/transform.rs` 测试与实现一起**

```rust
use crate::config::{Endpoint, Rule, RuleAction};
use http::HeaderMap;
use http::header::HeaderName;
use http::HeaderValue;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum TransformError {
    #[error("无法解析上游 URL：{0}")]
    InvalidUpstream(String),
    #[error("请求体不是合法 JSON：{0}")]
    InvalidJsonBody(String),
    #[error("请求体合并 patch 不是合法 JSON：{0}")]
    InvalidJsonPatch(String),
    #[error("非法 header：{0}")]
    InvalidHeader(String),
}

/// 路由匹配：在 enabled 端点中按 path 长度降序找
/// req.uri.path() 等于 endpoint.path 或以 endpoint.path + "/" 开头的端点
pub fn match_endpoint<'a>(routes: &'a [Endpoint], req_path: &str) -> Option<&'a Endpoint> {
    let mut sorted: Vec<&Endpoint> = routes.iter().filter(|e| e.enabled).collect();
    sorted.sort_by_key(|e| std::cmp::Reverse(e.path.len()));
    sorted.into_iter().find(|e| {
        req_path == e.path || req_path.starts_with(&format!("{}/", e.path))
    })
}

/// 构造上游 URL
pub fn build_upstream_url(
    endpoint: &Endpoint,
    req_path: &str,
    req_query: Option<&str>,
) -> Result<Url, TransformError> {
    let base = Url::parse(&endpoint.upstream_url)
        .map_err(|e| TransformError::InvalidUpstream(e.to_string()))?;
    let remaining = if endpoint.strip_prefix {
        let stripped = req_path.strip_prefix(&endpoint.path).unwrap_or(req_path);
        if stripped.is_empty() { "" } else { stripped }
    } else {
        req_path
    };
    let mut url = base.clone();
    if !remaining.is_empty() {
        let base_path = url.path().trim_end_matches('/').to_string();
        let suffix = remaining.trim_start_matches('/');
        let combined = if suffix.is_empty() {
            base_path
        } else {
            format!("{}/{}", base_path, suffix)
        };
        url.set_path(&combined);
    }
    if let Some(q) = req_query {
        url.set_query(Some(q));
    }
    Ok(url)
}

/// 对 URL 查询字符串应用 rules
pub fn apply_query_rules(url: &mut Url, rules: &[Rule]) {
    if rules.is_empty() {
        return;
    }
    let mut pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    for rule in rules {
        match rule.action {
            RuleAction::Remove => pairs.retain(|(k, _)| k != &rule.key),
            RuleAction::Set => {
                pairs.retain(|(k, _)| k != &rule.key);
                pairs.push((rule.key.clone(), rule.value.clone()));
            }
            RuleAction::Add => {
                pairs.push((rule.key.clone(), rule.value.clone()));
            }
        }
    }
    if pairs.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(pairs.iter());
    }
}

/// 对 headers 应用 rules
pub fn apply_header_rules(headers: &mut HeaderMap, rules: &[Rule]) -> Result<(), TransformError> {
    for rule in rules {
        let name = HeaderName::from_bytes(rule.key.as_bytes())
            .map_err(|_| TransformError::InvalidHeader(rule.key.clone()))?;
        match rule.action {
            RuleAction::Remove => {
                headers.remove(&name);
            }
            RuleAction::Set => {
                let value = HeaderValue::from_str(&rule.value)
                    .map_err(|_| TransformError::InvalidHeader(rule.value.clone()))?;
                headers.insert(name, value);
            }
            RuleAction::Add => {
                let value = HeaderValue::from_str(&rule.value)
                    .map_err(|_| TransformError::InvalidHeader(rule.value.clone()))?;
                headers.append(name, value);
            }
        }
    }
    Ok(())
}

/// 去掉 hop-by-hop headers（RFC 7230 §6.1）
pub fn strip_hop_by_hop(headers: &mut HeaderMap) {
    const HOP: &[&str] = &[
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailer",
        "transfer-encoding",
        "upgrade",
        "host",
    ];
    let connection_listed: Vec<HeaderName> = headers
        .get_all("connection")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|s| s.split(',').map(|t| t.trim().to_string()))
        .filter_map(|n| HeaderName::from_bytes(n.as_bytes()).ok())
        .collect();
    for h in HOP {
        headers.remove(*h);
    }
    for h in connection_listed {
        headers.remove(&h);
    }
}

/// JSON 深合并：对象字段递归合并，其余类型 patch 覆盖
pub fn merge_json_body(
    original: &[u8],
    patch: &serde_json::Value,
) -> Result<Vec<u8>, TransformError> {
    if original.is_empty() {
        return Ok(serde_json::to_vec(patch).unwrap());
    }
    let mut base: serde_json::Value = serde_json::from_slice(original)
        .map_err(|e| TransformError::InvalidJsonBody(e.to_string()))?;
    deep_merge(&mut base, patch);
    Ok(serde_json::to_vec(&base).unwrap())
}

fn deep_merge(target: &mut serde_json::Value, patch: &serde_json::Value) {
    use serde_json::Value::*;
    match (target, patch) {
        (Object(a), Object(b)) => {
            for (k, v) in b {
                deep_merge(a.entry(k.clone()).or_insert(Null), v);
            }
        }
        (slot, other) => {
            *slot = other.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Endpoint;

    fn ep(path: &str, upstream: &str, strip: bool) -> Endpoint {
        Endpoint {
            id: "id".into(),
            name: "n".into(),
            description: "".into(),
            enabled: true,
            path: path.into(),
            upstream_url: upstream.into(),
            strip_prefix: strip,
            header_rules: vec![],
            query_rules: vec![],
            body_merge: "".into(),
            created_at: 0,
            updated_at: 0,
        }
    }

    fn rule(action: RuleAction, k: &str, v: &str) -> Rule {
        Rule { action, key: k.into(), value: v.into() }
    }

    #[test]
    fn match_picks_longest_prefix_first() {
        let a = ep("/api", "https://a.com", true);
        let b = ep("/api/v1", "https://b.com", true);
        let routes = vec![a.clone(), b.clone()];
        assert_eq!(match_endpoint(&routes, "/api/v1/users").unwrap().path, "/api/v1");
        assert_eq!(match_endpoint(&routes, "/api/raw").unwrap().path, "/api");
    }

    #[test]
    fn match_requires_boundary() {
        let a = ep("/cc", "https://a.com", true);
        let routes = vec![a];
        assert!(match_endpoint(&routes, "/ccfoo").is_none());
        assert!(match_endpoint(&routes, "/cc").is_some());
        assert!(match_endpoint(&routes, "/cc/foo").is_some());
    }

    #[test]
    fn match_ignores_disabled() {
        let mut a = ep("/cc", "https://a.com", true);
        a.enabled = false;
        assert!(match_endpoint(&[a], "/cc/x").is_none());
    }

    #[test]
    fn build_url_strips_prefix() {
        let e = ep("/cc", "https://api.example.com/v1", true);
        let url = build_upstream_url(&e, "/cc/chat", None).unwrap();
        assert_eq!(url.as_str(), "https://api.example.com/v1/chat");
    }

    #[test]
    fn build_url_keeps_prefix_when_disabled() {
        let e = ep("/cc", "https://api.example.com/v1", false);
        let url = build_upstream_url(&e, "/cc/chat", None).unwrap();
        assert_eq!(url.as_str(), "https://api.example.com/v1/cc/chat");
    }

    #[test]
    fn build_url_empty_remaining_keeps_base() {
        let e = ep("/cc", "https://api.example.com/v1", true);
        let url = build_upstream_url(&e, "/cc", None).unwrap();
        assert_eq!(url.as_str(), "https://api.example.com/v1");
    }

    #[test]
    fn build_url_query_passthrough() {
        let e = ep("/cc", "https://api.example.com/v1", true);
        let url = build_upstream_url(&e, "/cc/x", Some("a=1&b=2")).unwrap();
        assert_eq!(url.as_str(), "https://api.example.com/v1/x?a=1&b=2");
    }

    #[test]
    fn apply_query_set_overwrites() {
        let mut url = Url::parse("https://x.com/?foo=1&foo=2").unwrap();
        apply_query_rules(&mut url, &[rule(RuleAction::Set, "foo", "9")]);
        assert_eq!(url.as_str(), "https://x.com/?foo=9");
    }

    #[test]
    fn apply_query_add_appends() {
        let mut url = Url::parse("https://x.com/?foo=1").unwrap();
        apply_query_rules(&mut url, &[rule(RuleAction::Add, "foo", "2")]);
        let q = url.query().unwrap();
        assert!(q.contains("foo=1") && q.contains("foo=2"));
    }

    #[test]
    fn apply_query_remove() {
        let mut url = Url::parse("https://x.com/?foo=1&bar=2").unwrap();
        apply_query_rules(&mut url, &[rule(RuleAction::Remove, "foo", "")]);
        assert_eq!(url.query(), Some("bar=2"));
    }

    #[test]
    fn apply_header_set_replaces() {
        let mut h = HeaderMap::new();
        h.insert("x-a", "old".parse().unwrap());
        apply_header_rules(&mut h, &[rule(RuleAction::Set, "x-a", "new")]).unwrap();
        assert_eq!(h.get("x-a").unwrap(), "new");
    }

    #[test]
    fn apply_header_add_appends() {
        let mut h = HeaderMap::new();
        h.insert("x-a", "1".parse().unwrap());
        apply_header_rules(&mut h, &[rule(RuleAction::Add, "x-a", "2")]).unwrap();
        let values: Vec<&str> = h.get_all("x-a").iter().filter_map(|v| v.to_str().ok()).collect();
        assert_eq!(values, vec!["1", "2"]);
    }

    #[test]
    fn apply_header_remove() {
        let mut h = HeaderMap::new();
        h.insert("x-a", "1".parse().unwrap());
        apply_header_rules(&mut h, &[rule(RuleAction::Remove, "x-a", "")]).unwrap();
        assert!(h.get("x-a").is_none());
    }

    #[test]
    fn strip_hop_by_hop_removes_standard() {
        let mut h = HeaderMap::new();
        h.insert("connection", "keep-alive, x-custom".parse().unwrap());
        h.insert("keep-alive", "timeout=5".parse().unwrap());
        h.insert("x-custom", "yes".parse().unwrap());
        h.insert("host", "old".parse().unwrap());
        h.insert("x-keep", "yes".parse().unwrap());
        strip_hop_by_hop(&mut h);
        assert!(h.get("connection").is_none());
        assert!(h.get("keep-alive").is_none());
        assert!(h.get("x-custom").is_none());
        assert!(h.get("host").is_none());
        assert_eq!(h.get("x-keep").unwrap(), "yes");
    }

    #[test]
    fn merge_json_object_deep() {
        let orig = br#"{"a":1,"b":{"c":2,"d":3}}"#;
        let patch: serde_json::Value = serde_json::json!({"b":{"c":20},"e":5});
        let out = merge_json_body(orig, &patch).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v, serde_json::json!({"a":1,"b":{"c":20,"d":3},"e":5}));
    }

    #[test]
    fn merge_json_array_replaces() {
        let orig = br#"{"a":[1,2,3]}"#;
        let patch: serde_json::Value = serde_json::json!({"a":[9]});
        let out = merge_json_body(orig, &patch).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v, serde_json::json!({"a":[9]}));
    }

    #[test]
    fn merge_json_empty_body_uses_patch() {
        let out = merge_json_body(b"", &serde_json::json!({"x":1})).unwrap();
        assert_eq!(out, br#"{"x":1}"#);
    }
}
```

- [ ] **Step 3: 在 `src-tauri/src/lib.rs` 顶部添加 `mod proxy;`**

```rust
mod config;
mod proxy;
```

- [ ] **Step 4: 运行测试**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo test proxy::transform:: -- --nocapture
```

预期：全部通过。

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(backend): add transform pure functions with TDD coverage"
```

---

## Phase 2：Rust 后端 — 服务与命令

### Task 8：axum 反向代理 handler

**Files:**
- Create: `src-tauri/src/proxy/handler.rs`
- Create: `src-tauri/src/proxy/server.rs`
- Modify: `src-tauri/src/proxy/mod.rs`

- [ ] **Step 1: 创建 `src-tauri/src/proxy/handler.rs`**

```rust
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use axum::body::{Body, Bytes};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use futures_util::TryStreamExt;
use http::header::HOST;
use serde_json::json;
use tracing::error;

use crate::config::Endpoint;
use crate::proxy::transform;

#[derive(Clone)]
pub struct ProxyState {
    pub routes: Arc<ArcSwap<Vec<Endpoint>>>,
    pub client: reqwest::Client,
    pub timeout: Arc<std::sync::atomic::AtomicU64>,
}

pub async fn proxy_handler(State(state): State<ProxyState>, req: Request) -> Response {
    let req_path = req.uri().path().to_string();
    let req_query = req.uri().query().map(|s| s.to_string());
    let routes = state.routes.load_full();

    let endpoint = match transform::match_endpoint(&routes, &req_path) {
        Some(e) => e.clone(),
        None => {
            return json_error(
                StatusCode::NOT_FOUND,
                "no endpoint matched",
                "NO_ENDPOINT_MATCHED",
                Some(&req_path),
                None,
            );
        }
    };

    let mut upstream_url =
        match transform::build_upstream_url(&endpoint, &req_path, req_query.as_deref()) {
            Ok(u) => u,
            Err(e) => {
                return json_error(
                    StatusCode::BAD_GATEWAY,
                    &e.to_string(),
                    "BUILD_URL_FAILED",
                    Some(&endpoint.path),
                    Some(&endpoint.upstream_url),
                );
            }
        };
    transform::apply_query_rules(&mut upstream_url, &endpoint.query_rules);

    let (parts, body) = req.into_parts();
    let mut headers = parts.headers.clone();
    transform::strip_hop_by_hop(&mut headers);
    if let Some(host) = upstream_url.host_str() {
        let host_value = match upstream_url.port() {
            Some(p) => format!("{host}:{p}"),
            None => host.to_string(),
        };
        if let Ok(v) = HeaderValue::from_str(&host_value) {
            headers.insert(HOST, v);
        }
    }
    if let Err(e) = transform::apply_header_rules(&mut headers, &endpoint.header_rules) {
        return json_error(
            StatusCode::BAD_REQUEST,
            &e.to_string(),
            "HEADER_RULE_FAILED",
            Some(&endpoint.path),
            None,
        );
    }

    let method = reqwest::Method::from_bytes(parts.method.as_str().as_bytes())
        .unwrap_or(reqwest::Method::GET);
    let timeout_secs = state.timeout.load(std::sync::atomic::Ordering::Relaxed);

    let mut warn_body_merge_skipped = false;
    let body_data: BodyData =
        match prepare_body(body, &mut headers, &endpoint, &mut warn_body_merge_skipped).await {
            Ok(b) => b,
            Err(msg) => {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    &msg,
                    "BODY_PREPARE_FAILED",
                    Some(&endpoint.path),
                    None,
                );
            }
        };

    let mut req_builder = state
        .client
        .request(method, upstream_url.clone())
        .timeout(Duration::from_secs(timeout_secs))
        .headers(reqwest_headers(&headers));

    req_builder = match body_data {
        BodyData::Empty => req_builder,
        BodyData::Bytes(b) => req_builder.body(b),
    };

    let upstream_response = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            error!("upstream request failed: {e}");
            let code = if e.is_timeout() {
                "UPSTREAM_TIMEOUT"
            } else if e.is_connect() {
                "UPSTREAM_CONNECT"
            } else {
                "UPSTREAM_ERROR"
            };
            return json_error(
                StatusCode::BAD_GATEWAY,
                &e.to_string(),
                code,
                Some(&endpoint.path),
                Some(&endpoint.upstream_url),
            );
        }
    };

    let status = StatusCode::from_u16(upstream_response.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    let mut resp_headers = HeaderMap::new();
    for (k, v) in upstream_response.headers() {
        if let (Ok(name), Ok(value)) = (
            http::HeaderName::from_bytes(k.as_str().as_bytes()),
            HeaderValue::from_bytes(v.as_bytes()),
        ) {
            resp_headers.append(name, value);
        }
    }
    transform::strip_hop_by_hop(&mut resp_headers);
    if warn_body_merge_skipped {
        resp_headers.insert("x-apiproxy-warn", HeaderValue::from_static("body-merge-skipped"));
    }

    let stream = upstream_response
        .bytes_stream()
        .map_ok(Bytes::from)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
    let body = Body::from_stream(stream);

    let mut response = Response::builder().status(status);
    if let Some(h) = response.headers_mut() {
        *h = resp_headers;
    }
    response.body(body).unwrap()
}

enum BodyData {
    Empty,
    Bytes(Vec<u8>),
}

async fn prepare_body(
    body: Body,
    headers: &mut HeaderMap,
    endpoint: &Endpoint,
    warn: &mut bool,
) -> Result<BodyData, String> {
    use http_body_util::BodyExt;
    if endpoint.body_merge.is_empty() {
        let collected = body.collect().await.map_err(|e| e.to_string())?;
        let bytes = collected.to_bytes();
        if bytes.is_empty() {
            Ok(BodyData::Empty)
        } else {
            headers.remove(http::header::CONTENT_LENGTH);
            Ok(BodyData::Bytes(bytes.to_vec()))
        }
    } else {
        let collected = body.collect().await.map_err(|e| e.to_string())?;
        let bytes = collected.to_bytes();
        let patch: serde_json::Value = serde_json::from_str(&endpoint.body_merge)
            .map_err(|e| format!("body_merge 解析失败：{e}"))?;
        let content_type = headers
            .get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let is_json = content_type.contains("application/json") || bytes.is_empty();
        if !is_json {
            *warn = true;
            headers.remove(http::header::CONTENT_LENGTH);
            return Ok(BodyData::Bytes(bytes.to_vec()));
        }
        match transform::merge_json_body(&bytes, &patch) {
            Ok(merged) => {
                headers.insert(
                    http::header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                headers.insert(
                    http::header::CONTENT_LENGTH,
                    HeaderValue::from_str(&merged.len().to_string()).unwrap(),
                );
                Ok(BodyData::Bytes(merged))
            }
            Err(_) => {
                *warn = true;
                headers.remove(http::header::CONTENT_LENGTH);
                Ok(BodyData::Bytes(bytes.to_vec()))
            }
        }
    }
}

fn reqwest_headers(src: &HeaderMap) -> reqwest::header::HeaderMap {
    let mut out = reqwest::header::HeaderMap::new();
    for (k, v) in src {
        if let (Ok(name), Ok(value)) = (
            reqwest::header::HeaderName::from_bytes(k.as_str().as_bytes()),
            reqwest::header::HeaderValue::from_bytes(v.as_bytes()),
        ) {
            out.append(name, value);
        }
    }
    out
}

fn json_error(
    status: StatusCode,
    msg: &str,
    code: &str,
    endpoint: Option<&str>,
    upstream: Option<&str>,
) -> Response {
    let body = json!({
        "error": msg,
        "code": code,
        "endpoint": endpoint,
        "upstream": upstream,
    });
    let mut resp = (status, axum::Json(body)).into_response();
    resp.headers_mut().insert(
        "x-apiproxy",
        HeaderValue::from_static("error"),
    );
    resp
}
```

- [ ] **Step 2: 创建 `src-tauri/src/proxy/server.rs`**

```rust
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use arc_swap::ArcSwap;
use axum::Router;
use axum::routing::any;
use tokio::sync::oneshot;

use crate::config::Endpoint;
use crate::proxy::handler::{proxy_handler, ProxyState};

pub struct RunningServer {
    pub address: String,
    pub port: u16,
    pub shutdown: oneshot::Sender<()>,
    pub join: tokio::task::JoinHandle<()>,
}

pub async fn spawn_server(
    address: String,
    port: u16,
    routes: Arc<ArcSwap<Vec<Endpoint>>>,
    timeout: Arc<AtomicU64>,
    client: reqwest::Client,
) -> Result<RunningServer, String> {
    let state = ProxyState { routes, client, timeout };
    let app: Router = Router::new()
        .fallback(any(proxy_handler))
        .with_state(state);

    let addr: SocketAddr = format!("{address}:{port}")
        .parse()
        .map_err(|e: std::net::AddrParseError| format!("地址解析失败：{e}"))?;

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("绑定端口失败 {addr}：{e}"))?;
    let local_addr = listener.local_addr().map_err(|e| e.to_string())?;

    let (tx, rx) = oneshot::channel::<()>();
    let join = tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = rx.await;
            })
            .await;
    });

    Ok(RunningServer {
        address: local_addr.ip().to_string(),
        port: local_addr.port(),
        shutdown: tx,
        join,
    })
}
```

- [ ] **Step 3: 更新 `src-tauri/src/proxy/mod.rs`**

```rust
pub mod handler;
pub mod server;
pub mod transform;

pub mod manager;
```

- [ ] **Step 4: 编译**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo build
```

预期：error（找不到 `manager` 模块）— 留到下一任务。先注释 `pub mod manager;` 让本任务通过。

- [ ] **Step 5: 临时去掉 `pub mod manager;`**

```rust
pub mod handler;
pub mod server;
pub mod transform;
```

- [ ] **Step 6: 编译验证**

```bash
cargo build
```

预期：成功。

- [ ] **Step 7: 提交**

```bash
git add -A
git commit -m "feat(backend): add axum proxy handler and server spawn"
```

---

### Task 9：ProxyManager（启停 + 热切换）

**Files:**
- Create: `src-tauri/src/proxy/manager.rs`
- Modify: `src-tauri/src/proxy/mod.rs`

- [ ] **Step 1: 创建 `src-tauri/src/proxy/manager.rs`**

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
    client: reqwest::Client,
    state: Mutex<Inner>,
}

struct Inner {
    running: Option<RunningServer>,
    started_at: Option<i64>,
    last_error: Option<String>,
}

impl ProxyManager {
    pub fn new(routes: Vec<Endpoint>, timeout_secs: u64) -> Self {
        let client = reqwest::Client::builder()
            .pool_idle_timeout(Some(std::time::Duration::from_secs(90)))
            .build()
            .expect("build reqwest client");
        Self {
            routes: Arc::new(ArcSwap::from_pointee(routes)),
            timeout: Arc::new(AtomicU64::new(timeout_secs)),
            client,
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
        let client = self.client.clone();
        let server = spawn_server(address, port, routes, timeout, client)
            .await
            .map_err(|e| {
                self.set_last_error(Some(e.clone()));
                e
            })?;
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
    ) -> Result<ServerStatus, String> {
        self.stop().await?;
        self.start(address, port).await
    }

    fn set_last_error(&self, msg: Option<String>) {
        let mut inner = self.state.lock().unwrap();
        inner.last_error = msg;
    }
}
```

- [ ] **Step 2: 把 `manager` 模块重新加入 `src-tauri/src/proxy/mod.rs`**

```rust
pub mod handler;
pub mod manager;
pub mod server;
pub mod transform;
```

- [ ] **Step 3: 编译**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo build
```

预期：成功。

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "feat(backend): add ProxyManager with hot route swap"
```

---

### Task 10：事件常量与 AppState

**Files:**
- Create: `src-tauri/src/events.rs`
- Create: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建 `src-tauri/src/events.rs`**

```rust
pub const STATUS_CHANGED: &str = "proxy://status-changed";
pub const ENDPOINTS_CHANGED: &str = "proxy://endpoints-changed";
pub const CONFIG_ERROR: &str = "proxy://config-error";
```

- [ ] **Step 2: 创建 `src-tauri/src/state.rs`**

```rust
use std::sync::Arc;

use crate::config::store::ConfigStore;
use crate::proxy::manager::ProxyManager;

pub struct AppState {
    pub store: Arc<ConfigStore>,
    pub manager: Arc<ProxyManager>,
}
```

- [ ] **Step 3: 在 `src-tauri/src/lib.rs` 顶部新增模块声明**

```rust
mod config;
mod events;
mod proxy;
mod state;
```

- [ ] **Step 4: 编译**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo build
```

预期：成功。

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(backend): add events constants and shared AppState"
```

---

### Task 11：Tauri 命令

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建 `src-tauri/src/commands.rs`**

```rust
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, Wry};

use crate::config::{
    self, Endpoint, GlobalConfig, ServerStatus,
};
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
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?;
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
```

- [ ] **Step 2: 编译**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo build
```

预期：成功。

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "feat(backend): add Tauri commands for endpoint and server management"
```

---

### Task 12：托盘与窗口集成 + lib.rs 重写

**Files:**
- Create: `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建 `src-tauri/src/tray.rs`**

```rust
use tauri::{
    AppHandle, Emitter, Manager, Wry,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
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

    let _tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
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
    let state = app.state::<AppState>();
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
    let state = app.state::<AppState>();
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
```

- [ ] **Step 2: 重写 `src-tauri/src/lib.rs`**

```rust
mod commands;
mod config;
mod events;
mod proxy;
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
            let store = Arc::new(ConfigStore::load(&handle)?);
            let g = store.global();
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
                    let status = mgr_for_start
                        .start(g.listen_address.clone(), g.listen_port)
                        .await
                        .unwrap_or_else(|e| {
                            let _ = handle_for_start
                                .emit(events::CONFIG_ERROR, &serde_json::json!({"message": e}));
                            mgr_for_start.status()
                        });
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
```

- [ ] **Step 3: 编译**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo build
```

预期：成功（warnings OK）。

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "feat(backend): wire tray, lifecycle, and auto-start server"
```

---

### Task 13：后端集成测试

**Files:**
- Create: `src-tauri/tests/proxy_integration.rs`

- [ ] **Step 1: 创建 `src-tauri/tests/proxy_integration.rs`**

```rust
use std::net::SocketAddr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;

use apiproxy_lib as _; // 触发链接
use arc_swap::ArcSwap;

// 直接复用内部模块——通过 path 引用源文件
#[path = "../src/config/mod.rs"]
mod config;
#[path = "../src/proxy/mod.rs"]
mod proxy;

use config::{Endpoint, Rule, RuleAction};
use proxy::server::spawn_server;

async fn fake_upstream() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    use axum::extract::Request;
    use axum::routing::any;
    use axum::Router;

    let app = Router::new().fallback(any(|req: Request| async move {
        let method = req.method().clone();
        let path = req.uri().path().to_string();
        let query = req.uri().query().unwrap_or("").to_string();
        let body = axum::body::to_bytes(req.into_body(), 1024 * 1024)
            .await
            .unwrap_or_default();
        let body_str = String::from_utf8_lossy(&body).to_string();
        axum::Json(serde_json::json!({
            "method": method.as_str(),
            "path": path,
            "query": query,
            "body": body_str,
        }))
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let join = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (addr, join)
}

fn ep(path: &str, upstream: &str, body_merge: &str) -> Endpoint {
    Endpoint {
        id: "id".into(),
        name: "ep".into(),
        description: "".into(),
        enabled: true,
        path: path.into(),
        upstream_url: upstream.into(),
        strip_prefix: true,
        header_rules: vec![],
        query_rules: vec![],
        body_merge: body_merge.into(),
        created_at: 0,
        updated_at: 0,
    }
}

#[tokio::test]
async fn forwards_basic_get() {
    let (upstream_addr, _u) = fake_upstream().await;
    let upstream = format!("http://{}/v1", upstream_addr);
    let routes = Arc::new(ArcSwap::from_pointee(vec![ep("/cc", &upstream, "")]));
    let timeout = Arc::new(AtomicU64::new(10));
    let client = reqwest::Client::new();
    let server = spawn_server("127.0.0.1".into(), 0, routes, timeout, client)
        .await
        .unwrap();
    let url = format!("http://{}:{}/cc/hello", server.address, server.port);

    let resp = reqwest::get(&url).await.unwrap();
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["method"], "GET");
    assert_eq!(json["path"], "/v1/hello");

    let _ = server.shutdown.send(());
    server.join.await.ok();
}

#[tokio::test]
async fn forwards_post_with_body_merge() {
    let (upstream_addr, _u) = fake_upstream().await;
    let upstream = format!("http://{}/v1", upstream_addr);
    let mut endpoint = ep("/cc", &upstream, r#"{"model":"x"}"#);
    endpoint.header_rules = vec![Rule {
        action: RuleAction::Set,
        key: "x-test".into(),
        value: "1".into(),
    }];
    let routes = Arc::new(ArcSwap::from_pointee(vec![endpoint]));
    let timeout = Arc::new(AtomicU64::new(10));
    let client = reqwest::Client::new();
    let server = spawn_server("127.0.0.1".into(), 0, routes, timeout, client.clone())
        .await
        .unwrap();
    let url = format!("http://{}:{}/cc/chat", server.address, server.port);
    let resp = client
        .post(&url)
        .header("content-type", "application/json")
        .body(r#"{"q":"hi","model":"orig"}"#)
        .send()
        .await
        .unwrap();
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["method"], "POST");
    assert_eq!(json["path"], "/v1/chat");
    let body_obj: serde_json::Value = serde_json::from_str(json["body"].as_str().unwrap()).unwrap();
    assert_eq!(body_obj["q"], "hi");
    assert_eq!(body_obj["model"], "x"); // patch wins

    let _ = server.shutdown.send(());
    server.join.await.ok();
}

#[tokio::test]
async fn returns_404_when_no_match() {
    let routes = Arc::new(ArcSwap::from_pointee(vec![]));
    let timeout = Arc::new(AtomicU64::new(5));
    let server = spawn_server("127.0.0.1".into(), 0, routes, timeout, reqwest::Client::new())
        .await
        .unwrap();
    let url = format!("http://{}:{}/missing", server.address, server.port);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 404);
    let _ = server.shutdown.send(());
    server.join.await.ok();
}

#[tokio::test]
async fn upstream_timeout_returns_502() {
    use std::convert::Infallible;
    use std::net::SocketAddr;
    use axum::routing::any;
    use axum::Router;

    let app: Router = Router::new().fallback(any(|| async {
        tokio::time::sleep(Duration::from_secs(5)).await;
        "ok"
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_addr = listener.local_addr().unwrap();
    let _upstream = tokio::spawn(async move {
        let _: Result<(), Infallible> = {
            let _ = axum::serve(listener, app).await;
            Ok(())
        };
    });
    let _: SocketAddr = upstream_addr;

    let upstream = format!("http://{}", upstream_addr);
    let routes = Arc::new(ArcSwap::from_pointee(vec![ep("/cc", &upstream, "")]));
    let timeout = Arc::new(AtomicU64::new(1));
    let server = spawn_server("127.0.0.1".into(), 0, routes, timeout, reqwest::Client::new())
        .await
        .unwrap();
    let url = format!("http://{}:{}/cc/slow", server.address, server.port);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 502);
    let _ = server.shutdown.send(());
    server.join.await.ok();
}
```

- [ ] **Step 2: 运行测试**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy/src-tauri
cargo test --test proxy_integration -- --nocapture
```

预期：全部通过（每个测试 1-2 秒）。

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "test(backend): add proxy integration tests"
```

---

## Phase 3：前端 — UI 基础组件

### Task 14：utils + types + api 封装

**Files:**
- Create: `src/lib/utils.ts`
- Create: `src/lib/platform.ts`
- Create: `src/types.ts`
- Create: `src/lib/api.ts`
- Create: `src/lib/schemas.ts`

- [ ] **Step 1: 创建 `src/lib/utils.ts`**

```ts
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

- [ ] **Step 2: 创建 `src/lib/platform.ts`**

```ts
export const isMac = () => navigator.userAgent.includes("Mac");
export const isWindows = () => navigator.userAgent.includes("Windows");
export const isLinux = () => navigator.userAgent.includes("Linux") && !isMac();

export const DRAG_REGION_ENABLED = !isLinux();
```

- [ ] **Step 3: 创建 `src/types.ts`**

```ts
export type RuleAction = "set" | "add" | "remove";

export interface Rule {
  action: RuleAction;
  key: string;
  value: string;
}

export interface Endpoint {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  path: string;
  upstreamUrl: string;
  stripPrefix: boolean;
  headerRules: Rule[];
  queryRules: Rule[];
  bodyMerge: string;
  createdAt: number;
  updatedAt: number;
}

export interface GlobalConfig {
  listenAddress: string;
  listenPort: number;
  requestTimeoutSecs: number;
  theme: "light" | "dark" | "system";
  closeToTray: boolean;
  autoStartServer: boolean;
}

export interface ServerStatus {
  running: boolean;
  listenAddress?: string;
  listenPort?: number;
  startedAt?: number;
  lastError?: string;
}

export interface InitPayload {
  global: GlobalConfig;
  endpoints: Endpoint[];
  status: ServerStatus;
}
```

- [ ] **Step 4: 创建 `src/lib/api.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";
import type { Endpoint, GlobalConfig, InitPayload, ServerStatus } from "@/types";

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
};

export const EVENTS = {
  STATUS_CHANGED: "proxy://status-changed",
  ENDPOINTS_CHANGED: "proxy://endpoints-changed",
  CONFIG_ERROR: "proxy://config-error",
} as const;
```

- [ ] **Step 5: 创建 `src/lib/schemas.ts`**

```ts
import { z } from "zod";

export const ruleActionSchema = z.enum(["set", "add", "remove"]);

export const ruleSchema = z.object({
  action: ruleActionSchema,
  key: z.string().min(1, "请填写 Key"),
  value: z.string(),
});

export const endpointFormSchema = z.object({
  id: z.string(),
  name: z.string().min(1, "名称必填").max(60, "名称不超过 60 字符"),
  description: z.string().max(200, "描述不超过 200 字符"),
  enabled: z.boolean(),
  path: z
    .string()
    .min(2, "路径不能为空")
    .regex(/^\/[A-Za-z0-9._\-/]+$/, "路径只能包含字母、数字、_、-、. 和 /")
    .refine((s) => s !== "/", "路径不能为根 /"),
  upstreamUrl: z
    .string()
    .url("请输入合法 URL")
    .refine(
      (s) => s.startsWith("http://") || s.startsWith("https://"),
      "仅支持 http/https",
    ),
  stripPrefix: z.boolean(),
  headerRules: z.array(ruleSchema),
  queryRules: z.array(ruleSchema),
  bodyMerge: z
    .string()
    .refine((s) => {
      if (!s.trim()) return true;
      try {
        const v = JSON.parse(s);
        return typeof v === "object" && v !== null && !Array.isArray(v);
      } catch {
        return false;
      }
    }, "必须为合法的 JSON 对象"),
});

export type EndpointForm = z.infer<typeof endpointFormSchema>;

export const globalConfigSchema = z.object({
  listenAddress: z.string().min(1, "请填写监听地址"),
  listenPort: z.number().int().min(1).max(65535),
  requestTimeoutSecs: z.number().int().min(1).max(600),
  theme: z.enum(["light", "dark", "system"]),
  closeToTray: z.boolean(),
  autoStartServer: z.boolean(),
});
```

- [ ] **Step 6: typecheck**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy
pnpm typecheck
```

预期：通过。

- [ ] **Step 7: 提交**

```bash
git add -A
git commit -m "feat(frontend): add utils, types, api wrapper, and zod schemas"
```

---

### Task 15：UI 基础组件 Part 1（Button / Input / Textarea / Label）

**Files:**
- Create: `src/components/ui/button.tsx`
- Create: `src/components/ui/input.tsx`
- Create: `src/components/ui/textarea.tsx`
- Create: `src/components/ui/label.tsx`

- [ ] **Step 1: 创建 `src/components/ui/button.tsx`**

```tsx
import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default:
          "bg-blue-500 text-white hover:bg-blue-600 dark:bg-blue-600 dark:hover:bg-blue-700",
        destructive:
          "bg-red-500 text-white hover:bg-red-600 dark:bg-red-600 dark:hover:bg-red-700",
        outline:
          "border bg-background text-muted-foreground hover:bg-gray-100 hover:text-gray-900 dark:hover:bg-gray-800 dark:hover:text-gray-100",
        secondary:
          "text-gray-500 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-800 dark:hover:text-gray-200",
        ghost:
          "text-gray-500 hover:text-gray-900 hover:bg-gray-100 dark:text-gray-400 dark:hover:text-gray-100 dark:hover:bg-gray-800",
        success:
          "bg-emerald-500 text-white hover:bg-emerald-600 dark:bg-emerald-600 dark:hover:bg-emerald-700",
        link:
          "text-blue-500 underline-offset-4 hover:underline dark:text-blue-400",
      },
      size: {
        default: "h-9 px-4 py-2",
        sm: "h-8 rounded-md px-3 text-xs",
        lg: "h-10 rounded-md px-8",
        icon: "h-9 w-9 p-1.5",
      },
    },
    defaultVariants: { variant: "default", size: "default" },
  },
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    );
  },
);
Button.displayName = "Button";

export { Button, buttonVariants };
```

- [ ] **Step 2: 创建 `src/components/ui/input.tsx`**

```tsx
import * as React from "react";
import { cn } from "@/lib/utils";

export interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, type, ...props }, ref) => (
    <input
      type={type}
      ref={ref}
      className={cn(
        "flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm shadow-sm transition-colors",
        "placeholder:text-muted-foreground",
        "focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:focus:ring-blue-400/20",
        "disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      {...props}
    />
  ),
);
Input.displayName = "Input";

export { Input };
```

- [ ] **Step 3: 创建 `src/components/ui/textarea.tsx`**

```tsx
import * as React from "react";
import { cn } from "@/lib/utils";

export type TextareaProps = React.TextareaHTMLAttributes<HTMLTextAreaElement>;

const Textarea = React.forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ className, ...props }, ref) => (
    <textarea
      ref={ref}
      autoComplete="off"
      autoCorrect="off"
      spellCheck={false}
      className={cn(
        "flex min-h-[80px] w-full rounded-md border bg-background px-3 py-2 text-sm shadow-sm transition-colors",
        "placeholder:text-muted-foreground",
        "focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:focus:ring-blue-400/20",
        "disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      {...props}
    />
  ),
);
Textarea.displayName = "Textarea";

export { Textarea };
```

- [ ] **Step 4: 创建 `src/components/ui/label.tsx`**

```tsx
import * as React from "react";
import * as LabelPrimitive from "@radix-ui/react-label";
import { cn } from "@/lib/utils";

const Label = React.forwardRef<
  React.ElementRef<typeof LabelPrimitive.Root>,
  React.ComponentPropsWithoutRef<typeof LabelPrimitive.Root>
>(({ className, ...props }, ref) => (
  <LabelPrimitive.Root
    ref={ref}
    className={cn("text-sm font-medium leading-none", className)}
    {...props}
  />
));
Label.displayName = "Label";

export { Label };
```

- [ ] **Step 5: typecheck**

```bash
pnpm typecheck
```

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "feat(frontend): add Button, Input, Textarea, Label primitives"
```

---

### Task 16：UI 基础组件 Part 2（Switch / Select / Dialog）

**Files:**
- Create: `src/components/ui/switch.tsx`
- Create: `src/components/ui/select.tsx`
- Create: `src/components/ui/dialog.tsx`

- [ ] **Step 1: 创建 `src/components/ui/switch.tsx`**

```tsx
import * as React from "react";
import * as SwitchPrimitive from "@radix-ui/react-switch";
import { cn } from "@/lib/utils";

const Switch = React.forwardRef<
  React.ElementRef<typeof SwitchPrimitive.Root>,
  React.ComponentPropsWithoutRef<typeof SwitchPrimitive.Root>
>(({ className, ...props }, ref) => (
  <SwitchPrimitive.Root
    ref={ref}
    className={cn(
      "peer inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent shadow transition-colors",
      "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
      "disabled:cursor-not-allowed disabled:opacity-50",
      "data-[state=checked]:bg-emerald-500",
      "data-[state=unchecked]:bg-gray-200 dark:data-[state=unchecked]:bg-gray-900",
      className,
    )}
    {...props}
  >
    <SwitchPrimitive.Thumb
      className={cn(
        "pointer-events-none block h-5 w-5 rounded-full bg-white dark:bg-gray-400 shadow-lg ring-0 transition-transform",
        "data-[state=checked]:translate-x-5 data-[state=unchecked]:translate-x-0",
      )}
    />
  </SwitchPrimitive.Root>
));
Switch.displayName = "Switch";

export { Switch };
```

- [ ] **Step 2: 创建 `src/components/ui/select.tsx`**

```tsx
import * as React from "react";
import * as SelectPrimitive from "@radix-ui/react-select";
import { Check, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";

const Select = SelectPrimitive.Root;
const SelectGroup = SelectPrimitive.Group;
const SelectValue = SelectPrimitive.Value;

const SelectTrigger = React.forwardRef<
  React.ElementRef<typeof SelectPrimitive.Trigger>,
  React.ComponentPropsWithoutRef<typeof SelectPrimitive.Trigger>
>(({ className, children, ...props }, ref) => (
  <SelectPrimitive.Trigger
    ref={ref}
    className={cn(
      "flex h-9 w-full items-center justify-between rounded-md border bg-background px-3 py-1 text-sm shadow-sm",
      "placeholder:text-muted-foreground",
      "focus:outline-none focus:ring-2 focus:ring-blue-500/20",
      "disabled:cursor-not-allowed disabled:opacity-50",
      className,
    )}
    {...props}
  >
    {children}
    <SelectPrimitive.Icon asChild>
      <ChevronDown className="h-4 w-4 opacity-50" />
    </SelectPrimitive.Icon>
  </SelectPrimitive.Trigger>
));
SelectTrigger.displayName = "SelectTrigger";

const SelectContent = React.forwardRef<
  React.ElementRef<typeof SelectPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof SelectPrimitive.Content>
>(({ className, children, position = "popper", ...props }, ref) => (
  <SelectPrimitive.Portal>
    <SelectPrimitive.Content
      ref={ref}
      position={position}
      className={cn(
        "relative z-[100] max-h-96 min-w-[8rem] overflow-hidden rounded-md border bg-popover text-popover-foreground shadow-md",
        "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0",
        position === "popper" && "translate-y-1",
        className,
      )}
      {...props}
    >
      <SelectPrimitive.Viewport
        className={cn(
          "p-1",
          position === "popper" && "w-full min-w-[var(--radix-select-trigger-width)]",
        )}
      >
        {children}
      </SelectPrimitive.Viewport>
    </SelectPrimitive.Content>
  </SelectPrimitive.Portal>
));
SelectContent.displayName = "SelectContent";

const SelectItem = React.forwardRef<
  React.ElementRef<typeof SelectPrimitive.Item>,
  React.ComponentPropsWithoutRef<typeof SelectPrimitive.Item>
>(({ className, children, ...props }, ref) => (
  <SelectPrimitive.Item
    ref={ref}
    className={cn(
      "relative flex w-full cursor-default select-none items-center rounded-sm py-1.5 pl-7 pr-2 text-sm outline-none",
      "focus:bg-accent focus:text-accent-foreground",
      "data-[disabled]:pointer-events-none data-[disabled]:opacity-50",
      className,
    )}
    {...props}
  >
    <span className="absolute left-2 flex h-3.5 w-3.5 items-center justify-center">
      <SelectPrimitive.ItemIndicator>
        <Check className="h-4 w-4" />
      </SelectPrimitive.ItemIndicator>
    </span>
    <SelectPrimitive.ItemText>{children}</SelectPrimitive.ItemText>
  </SelectPrimitive.Item>
));
SelectItem.displayName = "SelectItem";

export {
  Select,
  SelectGroup,
  SelectValue,
  SelectTrigger,
  SelectContent,
  SelectItem,
};
```

- [ ] **Step 3: 创建 `src/components/ui/dialog.tsx`**

```tsx
import * as React from "react";
import * as DialogPrimitive from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import { cn } from "@/lib/utils";

const Dialog = DialogPrimitive.Root;
const DialogTrigger = DialogPrimitive.Trigger;
const DialogPortal = DialogPrimitive.Portal;
const DialogClose = DialogPrimitive.Close;

const DialogOverlay = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Overlay>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Overlay>
>(({ className, ...props }, ref) => (
  <DialogPrimitive.Overlay
    ref={ref}
    className={cn(
      "fixed inset-0 z-40 bg-black/50 backdrop-blur-sm",
      "data-[state=open]:animate-in data-[state=closed]:animate-out",
      "data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0",
      className,
    )}
    {...props}
  />
));
DialogOverlay.displayName = "DialogOverlay";

const DialogContent = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Content> & {
    hideClose?: boolean;
  }
>(({ className, children, hideClose, ...props }, ref) => (
  <DialogPortal>
    <DialogOverlay />
    <DialogPrimitive.Content
      ref={ref}
      onInteractOutside={(e) => e.preventDefault()}
      className={cn(
        "fixed left-1/2 top-1/2 z-50 grid w-full max-w-lg max-h-[90vh] -translate-x-1/2 -translate-y-1/2 gap-4",
        "border bg-background p-6 text-foreground shadow-lg sm:rounded-lg",
        "data-[state=open]:animate-in data-[state=closed]:animate-out",
        "data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0",
        "data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95",
        className,
      )}
      {...props}
    >
      {children}
      {!hideClose && (
        <DialogPrimitive.Close className="absolute right-4 top-4 rounded-sm opacity-70 transition-opacity hover:opacity-100">
          <X className="h-4 w-4" />
          <span className="sr-only">关闭</span>
        </DialogPrimitive.Close>
      )}
    </DialogPrimitive.Content>
  </DialogPortal>
));
DialogContent.displayName = "DialogContent";

const DialogHeader = ({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) => (
  <div className={cn("flex flex-col space-y-1.5", className)} {...props} />
);

const DialogFooter = ({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) => (
  <div
    className={cn("flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2", className)}
    {...props}
  />
);

const DialogTitle = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Title>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Title>
>(({ className, ...props }, ref) => (
  <DialogPrimitive.Title
    ref={ref}
    className={cn("text-lg font-semibold leading-tight tracking-tight", className)}
    {...props}
  />
));
DialogTitle.displayName = "DialogTitle";

const DialogDescription = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Description>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Description>
>(({ className, ...props }, ref) => (
  <DialogPrimitive.Description
    ref={ref}
    className={cn("text-sm text-muted-foreground", className)}
    {...props}
  />
));
DialogDescription.displayName = "DialogDescription";

export {
  Dialog,
  DialogTrigger,
  DialogPortal,
  DialogOverlay,
  DialogClose,
  DialogContent,
  DialogHeader,
  DialogFooter,
  DialogTitle,
  DialogDescription,
};
```

- [ ] **Step 4: typecheck**

```bash
pnpm typecheck
```

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(frontend): add Switch, Select, Dialog primitives"
```

---

### Task 17：UI 基础组件 Part 3（Badge / Sonner / FullScreenPanel）

**Files:**
- Create: `src/components/ui/badge.tsx`
- Create: `src/components/ui/sonner.tsx`
- Create: `src/components/common/FullScreenPanel.tsx`

- [ ] **Step 1: 创建 `src/components/ui/badge.tsx`**

```tsx
import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const badgeVariants = cva(
  "inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide",
  {
    variants: {
      tone: {
        violet:
          "bg-violet-100 text-violet-700 dark:bg-violet-900/40 dark:text-violet-300",
        slate:
          "bg-slate-200 text-slate-700 dark:bg-slate-700/60 dark:text-slate-200",
        emerald:
          "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-300",
        red:
          "bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-300",
        sky:
          "bg-sky-100 text-sky-700 dark:bg-sky-900/40 dark:text-sky-300",
        amber:
          "bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300",
      },
    },
    defaultVariants: { tone: "slate" },
  },
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLSpanElement>,
    VariantProps<typeof badgeVariants> {}

export function Badge({ className, tone, ...props }: BadgeProps) {
  return <span className={cn(badgeVariants({ tone }), className)} {...props} />;
}
```

- [ ] **Step 2: 创建 `src/components/ui/sonner.tsx`**

```tsx
import { Toaster as SonnerToaster } from "sonner";

export function Toaster() {
  return (
    <SonnerToaster
      position="top-center"
      richColors
      duration={2000}
      toastOptions={{
        classNames: {
          toast:
            "group rounded-md border bg-background text-foreground shadow-lg",
        },
      }}
    />
  );
}
```

- [ ] **Step 3: 创建 `src/components/common/FullScreenPanel.tsx`**

```tsx
import { AnimatePresence, motion } from "framer-motion";
import { ArrowLeft } from "lucide-react";
import * as React from "react";
import { createPortal } from "react-dom";
import { Button } from "@/components/ui/button";
import { DRAG_REGION_ENABLED, isMac } from "@/lib/platform";
import { cn } from "@/lib/utils";

interface FullScreenPanelProps {
  open: boolean;
  onClose: () => void;
  title: string;
  footer?: React.ReactNode;
  children: React.ReactNode;
}

export function FullScreenPanel({
  open,
  onClose,
  title,
  footer,
  children,
}: FullScreenPanelProps) {
  React.useEffect(() => {
    if (!open) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const handler = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      const active = document.activeElement as HTMLElement | null;
      if (
        active &&
        (active.tagName === "INPUT" ||
          active.tagName === "TEXTAREA" ||
          active.isContentEditable)
      ) {
        return;
      }
      e.preventDefault();
      onClose();
    };
    window.addEventListener("keydown", handler);
    return () => {
      document.body.style.overflow = prev;
      window.removeEventListener("keydown", handler);
    };
  }, [open, onClose]);

  if (typeof document === "undefined") return null;

  const dragBarHeight = isMac() && DRAG_REGION_ENABLED ? 28 : 0;

  return createPortal(
    <AnimatePresence>
      {open && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.2 }}
          className="fixed inset-0 z-[60] flex flex-col bg-background"
        >
          {dragBarHeight > 0 && (
            <div
              data-tauri-drag-region
              style={{ height: dragBarHeight }}
              className="w-full shrink-0"
            />
          )}
          <header
            data-tauri-drag-region
            className={cn(
              "flex h-16 shrink-0 items-center gap-3 px-6 border-b",
              "bg-background/95 backdrop-blur",
            )}
          >
            <Button
              variant="outline"
              size="icon"
              className="rounded-lg"
              onClick={onClose}
              data-tauri-no-drag
            >
              <ArrowLeft className="h-4 w-4" />
            </Button>
            <h1 className="text-lg font-semibold">{title}</h1>
          </header>
          <main className="flex-1 overflow-y-auto px-6 py-6 space-y-6">
            {children}
          </main>
          {footer && (
            <footer className="flex shrink-0 items-center justify-end gap-2 px-6 py-4 border-t bg-background/95 backdrop-blur">
              {footer}
            </footer>
          )}
        </motion.div>
      )}
    </AnimatePresence>,
    document.body,
  );
}
```

- [ ] **Step 4: typecheck**

```bash
pnpm typecheck
```

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(frontend): add Badge, Toaster, and FullScreenPanel"
```

---

### Task 18：主题 Provider 与切换按钮

**Files:**
- Create: `src/components/theme-provider.tsx`
- Create: `src/components/mode-toggle.tsx`

- [ ] **Step 1: 创建 `src/components/theme-provider.tsx`**

```tsx
import * as React from "react";

type Theme = "light" | "dark" | "system";
const STORAGE_KEY = "apiproxy-theme";

interface ThemeContextValue {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  resolved: "light" | "dark";
}

const ThemeContext = React.createContext<ThemeContextValue | undefined>(undefined);

export function ThemeProvider({
  children,
  defaultTheme = "system",
}: {
  children: React.ReactNode;
  defaultTheme?: Theme;
}) {
  const [theme, setThemeState] = React.useState<Theme>(
    () => (localStorage.getItem(STORAGE_KEY) as Theme) || defaultTheme,
  );
  const [resolved, setResolved] = React.useState<"light" | "dark">("light");

  React.useEffect(() => {
    const apply = () => {
      const root = document.documentElement;
      let actual: "light" | "dark" = "light";
      if (theme === "system") {
        actual = window.matchMedia("(prefers-color-scheme: dark)").matches
          ? "dark"
          : "light";
      } else {
        actual = theme;
      }
      root.classList.remove("light", "dark");
      root.classList.add(actual);
      setResolved(actual);
    };
    apply();
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [theme]);

  const setTheme = React.useCallback((t: Theme) => {
    localStorage.setItem(STORAGE_KEY, t);
    setThemeState(t);
  }, []);

  return (
    <ThemeContext.Provider value={{ theme, setTheme, resolved }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const ctx = React.useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
  return ctx;
}
```

- [ ] **Step 2: 创建 `src/components/mode-toggle.tsx`**

```tsx
import { Moon, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useTheme } from "@/components/theme-provider";

export function ModeToggle() {
  const { resolved, setTheme } = useTheme();
  const next = resolved === "dark" ? "light" : "dark";
  return (
    <Button
      variant="ghost"
      size="icon"
      onClick={() => setTheme(next)}
      title={resolved === "dark" ? "切到亮色" : "切到深色"}
      className="h-8 w-8"
    >
      {resolved === "dark" ? (
        <Sun className="h-4 w-4" />
      ) : (
        <Moon className="h-4 w-4" />
      )}
    </Button>
  );
}
```

- [ ] **Step 3: typecheck**

```bash
pnpm typecheck
```

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "feat(frontend): add theme provider and mode toggle"
```

---

## Phase 4：前端 — 业务页面

### Task 19：App 壳与全局状态

**Files:**
- Modify: `src/main.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: 重写 `src/main.tsx`**

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ThemeProvider } from "@/components/theme-provider";
import { Toaster } from "@/components/ui/sonner";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ThemeProvider defaultTheme="system">
      <App />
      <Toaster />
    </ThemeProvider>
  </React.StrictMode>,
);
```

- [ ] **Step 2: 重写 `src/App.tsx` 为壳（不含列表实现，列表占位）**

```tsx
import * as React from "react";
import { Plus, Settings as SettingsIcon } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { ModeToggle } from "@/components/mode-toggle";
import { useTheme } from "@/components/theme-provider";
import { api, EVENTS } from "@/lib/api";
import { isMac } from "@/lib/platform";
import type { Endpoint, GlobalConfig, ServerStatus } from "@/types";

import { ServerToggle } from "@/components/ServerToggle";
import { ServerStatusBadge } from "@/components/ServerStatusBadge";
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
    getCurrentWindow().show().catch(() => undefined);
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
          <div className="flex items-center gap-2">
            <span className="text-xl font-semibold text-blue-500 dark:text-blue-400">
              API 代理
            </span>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={() => setShowSettings(true)}
              data-tauri-no-drag
              title="设置"
            >
              <SettingsIcon className="h-4 w-4" />
            </Button>
          </div>
          <div className="flex items-center gap-3" data-tauri-no-drag>
            <ServerStatusBadge status={status} />
            <ServerToggle status={status} />
            <ModeToggle />
            <Button onClick={() => setShowAdd(true)} size="icon" className="ml-2">
              <Plus className="h-5 w-5" />
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

- [ ] **Step 3: typecheck** — 会报错（缺很多组件），是预期，下面任务补齐。
- [ ] **Step 4: 提交**（暂时跳过 typecheck）

```bash
git add -A
git commit -m "feat(frontend): app shell with header, state, and event listeners"
```

---

### Task 20：服务开关与状态徽章

**Files:**
- Create: `src/components/ServerToggle.tsx`
- Create: `src/components/ServerStatusBadge.tsx`

- [ ] **Step 1: 创建 `src/components/ServerToggle.tsx`**

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
    } catch (e: any) {
      toast.error(typeof e === "string" ? e : e?.message ?? "操作失败");
    }
  };
  return (
    <div className="flex items-center gap-2">
      <span className="text-xs text-muted-foreground">服务</span>
      <Switch checked={status.running} onCheckedChange={handleChange} />
    </div>
  );
}
```

- [ ] **Step 2: 创建 `src/components/ServerStatusBadge.tsx`**

```tsx
import { Copy } from "lucide-react";
import { toast } from "sonner";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { ServerStatus } from "@/types";

export function ServerStatusBadge({ status }: { status: ServerStatus }) {
  const address = status.listenAddress
    ? `${status.listenAddress}:${status.listenPort}`
    : null;
  const copy = async () => {
    if (!address) return;
    await writeText(`http://${address}`);
    toast.success("已复制");
  };
  return (
    <div className="flex items-center gap-2">
      <Badge tone={status.running ? "emerald" : "slate"}>
        {status.running ? "运行中" : "已停止"}
      </Badge>
      {address && (
        <button
          onClick={copy}
          className={cn(
            "inline-flex items-center gap-1 font-mono text-xs",
            "text-muted-foreground hover:text-foreground transition-colors",
          )}
          title="点击复制"
        >
          {address}
          <Copy className="h-3 w-3" />
        </button>
      )}
    </div>
  );
}
```

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "feat(frontend): add ServerToggle and ServerStatusBadge"
```

---

### Task 21：端点列表 + 空状态 + 卡片

**Files:**
- Create: `src/components/endpoints/EndpointEmptyState.tsx`
- Create: `src/components/endpoints/EndpointList.tsx`
- Create: `src/components/endpoints/EndpointCard.tsx`
- Create: `src/components/endpoints/EndpointActions.tsx`
- Create: `src/components/ConfirmDialog.tsx`

- [ ] **Step 1: 创建 `src/components/endpoints/EndpointEmptyState.tsx`**

```tsx
import { Plus, Route } from "lucide-react";
import { Button } from "@/components/ui/button";

export function EndpointEmptyState({ onAdd }: { onAdd: () => void }) {
  return (
    <div className="m-6 flex flex-col items-center justify-center rounded-lg border border-dashed p-10 text-center">
      <div className="mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-muted">
        <Route className="h-7 w-7 text-muted-foreground" />
      </div>
      <h3 className="text-lg font-semibold">还没有任何端点</h3>
      <p className="mt-2 max-w-lg text-sm text-muted-foreground">
        点击下面的按钮创建第一个端点，把请求转发到上游 API。
      </p>
      <div className="mt-6">
        <Button onClick={onAdd}>
          <Plus className="h-4 w-4" />
          添加端点
        </Button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 创建 `src/components/endpoints/EndpointList.tsx`**

```tsx
import { EndpointCard } from "./EndpointCard";
import { EndpointEmptyState } from "./EndpointEmptyState";
import type { Endpoint, ServerStatus } from "@/types";

interface Props {
  endpoints: Endpoint[];
  status: ServerStatus;
  listenAddress: string;
  listenPort: number;
  onEdit: (e: Endpoint) => void;
  onAdd: () => void;
}

export function EndpointList({
  endpoints,
  status,
  listenAddress,
  listenPort,
  onEdit,
  onAdd,
}: Props) {
  if (endpoints.length === 0) {
    return <EndpointEmptyState onAdd={onAdd} />;
  }
  return (
    <div className="mt-4 space-y-4 px-6">
      {endpoints.map((e) => (
        <EndpointCard
          key={e.id}
          endpoint={e}
          serverRunning={status.running}
          listenAddress={listenAddress}
          listenPort={listenPort}
          onEdit={() => onEdit(e)}
        />
      ))}
    </div>
  );
}
```

- [ ] **Step 3: 创建 `src/components/endpoints/EndpointCard.tsx`**

```tsx
import { GripVertical, Route } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { Endpoint } from "@/types";
import { EndpointActions } from "./EndpointActions";

interface Props {
  endpoint: Endpoint;
  serverRunning: boolean;
  listenAddress: string;
  listenPort: number;
  onEdit: () => void;
}

export function EndpointCard({
  endpoint,
  serverRunning,
  listenAddress,
  listenPort,
  onEdit,
}: Props) {
  const localUrl = `http://${listenAddress}:${listenPort}${endpoint.path}`;
  const showInactive = endpoint.enabled && !serverRunning;
  return (
    <div
      className={cn(
        "relative overflow-hidden rounded-xl border p-4 transition-all duration-300",
        "bg-card text-card-foreground group hover:border-border-active hover:shadow-sm",
        endpoint.enabled &&
          "border-emerald-500/60 shadow-sm shadow-emerald-500/10",
      )}
    >
      {endpoint.enabled && (
        <div className="absolute inset-0 bg-gradient-to-r from-emerald-500/10 to-transparent pointer-events-none" />
      )}
      <div className="relative flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex flex-1 items-center gap-2">
          <button
            className="-ml-1.5 flex-shrink-0 cursor-grab p-1.5 text-muted-foreground/50 hover:text-muted-foreground"
            tabIndex={-1}
            aria-hidden
          >
            <GripVertical className="h-4 w-4" />
          </button>
          <div className="h-8 w-8 rounded-lg bg-muted flex items-center justify-center border">
            <Route className="h-5 w-5 text-muted-foreground" />
          </div>
          <div className="space-y-1 min-w-0">
            <div className="flex flex-wrap items-center gap-2 min-h-7">
              <h3 className="text-base font-semibold leading-none truncate">
                {endpoint.name}
              </h3>
              <Badge tone={endpoint.enabled ? "violet" : "slate"}>
                {endpoint.enabled ? "已启用" : "已停用"}
              </Badge>
              {showInactive && <Badge tone="amber">服务未运行</Badge>}
            </div>
            <div className="font-mono text-sm truncate max-w-[480px]">
              <span className="text-blue-500 dark:text-blue-400">
                {endpoint.path}
              </span>
              <span className="mx-2 text-muted-foreground">→</span>
              <span className="text-muted-foreground">{endpoint.upstreamUrl}</span>
            </div>
            {endpoint.description && (
              <div className="text-sm text-muted-foreground truncate max-w-[480px]">
                {endpoint.description}
              </div>
            )}
          </div>
        </div>
        <div
          className={cn(
            "flex items-center gap-1.5 flex-shrink-0",
            "opacity-0 pointer-events-none",
            "group-hover:opacity-100 group-focus-within:opacity-100",
            "group-hover:pointer-events-auto group-focus-within:pointer-events-auto",
            "transition-opacity duration-200",
          )}
        >
          <EndpointActions
            endpoint={endpoint}
            localUrl={localUrl}
            onEdit={onEdit}
          />
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 4: 创建 `src/components/endpoints/EndpointActions.tsx`**

```tsx
import * as React from "react";
import { Copy, Pause, Play, Trash2, Pencil } from "lucide-react";
import { toast } from "sonner";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { api } from "@/lib/api";
import type { Endpoint } from "@/types";

interface Props {
  endpoint: Endpoint;
  localUrl: string;
  onEdit: () => void;
}

const iconBtn = "h-8 w-8 p-1";

export function EndpointActions({ endpoint, localUrl, onEdit }: Props) {
  const [pendingDelete, setPendingDelete] = React.useState(false);

  const toggle = async () => {
    try {
      await api.toggleEndpoint(endpoint.id, !endpoint.enabled);
      toast.success(endpoint.enabled ? "已停用" : "已启用");
    } catch (e: any) {
      toast.error(typeof e === "string" ? e : e?.message ?? "操作失败");
    }
  };

  const copy = async () => {
    await writeText(localUrl);
    toast.success("已复制");
  };

  const remove = async () => {
    setPendingDelete(false);
    try {
      await api.deleteEndpoint(endpoint.id);
      toast.success("已删除");
    } catch (e: any) {
      toast.error(typeof e === "string" ? e : e?.message ?? "删除失败");
    }
  };

  return (
    <>
      <Button
        variant="ghost"
        size="icon"
        className={iconBtn}
        onClick={toggle}
        title={endpoint.enabled ? "停用" : "启用"}
      >
        {endpoint.enabled ? (
          <Pause className="h-4 w-4" />
        ) : (
          <Play className="h-4 w-4" />
        )}
      </Button>
      <Button variant="ghost" size="icon" className={iconBtn} onClick={copy} title="复制 URL">
        <Copy className="h-4 w-4" />
      </Button>
      <Button variant="ghost" size="icon" className={iconBtn} onClick={onEdit} title="编辑">
        <Pencil className="h-4 w-4" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className={`${iconBtn} hover:text-red-500 dark:hover:text-red-400`}
        onClick={() => setPendingDelete(true)}
        title="删除"
      >
        <Trash2 className="h-4 w-4" />
      </Button>
      <ConfirmDialog
        open={pendingDelete}
        title="删除端点"
        description={`确定要删除 "${endpoint.name}"？此操作不可撤销。`}
        confirmLabel="删除"
        confirmVariant="destructive"
        onCancel={() => setPendingDelete(false)}
        onConfirm={remove}
      />
    </>
  );
}
```

- [ ] **Step 5: 创建 `src/components/ConfirmDialog.tsx`**

```tsx
import * as React from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

interface Props {
  open: boolean;
  title: string;
  description?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  confirmVariant?: React.ComponentProps<typeof Button>["variant"];
  onCancel: () => void;
  onConfirm: () => void;
}

export function ConfirmDialog({
  open,
  title,
  description,
  confirmLabel = "确认",
  cancelLabel = "取消",
  confirmVariant = "default",
  onCancel,
  onConfirm,
}: Props) {
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onCancel()}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          {description && <DialogDescription>{description}</DialogDescription>}
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={onCancel}>
            {cancelLabel}
          </Button>
          <Button variant={confirmVariant} onClick={onConfirm}>
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
```

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "feat(frontend): add endpoint list, card, actions and confirm dialog"
```

---

### Task 22：端点表单（添加/编辑共用）

**Files:**
- Create: `src/components/endpoints/forms/RulesField.tsx`
- Create: `src/components/endpoints/forms/EndpointForm.tsx`
- Create: `src/components/endpoints/AddEndpointDialog.tsx`
- Create: `src/components/endpoints/EditEndpointDialog.tsx`

- [ ] **Step 1: 创建 `src/components/endpoints/forms/RulesField.tsx`**

```tsx
import { Plus, Trash2 } from "lucide-react";
import {
  Controller,
  useFieldArray,
  useFormContext,
} from "react-hook-form";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { EndpointForm } from "@/lib/schemas";

interface Props {
  name: "headerRules" | "queryRules";
  title: string;
  keyPlaceholder: string;
}

export function RulesField({ name, title, keyPlaceholder }: Props) {
  const { control, register } = useFormContext<EndpointForm>();
  const { fields, append, remove } = useFieldArray({ control, name });
  return (
    <section className="rounded-xl border p-4 space-y-3">
      <div className="flex items-center justify-between">
        <Label className="text-base font-semibold">{title}</Label>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => append({ action: "set", key: "", value: "" })}
        >
          <Plus className="h-4 w-4" />
          添加规则
        </Button>
      </div>
      {fields.length === 0 && (
        <p className="text-sm text-muted-foreground">暂无规则</p>
      )}
      <div className="space-y-2">
        {fields.map((field, idx) => (
          <div key={field.id} className="grid grid-cols-[7rem_1fr_1fr_2.5rem] gap-2">
            <Controller
              control={control}
              name={`${name}.${idx}.action` as const}
              render={({ field: f }) => (
                <Select value={f.value} onValueChange={f.onChange}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="set">覆盖 / 新增</SelectItem>
                    <SelectItem value="add">追加</SelectItem>
                    <SelectItem value="remove">删除</SelectItem>
                  </SelectContent>
                </Select>
              )}
            />
            <Input
              placeholder={keyPlaceholder}
              {...register(`${name}.${idx}.key` as const)}
            />
            <Controller
              control={control}
              name={`${name}.${idx}.action` as const}
              render={({ field: a }) => (
                <Input
                  placeholder="Value"
                  disabled={a.value === "remove"}
                  {...register(`${name}.${idx}.value` as const)}
                />
              )}
            />
            <Button
              type="button"
              variant="ghost"
              size="icon"
              onClick={() => remove(idx)}
              className="hover:text-red-500"
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        ))}
      </div>
    </section>
  );
}
```

- [ ] **Step 2: 创建 `src/components/endpoints/forms/EndpointForm.tsx`**

```tsx
import * as React from "react";
import { Controller, FormProvider, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { endpointFormSchema, type EndpointForm as TForm } from "@/lib/schemas";
import type { Endpoint } from "@/types";
import { RulesField } from "./RulesField";

interface Props {
  initial?: Endpoint;
  formId: string;
  onSubmit: (values: TForm) => Promise<void>;
}

function makeDefault(initial?: Endpoint): TForm {
  return {
    id: initial?.id ?? "",
    name: initial?.name ?? "",
    description: initial?.description ?? "",
    enabled: initial?.enabled ?? true,
    path: initial?.path ?? "/",
    upstreamUrl: initial?.upstreamUrl ?? "",
    stripPrefix: initial?.stripPrefix ?? true,
    headerRules: initial?.headerRules ?? [],
    queryRules: initial?.queryRules ?? [],
    bodyMerge: initial?.bodyMerge ?? "",
  };
}

export function EndpointForm({ initial, formId, onSubmit }: Props) {
  const methods = useForm<TForm>({
    resolver: zodResolver(endpointFormSchema),
    defaultValues: makeDefault(initial),
    mode: "onSubmit",
  });
  const { register, control, handleSubmit, formState: { errors } } = methods;

  return (
    <FormProvider {...methods}>
      <form
        id={formId}
        onSubmit={handleSubmit(onSubmit)}
        className="space-y-6"
      >
        <section className="rounded-xl border p-4 space-y-4">
          <Label className="text-base font-semibold">基本信息</Label>
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            <div className="space-y-1.5">
              <Label htmlFor="name">名称</Label>
              <Input id="name" placeholder="例如 Moonshot 转发" {...register("name")} />
              {errors.name && (
                <p className="text-sm text-destructive">{errors.name.message}</p>
              )}
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="description">描述（可选）</Label>
              <Input id="description" placeholder="可选说明" {...register("description")} />
            </div>
          </div>
          <div className="flex items-center justify-between rounded-md border bg-muted/30 p-3">
            <div>
              <Label className="text-sm font-medium">启用端点</Label>
              <p className="text-xs text-muted-foreground">关闭后此端点收到的请求会返回 404</p>
            </div>
            <Controller
              control={control}
              name="enabled"
              render={({ field }) => (
                <Switch checked={field.value} onCheckedChange={field.onChange} />
              )}
            />
          </div>
        </section>

        <section className="rounded-xl border p-4 space-y-4">
          <Label className="text-base font-semibold">路由</Label>
          <div className="space-y-1.5">
            <Label htmlFor="path">端点路径</Label>
            <Input
              id="path"
              placeholder="/cc"
              className="font-mono"
              {...register("path")}
            />
            {errors.path && (
              <p className="text-sm text-destructive">{errors.path.message}</p>
            )}
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="upstreamUrl">上游地址</Label>
            <Input
              id="upstreamUrl"
              placeholder="https://api.moonshot.com/v1"
              className="font-mono"
              {...register("upstreamUrl")}
            />
            {errors.upstreamUrl && (
              <p className="text-sm text-destructive">{errors.upstreamUrl.message}</p>
            )}
          </div>
          <div className="flex items-center justify-between rounded-md border bg-muted/30 p-3">
            <div>
              <Label className="text-sm font-medium">剥离路径前缀</Label>
              <p className="text-xs text-muted-foreground">
                开启：/cc/x → 上游 /x；关闭：/cc/x → 上游 /cc/x
              </p>
            </div>
            <Controller
              control={control}
              name="stripPrefix"
              render={({ field }) => (
                <Switch checked={field.value} onCheckedChange={field.onChange} />
              )}
            />
          </div>
        </section>

        <RulesField
          name="headerRules"
          title="请求头规则"
          keyPlaceholder="Header 名（如 Authorization）"
        />
        <RulesField
          name="queryRules"
          title="查询参数规则"
          keyPlaceholder="参数名（如 model）"
        />

        <section className="rounded-xl border p-4 space-y-3">
          <Label htmlFor="bodyMerge" className="text-base font-semibold">
            请求体 JSON 合并（可选）
          </Label>
          <p className="text-sm text-muted-foreground">
            仅当请求体为 JSON 时生效。这里填的对象会深合并到原 body，键冲突时此处覆盖。
          </p>
          <Textarea
            id="bodyMerge"
            placeholder='{"model":"kimi-k2"}'
            className="font-mono"
            {...register("bodyMerge")}
          />
          {errors.bodyMerge && (
            <p className="text-sm text-destructive">{errors.bodyMerge.message}</p>
          )}
        </section>
      </form>
    </FormProvider>
  );
}
```

- [ ] **Step 3: 创建 `src/components/endpoints/AddEndpointDialog.tsx`**

```tsx
import { Plus } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import { api } from "@/lib/api";
import { EndpointForm } from "./forms/EndpointForm";
import type { Endpoint } from "@/types";

interface Props {
  open: boolean;
  onClose: () => void;
  onSaved: () => void;
}

const FORM_ID = "endpoint-add-form";

export function AddEndpointDialog({ open, onClose, onSaved }: Props) {
  return (
    <FullScreenPanel
      open={open}
      onClose={onClose}
      title="添加端点"
      footer={
        <>
          <Button variant="outline" onClick={onClose}>
            取消
          </Button>
          <Button type="submit" form={FORM_ID}>
            <Plus className="h-4 w-4" />
            添加
          </Button>
        </>
      }
    >
      {open && (
        <EndpointForm
          formId={FORM_ID}
          onSubmit={async (values) => {
            try {
              const ep: Endpoint = {
                ...values,
                createdAt: 0,
                updatedAt: 0,
              };
              await api.saveEndpoint(ep);
              toast.success("已保存");
              onSaved();
            } catch (e: any) {
              toast.error(typeof e === "string" ? e : e?.message ?? "保存失败");
            }
          }}
        />
      )}
    </FullScreenPanel>
  );
}
```

- [ ] **Step 4: 创建 `src/components/endpoints/EditEndpointDialog.tsx`**

```tsx
import { Save } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import { api } from "@/lib/api";
import { EndpointForm } from "./forms/EndpointForm";
import type { Endpoint } from "@/types";

interface Props {
  endpoint: Endpoint;
  onClose: () => void;
  onSaved: () => void;
}

const FORM_ID = "endpoint-edit-form";

export function EditEndpointDialog({ endpoint, onClose, onSaved }: Props) {
  return (
    <FullScreenPanel
      open={true}
      onClose={onClose}
      title={`编辑：${endpoint.name}`}
      footer={
        <>
          <Button variant="outline" onClick={onClose}>
            取消
          </Button>
          <Button type="submit" form={FORM_ID}>
            <Save className="h-4 w-4" />
            保存
          </Button>
        </>
      }
    >
      <EndpointForm
        initial={endpoint}
        formId={FORM_ID}
        onSubmit={async (values) => {
          try {
            const ep: Endpoint = {
              ...values,
              createdAt: endpoint.createdAt,
              updatedAt: endpoint.updatedAt,
            };
            await api.saveEndpoint(ep);
            toast.success("已保存");
            onSaved();
          } catch (e: any) {
            toast.error(typeof e === "string" ? e : e?.message ?? "保存失败");
          }
        }}
      />
    </FullScreenPanel>
  );
}
```

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(frontend): add endpoint form and add/edit dialogs"
```

---

### Task 23：设置面板

**Files:**
- Create: `src/components/settings/SettingsPanel.tsx`

- [ ] **Step 1: 创建 `src/components/settings/SettingsPanel.tsx`**

```tsx
import * as React from "react";
import { Save } from "lucide-react";
import { Controller, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { api } from "@/lib/api";
import { globalConfigSchema } from "@/lib/schemas";
import type { GlobalConfig } from "@/types";

interface Props {
  open: boolean;
  global: GlobalConfig;
  onClose: () => void;
  onSaved: (g: GlobalConfig) => void;
}

const FORM_ID = "settings-form";

export function SettingsPanel({ open, global, onClose, onSaved }: Props) {
  const { register, control, handleSubmit, reset, formState: { errors } } =
    useForm<GlobalConfig>({
      resolver: zodResolver(globalConfigSchema),
      defaultValues: global,
    });

  React.useEffect(() => {
    if (open) reset(global);
  }, [open, global, reset]);

  const submit = handleSubmit(async (values) => {
    try {
      const status = await api.applyGlobalConfig(values);
      toast.success(
        status.running
          ? `服务运行中：${status.listenAddress}:${status.listenPort}`
          : "已保存",
      );
      onSaved(values);
      onClose();
    } catch (e: any) {
      toast.error(typeof e === "string" ? e : e?.message ?? "保存失败");
    }
  });

  return (
    <FullScreenPanel
      open={open}
      onClose={onClose}
      title="设置"
      footer={
        <>
          <Button variant="outline" onClick={onClose}>
            取消
          </Button>
          <Button type="submit" form={FORM_ID}>
            <Save className="h-4 w-4" />
            应用
          </Button>
        </>
      }
    >
      <form id={FORM_ID} onSubmit={submit} className="space-y-6">
        <section className="rounded-xl border p-4 space-y-4">
          <Label className="text-base font-semibold">代理监听</Label>
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            <div className="space-y-1.5">
              <Label htmlFor="listenAddress">监听地址</Label>
              <Input
                id="listenAddress"
                className="font-mono"
                placeholder="0.0.0.0"
                {...register("listenAddress")}
              />
              {errors.listenAddress && (
                <p className="text-sm text-destructive">
                  {errors.listenAddress.message}
                </p>
              )}
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="listenPort">监听端口</Label>
              <Input
                id="listenPort"
                type="number"
                min={1}
                max={65535}
                {...register("listenPort", { valueAsNumber: true })}
              />
              {errors.listenPort && (
                <p className="text-sm text-destructive">
                  {errors.listenPort.message}
                </p>
              )}
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="requestTimeoutSecs">请求超时（秒）</Label>
              <Input
                id="requestTimeoutSecs"
                type="number"
                min={1}
                max={600}
                {...register("requestTimeoutSecs", { valueAsNumber: true })}
              />
              {errors.requestTimeoutSecs && (
                <p className="text-sm text-destructive">
                  {errors.requestTimeoutSecs.message}
                </p>
              )}
            </div>
          </div>
        </section>

        <section className="rounded-xl border p-4 space-y-4">
          <Label className="text-base font-semibold">行为</Label>
          <div className="flex items-center justify-between rounded-md border bg-muted/30 p-3">
            <div>
              <Label className="text-sm font-medium">关闭窗口最小化到托盘</Label>
              <p className="text-xs text-muted-foreground">
                关闭后只隐藏窗口，代理服务继续在后台运行
              </p>
            </div>
            <Controller
              control={control}
              name="closeToTray"
              render={({ field }) => (
                <Switch checked={field.value} onCheckedChange={field.onChange} />
              )}
            />
          </div>
          <div className="flex items-center justify-between rounded-md border bg-muted/30 p-3">
            <div>
              <Label className="text-sm font-medium">启动时自动启动代理</Label>
              <p className="text-xs text-muted-foreground">
                打开应用后立即启动代理服务
              </p>
            </div>
            <Controller
              control={control}
              name="autoStartServer"
              render={({ field }) => (
                <Switch checked={field.value} onCheckedChange={field.onChange} />
              )}
            />
          </div>
        </section>

        <section className="rounded-xl border p-4 space-y-4">
          <Label className="text-base font-semibold">外观</Label>
          <div className="space-y-1.5">
            <Label>主题</Label>
            <Controller
              control={control}
              name="theme"
              render={({ field }) => (
                <Select value={field.value} onValueChange={field.onChange}>
                  <SelectTrigger className="max-w-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="light">亮色</SelectItem>
                    <SelectItem value="dark">深色</SelectItem>
                    <SelectItem value="system">跟随系统</SelectItem>
                  </SelectContent>
                </Select>
              )}
            />
          </div>
        </section>
      </form>
    </FullScreenPanel>
  );
}
```

- [ ] **Step 2: typecheck**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy
pnpm typecheck
```

预期：通过（如有错误根据提示修正）。

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "feat(frontend): add settings panel with global config form"
```

---

## Phase 5：联调与手测

### Task 24：联调与手测

**Files:**（无新文件，仅运行验证）

- [ ] **Step 1: typecheck + cargo test 全量验证**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy
pnpm typecheck
cd src-tauri && cargo test
```

预期：全部通过。

- [ ] **Step 2: 启动开发模式**

```bash
cd /Users/fanjunjie/Documents/repositories/personal/apiproxy
pnpm tauri dev
```

预期窗口出现，主页显示空状态。

- [ ] **Step 3: 手动测试清单**（按顺序勾选）：

```
1. 点击右上角 "+" → 添加端点对话框打开
2. 填写：名称 "Moonshot"，路径 "/cc"，上游 "https://api.moonshot.com/v1"
   → 添加 → 列表出现卡片
3. 列表卡片 hover 显示操作按钮
4. 点击卡片"复制"按钮 → toast "已复制"
5. 终端执行：curl -i http://127.0.0.1:8118/cc/  → 应返回 Moonshot 上游响应或上游错误（连通即可）
6. 设置 → 把端口改成 8119 → 应用 → toast 提示端口变更，监听徽章更新
7. 关闭主窗口 → 应隐藏到托盘（macOS Dock 可能消失，Win/Linux 见托盘图标）
8. 托盘菜单 → "停止代理" → 状态徽章变 "已停止"
9. 托盘 → "启动代理" → 状态徽章变 "运行中"
10. 主题切换 → light/dark 视觉正常
11. 端点详情：编辑加 header 规则 Authorization: Bearer xxx，保存，再 curl 一次 → 上游应见 header
12. 编辑：path 改成 "/cc"（与已有冲突端点对应） → 应出错 toast
```

- [ ] **Step 4: 修复任何手测发现的问题，每个问题一个提交**

- [ ] **Step 5: 提交手测完成标记**

```bash
git add -A
git commit -m "chore: complete manual verification round"
```

---

## 完成

至此 MVP 实现完成，覆盖：
- 单端口路径前缀代理 + 路径剥离
- 请求头/查询参数 set/add/remove
- JSON 请求体深合并
- SSE 流式响应透传
- 端点 CRUD + 启停 + 热应用
- 全局服务开关 + 系统托盘
- 端口/超时/主题/托盘行为设置
- light/dark/system 主题
- 后端单元 + 集成测试
- macOS 风格 cc-switch 一致 UI
