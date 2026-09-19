# 多上游支持设计规格

日期：2026-09-16
状态：已确认（加权轮询 / 连接失败+5xx 重试 / P3 三项预留）

## 1. 背景与目标

当前一个端点仅支持一个上游（`Endpoint.upstream_url`），上游故障即 502 无退路。
本功能为每个端点提供多个上游，支持：

- 负载均衡（加权轮询）
- 失败转移（连接失败 + 可重试状态码）
- 上游级启用/禁用开关
- 复制端点快捷创建（现有功能，天然兼容）

## 2. 数据模型

```rust
pub struct Upstream {
    pub id: String,          // UUID，端点内唯一
    pub name: String,        // 展示/日志用，可空
    pub url: String,         // http/https
    pub enabled: bool,       // 上游级开关；false 不参与均衡与转移
    pub weight: u32,         // 1..=100，默认 1
    #[serde(default)]
    pub header_rules: Vec<Rule>, // 本上游独有请求头规则，端点级规则之后应用，同 key 覆盖
    #[serde(default)]
    pub sort_index: i32,     // P3 拖拽排序预留：同权重时的优先顺序
}

pub struct Endpoint {
    // ...现有字段不变，仅一处变更：
    pub upstreams: Vec<Upstream>,   // 替换原 upstream_url: String
}
```

语义约定：

- `enabled = false` 的上游既不被均衡选中，也不参与失败转移。
- 无优先级概念：所有启用的上游按权重比例分担流量（如 3:1 即 75% / 25%），权重 0 不定义特殊语义。
- header 规则分两级：端点级（公共默认，对所有上游生效）→ 上游级（仅本上游，同 key 覆盖端点级）。
  典型场景：同一端点下多个上游各自需要不同的 Authorization。query 规则保持在端点级。
  规则在保存时校验（合法 header 名/值），运行时失败属内部错误。
- 失败转移的上游顺序 = 加权轮询游标推进顺序，与列表顺序无关（列表顺序只影响同权重展开项的相对位置，配合 sort_index）。

## 3. 选择算法

加权轮询（展开式 + 游标）：

```
expanded = enabled 上游按权重展开，如 [A,A,A,B]（A.weight=3, B.weight=1）
idx = cursor[endpoint_id]++ % expanded.len()
选中 expanded[idx]
```

- 状态存放：`ProxyState.cursors: Arc<Mutex<HashMap<String, usize>>>`（key = endpoint_id）。
- 配置热更新（replace_routes）不重置游标；重启清空。
- enabled 上游为 0 → 直接 502（`NO_UPSTREAM`）。

## 4. 失败转移

### 4.1 触发条件

| 情形 | 是否转移 |
|---|---|
| 连接失败（`is_connect`） | 是 |
| 超时（`is_timeout`） | 是 |
| 上游返回 429 / 500 / 502 / 503 / 504 | 是（可重试状态码白名单） |
| 其他 4xx（400/401/422 等） | 否，原样透传（客户端错误换上游无用） |
| 流式响应已开始（收到首个字节） | 否，任何情况都不再重试 |

### 4.2 语义护栏（代码注释必须写明）

1. 重试上限 = enabled 上游数量，全部失败才返回 502。
2. 仅在收到上游首个字节之前允许转移；SSE 中途断线不重试（避免重复推内容）。
3. 5xx 重试的固有风险：上游可能已处理请求（LLM 重复计费 / 非幂等副作用）。
   这是用户显式选择的语义，由调用方承担。
4. 每次转移前丢弃（drain）失败响应的 body，保持连接复用。

### 4.3 重试流程（handler 伪代码）

```
attempts = []
for i in 0..enabled_upstreams.len():
    upstream = pick(endpoint, cursors, offset=i)
    match send(req, upstream):
        Err(e) if e.is_connect() || e.is_timeout():
            attempts.push("upstream_url → 连接失败/超时: e")
            continue
        Err(e):
            attempts.push(...); break          // 其他错误不重试
        Ok(resp):
            if resp.status in RETRYABLE && i < N-1:
                drain(resp); attempts.push("url → status"); continue
            resp.status not in RETRYABLE 或最后一次尝试:
                流式回写并返回
全部失败 → 502，error 汇总 attempts
```

