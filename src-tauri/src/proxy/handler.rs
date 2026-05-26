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

#[derive(Clone)]
pub struct ProxyState {
    pub routes: Arc<ArcSwap<Vec<Endpoint>>>,
    pub timeout: Arc<std::sync::atomic::AtomicU64>,
    pub log_store: Arc<LogStore>,
    pub app_handle: Option<AppHandle<Wry>>,
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
        endpoint_name: endpoint.name.clone(),
        started_at: Utc::now().timestamp_millis(),
        client_addr,
        req_method: req_method.to_string(),
        req_path: req_path.to_string(),
        req_query: req_query.clone(),
        req_headers: header_entries(req_headers),
        req_body_b64: String::new(),
        req_body_len: 0,
        req_body_binary: false,
        upstream_url: endpoint.upstream_url.clone(),
        upstream_headers: vec![],
        upstream_body_b64: String::new(),
        upstream_body_len: 0,
        status_code: None,
        resp_headers: vec![],
        resp_body_b64: String::new(),
        resp_body_len: 0,
        resp_body_binary: false,
        resp_content_type: None,
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
                None,
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

    let mut upstream_url =
        match transform::build_upstream_url(&endpoint, &req_path, req_query.as_deref()) {
            Ok(u) => u,
            Err(e) => {
                log.error = Some(e.to_string());
                finalize_log(&state, log, started);
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
    log.upstream_url = upstream_url.to_string();

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
        log.error = Some(e.to_string());
        finalize_log(&state, log, started);
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
    let raw_request_body: Vec<u8>;
    let body_data: BodyData =
        match prepare_body(body, &mut headers, &endpoint, &mut warn_body_merge_skipped).await {
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
                    None,
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
    let upstream_body_bytes: Vec<u8> = match &body_data {
        BodyData::Empty => Vec::new(),
        BodyData::Bytes(b) => b.clone(),
    };
    {
        let (b64, len) = encode_body(&upstream_body_bytes);
        log.upstream_body_b64 = b64;
        log.upstream_body_len = len;
        log.upstream_headers = header_entries(&headers);
    }

    let client = crate::proxy::http_client::get();
    let mut req_builder = client
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
            log.error = Some(format!("{code}: {e}"));
            finalize_log(&state, log, started);
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
    transform::strip_hop_by_hop(&mut resp_headers);
    if warn_body_merge_skipped {
        resp_headers.insert(
            "x-apiproxy-warn",
            HeaderValue::from_static("body-merge-skipped"),
        );
    }
    log.resp_headers = header_entries(&resp_headers);
    let content_type = content_type_of(&resp_headers);
    log.resp_content_type = content_type.clone();
    let skip_resp_body = content_type
        .as_deref()
        .map(|c| is_binary_content_type(c))
        .unwrap_or(false);
    log.resp_body_binary = skip_resp_body;

    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(16);
    let log_state = state.clone();
    let log_started = started;
    let mut prelog = log;
    let mut upstream_stream = upstream_response.bytes_stream();

    tokio::spawn(async move {
        let mut buf: Vec<u8> = Vec::new();
        while let Some(chunk_result) = upstream_stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let bytes = Bytes::from(chunk);
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
                    let _ = tx
                        .send(Err(std::io::Error::new(std::io::ErrorKind::Other, e)))
                        .await;
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

enum BodyData {
    Empty,
    Bytes(Vec<u8>),
}

async fn prepare_body(
    body: Body,
    headers: &mut HeaderMap,
    endpoint: &Endpoint,
    warn: &mut bool,
) -> Result<(BodyData, Vec<u8>), String> {
    use http_body_util::BodyExt;
    let collected = body.collect().await.map_err(|e| e.to_string())?;
    let raw = collected.to_bytes().to_vec();

    if endpoint.body_merge.is_empty() {
        if raw.is_empty() {
            return Ok((BodyData::Empty, raw));
        }
        headers.remove(http::header::CONTENT_LENGTH);
        let bytes = raw.clone();
        return Ok((BodyData::Bytes(bytes), raw));
    }

    let patch: serde_json::Value = serde_json::from_str(&endpoint.body_merge)
        .map_err(|e| format!("body_merge 解析失败：{e}"))?;
    let content_type = headers
        .get(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let is_json = content_type.contains("application/json") || raw.is_empty();
    if !is_json {
        *warn = true;
        headers.remove(http::header::CONTENT_LENGTH);
        return Ok((BodyData::Bytes(raw.clone()), raw));
    }
    match transform::merge_json_body(&raw, &patch) {
        Ok(merged) => {
            headers.insert(
                http::header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            headers.insert(
                http::header::CONTENT_LENGTH,
                HeaderValue::from_str(&merged.len().to_string()).unwrap(),
            );
            Ok((BodyData::Bytes(merged), raw))
        }
        Err(_) => {
            *warn = true;
            headers.remove(http::header::CONTENT_LENGTH);
            Ok((BodyData::Bytes(raw.clone()), raw))
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
    resp.headers_mut()
        .insert("x-apiproxy", HeaderValue::from_static("error"));
    resp
}
