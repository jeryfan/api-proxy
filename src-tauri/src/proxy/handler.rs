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
        resp_headers.insert(
            "x-apiproxy-warn",
            HeaderValue::from_static("body-merge-skipped"),
        );
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
    resp.headers_mut()
        .insert("x-apiproxy", HeaderValue::from_static("error"));
    resp
}