## 5. 配置迁移（serde 兼容，用户无感知）

- `Endpoint` 手写 `Deserialize`：影子结构体 `EndpointShadow` 同时接受旧键 `upstreamUrl`
  与新键 `upstreams`；旧数据读入时转为单元素 `upstreams`（enabled=true, weight=1, name=""）。
- 保存即升级到新格式；旧键只读不写。
- `Serialize` 保持 derive（camelCase）。

## 6. 校验规则

- 端点 ≥ 1 个上游。
- enabled 的上游 ≥ 1（防止路由永远 502）。
- 每个上游 URL 合法 http/https（复用现有 `InvalidUrl` 语义，按上游名标注）。
- 权重 1..=100。
- 上游 id 端点内唯一（UUID 生成，天然满足）。

## 7. 日志

`RequestLog` 变更：

- `upstream_url` 语义不变 = 实际命中的上游 URL。
- 新增 `failover_log: Vec<String>`（camelCase `failoverLog`）：每次转移一条，如
  `"A (https://a.com) → 503，切换到 B"`。无转移时为空数组。
- 前端日志面板在详情区展示转移轨迹。

## 8. 前端设计

| 位置 | 变化 |
|---|---|
| types.ts | 新增 `Upstream` 接口；`Endpoint.upstreamUrl` → `upstreams: Upstream[]`；`RequestLog` 增 `failoverLog: string[]` |
| schemas.ts | 上游数组 zod 校验（URL、权重 1-100、enabled 上游 ≥1） |
| EndpointForm | 「路由」段只留端点路径；新增「上游」段：可增删列表（名称(可空)/URL/权重/启用开关/删除行），底部「添加上游」 |
| EndpointCard | 显示首个上游 URL + `共 N 个` 徽标（或 N=1 时不显示） |
| EndpointActions.duplicate | 深拷贝 upstreams 并重新生成上游 id（现有逻辑扩展一行 map） |
| RequestLogPanel | 详情区展示 failoverLog（转移轨迹，无则不显示） |

表单中「启用端点」开关移除（与卡片 Pause/Play 重复，同一字段）；新建端点默认启用。

## 9. 分阶段实施

### P1 后端核心

- Upstream 结构体 + Endpoint 字段替换 + serde 迁移（影子结构体）
- config 校验规则
- `proxy/upstream.rs`：展开式加权轮询选择
- handler 失败转移循环（含 5xx 白名单、drain、流式保护）
- `ProxyState.cursors`
- RequestLog.failover_log
- 测试：加权轮询分布、单上游兼容、主挂切备、全挂 502、5xx 重试、禁用上游不被选中、旧配置迁移反序列化

### P2 前端

- types/schemas/api 适配
- EndpointForm 上游列表编辑
- EndpointCard 展示 + duplicate 适配
- RequestLogPanel failoverLog 展示
- 移除表单「启用端点」开关
- 手测 + typecheck + vite build

### P3 后续可选（本次只预留接口，不实现）

| 项 | 预留内容 |
|---|---|
| 拖拽排序 | `Upstream.sort_index` 字段已建；选择算法同权重时按 sort_index 稳定排序 |
| 每上游成功率统计 | `ProxyState` 预留 `upstream_stats: HashMap<(endpoint_id, upstream_id), {ok, err}>` 的写入点（handler 转移循环出口处埋点） |
| 熔断冷却 | 预留配置概念（fail_threshold / cooldown_secs），在 pick() 前加过滤；数据结构待定 |

## 10. 非目标（明确不做）

- per-upstream 的 header/query 规则
- 主动健康检查（探测端点）
- DNS 级别的负载均衡
- 跨端点的上游共享/引用
