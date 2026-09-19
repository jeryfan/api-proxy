use std::sync::Arc;
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use axum::body::{Body, Bytes};
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use futures_util::StreamExt;
use http::header::HOST;
use serde_json::json;
use std::net::SocketAddr;
use tauri::{AppHandle, Emitter, Wry};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::error;

use crate::config::Endpoint;
use crate::events;
use crate::proxy::log_store::{
    encode_body, is_binary_content_type, HeaderEntry, LogStore, RequestLog,
};
use crate::proxy::transform;
use crate::proxy::upstream::{self, UpstreamCursors};

/// 可重试的上游状态码：限流与网关类错误换上游重发；
/// 其他 4xx 属客户端错误，换上游无用，原样透传。
const RETRYABLE_STATUS: &[u16] = &[429, 500, 502, 503, 504];

#[derive(Clone)]
pub struct ProxyState {
    pub routes: Arc<ArcSwap<Vec<Endpoint>>>,
    pub timeout: Arc<std::sync::atomic::AtomicU64>,
    pub log_store: Arc<LogStore>,
    pub app_handle: Option<AppHandle<Wry>>,
    pub cursors: UpstreamCursors,
    pub health_tracker: upstream::UpstreamHealthTracker,
}

fn header_entries(headers: &HeaderMap) -> Vec<HeaderEntry> {
    headers
        .iter()
        .map(|(k, v)| HeaderEntry {
            key: k.as_str().to_string(),
            value: v.to_str().unwrap_or("").to_string(),
        })
        .collect()
}

fn content_type_of(headers: &HeaderMap) -> Option<String> {
    headers
        .get(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn make_log_skeleton(
    endpoint: &Endpoint,
    req_path: &str,
    req_query: &Option<String>,
    client_addr: Option<String>,
    req_headers: &HeaderMap,
    req_method: &str,
) -> RequestLog {
    RequestLog {
        id: uuid::Uuid::new_v4().to_string(),
        endpoint_id: endpoint.id.clone(),
        started_at: Utc::now().timestamp_millis(),
        client_addr,
        req_method: req_method.to_string(),
        req_path: req_path.to_string(),
        req_query: req_query.clone(),
        req_headers: header_entries(req_headers),
        req_body_b64: String::new(),
        req_body_len: 0,
        req_body_binary: false,
        upstream_url: String::new(),
        upstream_headers: vec![],
        status_code: None,
        resp_headers: vec![],
        upstream_resp_headers: vec![],
        resp_body_b64: String::new(),
        resp_body_len: 0,
        resp_body_binary: false,
        resp_content_type: None,
        failover_log: vec![],
        duration_ms: 0,
        error: None,
    }
}

fn finalize_log(state: &ProxyState, mut log: RequestLog, started: Instant) {
    log.duration_ms = started.elapsed().as_millis() as u64;
    state.log_store.push(log.clone());
    if let Some(handle) = &state.app_handle {
        let _ = handle.emit(events::LOG_RECORDED, &log);
    }
}

fn upstream_label(name: &str, url: &str) -> String {
    if name.is_empty() {
        url.to_string()
    } else {
        format!("{name} ({url})")
    }
}

pub async fn proxy_handler(
    State(state): State<ProxyState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
) -> Response {
    let started = Instant::now();
    let req_path = req.uri().path().to_string();
    let req_query = req.uri().query().map(|s| s.to_string());
    let req_method = req.method().as_str().to_string();
    let client_addr = Some(addr.to_string());
    let routes = state.routes.load_full();

    let endpoint = match transform::match_endpoint(&routes, &req_path) {
        Some(e) => e.clone(),
        None => {
            return json_error(
                StatusCode::NOT_FOUND,
                "no endpoint matched",
                "NO_ENDPOINT_MATCHED",
                Some(&req_path),
            );
        }
    };

    let mut log = make_log_skeleton(
        &endpoint,
        &req_path,
        &req_query,
        client_addr.clone(),
        req.headers(),
        &req_method,
    );

    // 本次请求的上游计划：起始上游由加权轮询决定，排除熔断与禁用，后续为转移顺序（无重复）。
    let plan = match upstream::plan(&endpoint, &state.cursors, &state.health_tracker) {
        Ok(p) => p,
        Err(upstream::PlanError::NoEnabledUpstream) => {
            log.error = Some("no enabled upstream".into());
            finalize_log(&state, log, started);
            return json_error(
                StatusCode::BAD_GATEWAY,
                "no enabled upstream",
                "NO_UPSTREAM",
                Some(&endpoint.path),
            );
        }
        Err(upstream::PlanError::AllUpstreamsTripped) => {
            let msg = "all enabled upstreams are tripped (熔断失效)，可前往控制台手动重新开启";
            log.error = Some(msg.into());
            finalize_log(&state, log, started);
            return json_error(
                StatusCode::BAD_GATEWAY,
                msg,
                "ALL_UPSTREAMS_TRIPPED",
                Some(&endpoint.path),
            );
        }
    };
    let max_attempts = plan.len();

    // 请求头预处理（与上游无关的部分只做一次）：剥离 hop-by-hop、应用 header 规则。
    let (parts, body) = req.into_parts();
    let mut headers = parts.headers.clone();
    transform::strip_hop_by_hop(&mut headers);
    if let Err(e) = transform::apply_header_rules(&mut headers, &endpoint.header_rules) {
        log.error = Some(e.to_string());
        finalize_log(&state, log, started);
        return json_error(
            StatusCode::BAD_REQUEST,
            &e.to_string(),
            "HEADER_RULE_FAILED",
            Some(&endpoint.path),
        );
    }

    let method = reqwest::Method::from_bytes(parts.method.as_str().as_bytes())
        .unwrap_or(reqwest::Method::GET);
    let timeout_secs = state.timeout.load(std::sync::atomic::Ordering::Relaxed);

    let raw_request_body: Vec<u8>;
    let body_data: BodyData = match prepare_body(body, &mut headers).await {
        Ok((data, raw)) => {
            raw_request_body = raw;
            data
        }
        Err(msg) => {
            log.error = Some(msg.clone());
            finalize_log(&state, log, started);
            return json_error(
                StatusCode::BAD_REQUEST,
                &msg,
                "BODY_PREPARE_FAILED",
                Some(&endpoint.path),
            );
        }
    };

    {
        let (b64, len) = encode_body(&raw_request_body);
        log.req_body_b64 = b64;
        log.req_body_len = len;
        log.req_body_binary = log
            .req_headers
            .iter()
            .find(|h| h.key.eq_ignore_ascii_case("content-type"))
            .map(|h| is_binary_content_type(&h.value))
            .unwrap_or(false);
    }

    let client = crate::proxy::http_client::get();

    // 失败转移主循环。
    // 语义护栏：
    // 1. 仅连接级失败（连不上/超时）与可重试状态码（429/5xx 白名单）触发转移；
    // 2. 仅在收到上游首个字节之前允许转移——流式响应一旦开始绝不重试（避免重复推内容）；
    // 3. 重试上限 = enabled 上游数，全部失败才返回 502；
    // 4. 5xx 重试的固有风险：上游可能已处理请求（LLM 重复计费/非幂等副作用），由调用方承担；
    // 5. 每次转移前 drain 失败响应体，保持连接复用。
    enum UpstreamReceived {
        Stream(reqwest::Response),
        Buffered {
            status: StatusCode,
            headers: HeaderMap,
            body: Bytes,
        },
    }

    let mut received_response: Option<UpstreamReceived> = None;
    let mut last_error: Option<(StatusCode, String, &'static str)> = None;

    for (offset, up) in plan.iter().enumerate() {
        let label = upstream_label(&up.name, &up.url);

        let mut attempt_url = match transform::build_upstream_url(
            &up.url,
            &endpoint,
            &req_path,
            req_query.as_deref(),
        ) {
            Ok(u) => u,
            Err(e) => {
                last_error = Some((
                    StatusCode::BAD_GATEWAY,
                    e.to_string(),
                    "BUILD_URL_FAILED",
                ));
                break;
            }
        };
        transform::apply_query_rules(&mut attempt_url, &endpoint.query_rules);

        // Host 头按本次选中的上游设置；随后应用本上游独有的 header 规则
        // （端点级规则在前、上游级在后，同 key 以上游级为准）。
        let mut attempt_headers = headers.clone();
        if let Some(host) = attempt_url.host_str() {
            let host_value = match attempt_url.port() {
                Some(p) => format!("{host}:{p}"),
                None => host.to_string(),
            };
            if let Ok(v) = HeaderValue::from_str(&host_value) {
                attempt_headers.insert(HOST, v);
            }
        }
        if let Err(e) = transform::apply_header_rules(&mut attempt_headers, &up.header_rules) {
            // 规则已在保存时校验，运行时失败属内部错误，不重试。
            last_error = Some((
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
                "UPSTREAM_HEADER_RULE_FAILED",
            ));
            break;
        }

        let mut builder = client
            .request(method.clone(), attempt_url.clone())
            .timeout(Duration::from_secs(timeout_secs))
            .headers(reqwest_headers(&attempt_headers));
        builder = match &body_data {
            BodyData::Empty => builder,
            BodyData::Bytes(b) => builder.body(b.clone()),
        };

        match builder.send().await {
            Err(e) => {
                let retryable = e.is_connect() || e.is_timeout();
                let code = if e.is_timeout() {
                    "UPSTREAM_TIMEOUT"
                } else if e.is_connect() {
                    "UPSTREAM_CONNECT"
                } else {
                    "UPSTREAM_ERROR"
                };

                // 若开启熔断健康检测且计入网络连接/超时错误
                if up.health.enabled && up.health.count_connect_error && retryable {
                    let (hs, newly_tripped) = state.health_tracker.record_failure(
                        &endpoint.id,
                        &up.id,
                        up.health.failure_threshold,
                        format!("{code}: {e}"),
                    );
                    if newly_tripped {
                        if let Some(h) = &state.app_handle {
                            let _ = h.emit(events::UPSTREAM_HEALTH_CHANGED, &hs);
                        }
                    }
                }

                if retryable && offset + 1 < max_attempts {
                    log.failover_log.push(format!("{label} → 连接失败/超时：{e}"));
                    continue;
                }
                error!("upstream request failed: {e}");
                last_error = Some((StatusCode::BAD_GATEWAY, format!("{code}: {e}"), code));
                break;
            }
            Ok(resp) => {
                let status_code = resp.status().as_u16();
                let is_status_fail = if up.health.enabled {
                    up.health.status_codes.contains(&status_code)
                } else {
                    RETRYABLE_STATUS.contains(&status_code)
                };

                let has_body_match = up.health.enabled
                    && up
                        .health
                        .body_match
                        .as_ref()
                        .map_or(false, |bm| !bm.pattern.is_empty());

                let resp_content_type = content_type_of(resp.headers());
                let is_sse = resp_content_type
                    .as_deref()
                    .map(|ct| ct.starts_with("text/event-stream"))
                    .unwrap_or(false);

                // 如果配置了响应体匹配（正则或字符串）且非流式响应，则先读取响应体进行检测
                if has_body_match && !is_sse {
                    let bm = up.health.body_match.as_ref().unwrap();
                    let resp_status = StatusCode::from_u16(status_code).unwrap_or(StatusCode::BAD_GATEWAY);
                    let mut resp_headers = HeaderMap::new();
                    for (k, v) in resp.headers() {
                        if let (Ok(name), Ok(value)) = (
                            http::HeaderName::from_bytes(k.as_str().as_bytes()),
                            HeaderValue::from_bytes(v.as_bytes()),
                        ) {
                            resp_headers.append(name, value);
                        }
                    }

                    let body_bytes = match resp.bytes().await {
                        Ok(b) => b,
                        Err(e) => {
                            last_error = Some((
                                StatusCode::BAD_GATEWAY,
                                format!("READ_BODY_FAILED: {e}"),
                                "READ_BODY_FAILED",
                            ));
                            break;
                        }
                    };
                    let body_str = String::from_utf8_lossy(&body_bytes);
                    let body_matched = bm.is_match(&body_str);

                    if body_matched || is_status_fail {
                        let reason = if body_matched {
                            format!("响应体匹配失败规则：'{}'", bm.pattern)
                        } else {
                            format!("HTTP {status_code}")
                        };

                        if up.health.enabled {
                            let (hs, newly_tripped) = state.health_tracker.record_failure(
                                &endpoint.id,
                                &up.id,
                                up.health.failure_threshold,
                                reason.clone(),
                            );
                            if newly_tripped {
                                if let Some(h) = &state.app_handle {
                                    let _ = h.emit(events::UPSTREAM_HEALTH_CHANGED, &hs);
                                }
                            }
                        }

                        if offset + 1 < max_attempts {
                            log.failover_log.push(format!("{label} → {reason}"));
                            continue;
                        }

                        log.upstream_url = attempt_url.to_string();
                        log.upstream_headers = header_entries(&attempt_headers);
                        received_response = Some(UpstreamReceived::Buffered {
                            status: resp_status,
                            headers: resp_headers,
                            body: body_bytes,
                        });
                        break;
                    } else {
                        if up.health.enabled {
                            if let Some(s) = state.health_tracker.record_success(&endpoint.id, &up.id) {
                                if let Some(h) = &state.app_handle {
                                    let _ = h.emit(events::UPSTREAM_HEALTH_CHANGED, &s);
                                }
                            }
                        }
                        log.upstream_url = attempt_url.to_string();
                        log.upstream_headers = header_entries(&attempt_headers);
                        received_response = Some(UpstreamReceived::Buffered {
                            status: resp_status,
                            headers: resp_headers,
                            body: body_bytes,
                        });
                        break;
                    }
                } else {
                    if is_status_fail {
                        if up.health.enabled {
                            let (hs, newly_tripped) = state.health_tracker.record_failure(
                                &endpoint.id,
                                &up.id,
                                up.health.failure_threshold,
                                format!("HTTP {status_code}"),
                            );
                            if newly_tripped {
                                if let Some(h) = &state.app_handle {
                                    let _ = h.emit(events::UPSTREAM_HEALTH_CHANGED, &hs);
                                }
                            }
                        }

                        if offset + 1 < max_attempts {
                            let _ = resp.bytes().await;
                            log.failover_log.push(format!("{label} → HTTP {status_code}"));
                            continue;
                        }
                    } else if up.health.enabled {
                        if let Some(s) = state.health_tracker.record_success(&endpoint.id, &up.id) {
                            if let Some(h) = &state.app_handle {
                                let _ = h.emit(events::UPSTREAM_HEALTH_CHANGED, &s);
                            }
                        }
                    }

                    log.upstream_url = attempt_url.to_string();
                    log.upstream_headers = header_entries(&attempt_headers);
                    received_response = Some(UpstreamReceived::Stream(resp));
                    break;
                }
            }
        }
    }

    let received = match received_response {
        Some(r) => r,
        None => {
            let (status, msg, code) = last_error.unwrap_or((
                StatusCode::BAD_GATEWAY,
                "all upstreams failed".to_string(),
                "UPSTREAM_ERROR",
            ));
            if !log.failover_log.is_empty() {
                msg_push_attempts(&mut log, &msg);
            }
            log.error = Some(msg.clone());
            finalize_log(&state, log, started);
            return json_error(status, &msg, code, Some(&endpoint.path));
        }
    };

    match received {
        UpstreamReceived::Buffered {
            status,
            mut headers,
            body,
        } => {
            log.status_code = Some(status.as_u16());
            log.upstream_resp_headers = header_entries(&headers);
            transform::strip_hop_by_hop(&mut headers);
            log.resp_headers = header_entries(&headers);
            let content_type = content_type_of(&headers);
            log.resp_content_type = content_type.clone();
            let skip_resp_body = content_type
                .as_deref()
                .map(is_binary_content_type)
                .unwrap_or(false);
            log.resp_body_binary = skip_resp_body;

            if !skip_resp_body {
                let (b64, len) = encode_body(&body);
                log.resp_body_b64 = b64;
                log.resp_body_len = len;
            }
            finalize_log(&state, log, started);

            let mut response = Response::builder().status(status);
            if let Some(h) = response.headers_mut() {
                *h = headers;
            }
            response.body(Body::from(body)).unwrap()
        }
        UpstreamReceived::Stream(upstream_response) => {
            let status = StatusCode::from_u16(upstream_response.status().as_u16())
                .unwrap_or(StatusCode::BAD_GATEWAY);
            log.status_code = Some(status.as_u16());

            let mut resp_headers = HeaderMap::new();
            for (k, v) in upstream_response.headers() {
                if let (Ok(name), Ok(value)) = (
                    http::HeaderName::from_bytes(k.as_str().as_bytes()),
                    HeaderValue::from_bytes(v.as_bytes()),
                ) {
                    resp_headers.append(name, value);
                }
            }
            log.upstream_resp_headers = header_entries(&resp_headers);
            transform::strip_hop_by_hop(&mut resp_headers);
            log.resp_headers = header_entries(&resp_headers);
            let content_type = content_type_of(&resp_headers);
            log.resp_content_type = content_type.clone();
            let skip_resp_body = content_type
                .as_deref()
                .map(is_binary_content_type)
                .unwrap_or(false);
            log.resp_body_binary = skip_resp_body;

            let mut upstream_stream = upstream_response
                .bytes_stream()
                .map(|r| r.map_err(std::io::Error::other));

            let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(16);
            let log_state = state.clone();
            let log_started = started;
            let mut prelog = log;

            tokio::spawn(async move {
                let mut buf: Vec<u8> = Vec::new();
                while let Some(chunk_result) = upstream_stream.next().await {
                    match chunk_result {
                        Ok(bytes) => {
                            if !skip_resp_body {
                                buf.extend_from_slice(&bytes);
                            }
                            if tx.send(Ok(bytes)).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            prelog.error = Some(msg.clone());
                            let _ = tx.send(Err(std::io::Error::other(e))).await;
                            break;
                        }
                    }
                }
                if !skip_resp_body {
                    let (b64, len) = encode_body(&buf);
                    prelog.resp_body_b64 = b64;
                    prelog.resp_body_len = len;
                }
                finalize_log(&log_state, prelog, log_started);
            });

            let body = Body::from_stream(ReceiverStream::new(rx));
            let mut response = Response::builder().status(status);
            if let Some(h) = response.headers_mut() {
                *h = resp_headers;
            }
            response.body(body).unwrap()
        }
    }
}

fn msg_push_attempts(log: &mut RequestLog, msg: &str) {
    log.error = Some(format!(
        "{msg}；转移记录：{}",
        log.failover_log.join("；")
    ));
}

enum BodyData {
    Empty,
    Bytes(Vec<u8>),
}

async fn prepare_body(body: Body, headers: &mut HeaderMap) -> Result<(BodyData, Vec<u8>), String> {
    use http_body_util::BodyExt;
    let collected = body.collect().await.map_err(|e| e.to_string())?;
    let raw = collected.to_bytes().to_vec();

    if raw.is_empty() {
        return Ok((BodyData::Empty, raw));
    }
    // 转发时由 reqwest 重新计算 Content-Length，先移除旧的避免不一致。
    headers.remove(http::header::CONTENT_LENGTH);
    Ok((BodyData::Bytes(raw.clone()), raw))
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
) -> Response {
    let body = json!({
        "error": msg,
        "code": code,
        "endpoint": endpoint,
    });
    let mut resp = (status, axum::Json(body)).into_response();
    resp.headers_mut()
        .insert("x-apiproxy", HeaderValue::from_static("error"));
    resp
}
