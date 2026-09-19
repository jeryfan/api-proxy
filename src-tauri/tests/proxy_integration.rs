use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;

use apiproxy_lib::config::{Endpoint, Rule, RuleAction, Upstream};
use apiproxy_lib::proxy::log_store::LogStore;
use apiproxy_lib::proxy::server::spawn_server;

fn upstream(id: &str, url: &str, enabled: bool, weight: u32) -> Upstream {
    Upstream {
        id: id.into(),
        name: id.into(),
        url: url.into(),
        enabled,
        weight,
        header_rules: vec![],
        health: apiproxy_lib::config::HealthConfig::default(),
    }
}

fn ep(path: &str, upstreams: Vec<Upstream>) -> Endpoint {
    Endpoint {
        id: "id".into(),
        name: "ep".into(),
        description: "".into(),
        enabled: true,
        path: path.into(),
        upstreams,
        strip_prefix: true,
        fixed_upstream: false,
        header_rules: vec![],
        query_rules: vec![],
        sort_index: 0,
        created_at: 0,
    }
}

async fn spawn(
    routes: Arc<ArcSwap<Vec<Endpoint>>>,
    timeout_secs: u64,
) -> apiproxy_lib::proxy::server::RunningServer {
    spawn_server(
        "127.0.0.1".into(),
        0,
        routes,
        Arc::new(AtomicU64::new(timeout_secs)),
        Arc::new(LogStore::new(50)),
        apiproxy_lib::proxy::upstream::UpstreamHealthTracker::new(),
        None,
    )
    .await
    .unwrap()
}

/// 回显上游：把 method/path/query/headers/body 以 JSON 返回。
async fn fake_upstream() -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    use axum::extract::Request;
    use axum::routing::any;
    use axum::Router;

    let app = Router::new().fallback(any(|req: Request| async move {
        let method = req.method().clone();
        let path = req.uri().path().to_string();
        let query = req.uri().query().unwrap_or("").to_string();
        let headers: serde_json::Map<String, serde_json::Value> = req
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    serde_json::Value::String(v.to_str().unwrap_or("").to_string()),
                )
            })
            .collect();
        let body = axum::body::to_bytes(req.into_body(), 1024 * 1024)
            .await
            .unwrap_or_default();
        let body_str = String::from_utf8_lossy(&body).to_string();
        axum::Json(serde_json::json!({
            "method": method.as_str(),
            "path": path,
            "query": query,
            "headers": headers,
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

#[tokio::test]
async fn forwards_basic_get() {
    let _ = apiproxy_lib::proxy::http_client::init(None);
    let (upstream_addr, _u) = fake_upstream().await;
    let upstream_url = format!("http://{}/v1", upstream_addr);
    let routes = Arc::new(ArcSwap::from_pointee(vec![ep(
        "/cc",
        vec![upstream("a", &upstream_url, true, 1)],
    )]));
    let server = spawn(routes, 10).await;
    let url = format!("http://{}:{}/cc/hello", server.address, server.port);

    let resp = reqwest::get(&url).await.unwrap();
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["method"], "GET");
    assert_eq!(json["path"], "/v1/hello");

    let _ = server.shutdown.send(());
    server.join.await.ok();
}

#[tokio::test]
async fn forwards_post_with_header_rule_and_passthrough_body() {
    let _ = apiproxy_lib::proxy::http_client::init(None);
    let (upstream_addr, _u) = fake_upstream().await;
    let upstream_url = format!("http://{}/v1", upstream_addr);
    let mut endpoint = ep("/cc", vec![upstream("a", &upstream_url, true, 1)]);
    endpoint.header_rules = vec![Rule {
        action: RuleAction::Set,
        key: "x-test".into(),
        value: "1".into(),
    }];
    let routes = Arc::new(ArcSwap::from_pointee(vec![endpoint]));
    let server = spawn(routes, 10).await;
    let url = format!("http://{}:{}/cc/chat", server.address, server.port);
    let client = reqwest::Client::new();
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
    assert_eq!(json["headers"]["x-test"], "1");
    assert_eq!(json["body"], r#"{"q":"hi","model":"orig"}"#);

    let _ = server.shutdown.send(());
    server.join.await.ok();
}

#[tokio::test]
async fn returns_404_when_no_match() {
    let _ = apiproxy_lib::proxy::http_client::init(None);
    let routes: Arc<ArcSwap<Vec<Endpoint>>> = Arc::new(ArcSwap::from_pointee(vec![]));
    let server = spawn(routes, 5).await;
    let url = format!("http://{}:{}/missing", server.address, server.port);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 404);
    let _ = server.shutdown.send(());
    server.join.await.ok();
}

#[tokio::test]
async fn upstream_timeout_returns_502() {
    let _ = apiproxy_lib::proxy::http_client::init(None);
    use axum::routing::any;
    use axum::Router;

    let app: Router = Router::new().fallback(any(|| async {
        tokio::time::sleep(Duration::from_secs(5)).await;
        "ok"
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_addr = listener.local_addr().unwrap();
    let _upstream = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let upstream_url = format!("http://{}", upstream_addr);
    let routes = Arc::new(ArcSwap::from_pointee(vec![ep(
        "/cc",
        vec![upstream("a", &upstream_url, true, 1)],
    )]));
    let server = spawn(routes, 1).await;
    let url = format!("http://{}:{}/cc/slow", server.address, server.port);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 502);
    let _ = server.shutdown.send(());
    server.join.await.ok();
}

/// 绑定后立即释放的端口：连接被拒绝，模拟上游宕机。
async fn dead_addr() -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap()
}

#[tokio::test]
async fn failover_to_backup_when_primary_down() {
    let _ = apiproxy_lib::proxy::http_client::init(None);
    let dead = format!("http://{}", dead_addr().await);
    let (backup_addr, _u) = fake_upstream().await;
    let backup = format!("http://{}/v1", backup_addr);

    // 单请求：第一次 pick 命中展开列表首位，权重相同按列表顺序 → a 在前。
    let routes = Arc::new(ArcSwap::from_pointee(vec![ep(
        "/cc",
        vec![upstream("a", &dead, true, 1), upstream("b", &backup, true, 1)],
    )]));
    let log_store = Arc::new(LogStore::new(50));
    let server = spawn_server(
        "127.0.0.1".into(),
        0,
        routes,
        Arc::new(AtomicU64::new(5)),
        log_store.clone(),
        apiproxy_lib::proxy::upstream::UpstreamHealthTracker::new(),
        None,
    )
    .await
    .unwrap();
    let url = format!("http://{}:{}/cc/hello", server.address, server.port);

    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["path"], "/v1/hello");

    // 日志应留下转移轨迹，且实际命中备份上游。
    let logs = log_store.list(None, None);
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].upstream_url, format!("{backup}/hello"));
    assert_eq!(logs[0].failover_log.len(), 1);
    let entry = &logs[0].failover_log[0];
    assert!(
        entry.contains("连接失败") || entry.contains("HTTP 502"),
        "actual: {:?}",
        logs[0].failover_log
    );

    let _ = server.shutdown.send(());
    server.join.await.ok();
}

#[tokio::test]
async fn retry_on_500_then_failover() {
    let _ = apiproxy_lib::proxy::http_client::init(None);
    use axum::routing::any;
    use axum::Router;

    // 上游 A：返回 500。
    let app_a: Router = Router::new().fallback(any(|| async {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({"err": "boom"})),
        )
    }));
    let listener_a = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_a = listener_a.local_addr().unwrap();
    let _a = tokio::spawn(async move {
        let _ = axum::serve(listener_a, app_a).await;
    });

    let (addr_b, _u) = fake_upstream().await;
    let url_a = format!("http://{}", addr_a);
    let url_b = format!("http://{}", addr_b);

    let routes = Arc::new(ArcSwap::from_pointee(vec![ep(
        "/cc",
        vec![upstream("a", &url_a, true, 1), upstream("b", &url_b, true, 1)],
    )]));
    let log_store = Arc::new(LogStore::new(50));
    let server = spawn_server(
        "127.0.0.1".into(),
        0,
        routes,
        Arc::new(AtomicU64::new(5)),
        log_store.clone(),
        apiproxy_lib::proxy::upstream::UpstreamHealthTracker::new(),
        None,
    )
    .await
    .unwrap();
    let url = format!("http://{}:{}/cc/ping", server.address, server.port);

    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    let logs = log_store.list(None, None);
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].upstream_url, format!("{url_b}/ping"));
    assert_eq!(logs[0].failover_log.len(), 1);
    assert!(logs[0].failover_log[0].contains("HTTP 500"));

    let _ = server.shutdown.send(());
    server.join.await.ok();
}

#[tokio::test]
async fn all_upstreams_down_returns_502() {
    let _ = apiproxy_lib::proxy::http_client::init(None);
    let dead1 = format!("http://{}", dead_addr().await);
    let dead2 = format!("http://{}", dead_addr().await);
    let routes = Arc::new(ArcSwap::from_pointee(vec![ep(
        "/cc",
        vec![upstream("a", &dead1, true, 1), upstream("b", &dead2, true, 1)],
    )]));
    let log_store = Arc::new(LogStore::new(50));
    let server = spawn_server(
        "127.0.0.1".into(),
        0,
        routes,
        Arc::new(AtomicU64::new(5)),
        log_store.clone(),
        apiproxy_lib::proxy::upstream::UpstreamHealthTracker::new(),
        None,
    )
    .await
    .unwrap();
    let url = format!("http://{}:{}/cc/x", server.address, server.port);

    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 502);

    let logs = log_store.list(None, None);
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].failover_log.len(), 1);

    let _ = server.shutdown.send(());
    server.join.await.ok();
}

#[tokio::test]
async fn per_upstream_header_rules_override_endpoint_rules() {
    let _ = apiproxy_lib::proxy::http_client::init(None);
    let (addr_a, _ua) = fake_upstream().await;
    let (addr_b, _ub) = fake_upstream().await;
    let url_a = format!("http://{}", addr_a);
    let url_b = format!("http://{}", addr_b);

    let mut a = upstream("a", &url_a, true, 1);
    a.header_rules = vec![Rule {
        action: RuleAction::Set,
        key: "x-auth".into(),
        value: "key-a".into(),
    }];
    let mut b = upstream("b", &url_b, true, 1);
    b.header_rules = vec![Rule {
        action: RuleAction::Set,
        key: "x-auth".into(),
        value: "key-b".into(),
    }];
    let mut endpoint = ep("/cc", vec![a, b]);
    // 端点级规则：公共头发给所有上游；x-auth 会被各上游级规则覆盖。
    endpoint.header_rules = vec![
        Rule {
            action: RuleAction::Set,
            key: "x-common".into(),
            value: "yes".into(),
        },
        Rule {
            action: RuleAction::Set,
            key: "x-auth".into(),
            value: "endpoint-default".into(),
        },
    ];
    let routes = Arc::new(ArcSwap::from_pointee(vec![endpoint]));
    let server = spawn(routes, 5).await;
    let url = format!("http://{}:{}/cc/x", server.address, server.port);

    // 轮询：第 1 次 → A，第 2 次 → B。
    let json1: serde_json::Value = reqwest::get(&url).await.unwrap().json().await.unwrap();
    assert_eq!(json1["headers"]["x-common"], "yes");
    assert_eq!(json1["headers"]["x-auth"], "key-a");

    let json2: serde_json::Value = reqwest::get(&url).await.unwrap().json().await.unwrap();
    assert_eq!(json2["headers"]["x-common"], "yes");
    assert_eq!(json2["headers"]["x-auth"], "key-b");

    let _ = server.shutdown.send(());
    server.join.await.ok();
}

#[tokio::test]
async fn upstream_rule_overrides_client_authorization() {
    let _ = apiproxy_lib::proxy::http_client::init(None);
    let (addr_a, _ua) = fake_upstream().await;
    let url_a = format!("http://{}", addr_a);

    let mut a = upstream("a", &url_a, true, 1);
    // 完整复刻用户场景：客户端带占位 Authorization，上游级规则用小写 key 覆盖。
    a.header_rules = vec![Rule {
        action: RuleAction::Set,
        key: "authorization".into(),
        value: "Bearer sk-real-key".into(),
    }];
    let routes = Arc::new(ArcSwap::from_pointee(vec![ep("/cc", vec![a])]));
    let server = spawn(routes, 5).await;
    let url = format!("http://{}:{}/cc/x", server.address, server.port);

    let client = reqwest::Client::new();
    let json: serde_json::Value = client
        .get(&url)
        .header("Authorization", "Bearer 1")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        json["headers"]["authorization"], "Bearer sk-real-key",
        "上游级规则应覆盖客户端的 Authorization"
    );

    let _ = server.shutdown.send(());
    server.join.await.ok();
}

#[tokio::test]
async fn disabled_upstream_not_selected() {
    let _ = apiproxy_lib::proxy::http_client::init(None);
    let (addr_a, _ua) = fake_upstream().await;
    let dead = format!("http://{}", dead_addr().await);
    let url_a = format!("http://{}", addr_a);

    // b 被禁用，即使排在列表首位也不应被选中。
    let b = upstream("b", &dead, false, 1);
    let routes = Arc::new(ArcSwap::from_pointee(vec![ep(
        "/cc",
        vec![b, upstream("a", &url_a, true, 1)],
    )]));
    let log_store = Arc::new(LogStore::new(50));
    let server = spawn_server(
        "127.0.0.1".into(),
        0,
        routes,
        Arc::new(AtomicU64::new(5)),
        log_store.clone(),
        apiproxy_lib::proxy::upstream::UpstreamHealthTracker::new(),
        None,
    )
    .await
    .unwrap();
    let url = format!("http://{}:{}/cc/ok", server.address, server.port);

    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    let logs = log_store.list(None, None);
    assert_eq!(logs[0].upstream_url, format!("{url_a}/ok"));
    assert!(logs[0].failover_log.is_empty());

    let _ = server.shutdown.send(());
    server.join.await.ok();
}

#[tokio::test]
async fn circuit_breaker_trips_and_manual_recovery() {
    let _ = apiproxy_lib::proxy::http_client::init(None);
    use axum::routing::any;
    use axum::Router;

    // 上游 A：持续返回 500
    let app_a: Router = Router::new().fallback(any(|| async {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({"err": "upstream error"})),
        )
    }));
    let listener_a = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_a = listener_a.local_addr().unwrap();
    let _a = tokio::spawn(async move {
        let _ = axum::serve(listener_a, app_a).await;
    });

    // 上游 B：正常返回 200
    let (addr_b, _ub) = fake_upstream().await;
    let url_a = format!("http://{}", addr_a);
    let url_b = format!("http://{}", addr_b);

    let mut a = upstream("a", &url_a, true, 1);
    a.health.enabled = true;
    a.health.failure_threshold = 2;

    let mut b = upstream("b", &url_b, true, 1);
    b.health.enabled = true;

    let tracker = apiproxy_lib::proxy::upstream::UpstreamHealthTracker::new();
    let routes = Arc::new(ArcSwap::from_pointee(vec![ep("/cc", vec![a, b])]));
    let log_store = Arc::new(LogStore::new(50));
    let server = spawn_server(
        "127.0.0.1".into(),
        0,
        routes,
        Arc::new(AtomicU64::new(5)),
        log_store.clone(),
        tracker.clone(),
        None,
    )
    .await
    .unwrap();
    let url = format!("http://{}:{}/cc/test", server.address, server.port);

    // 第 1 次请求：先尝试 A（500 失败），转移到 B（成功 200）
    let resp1 = reqwest::get(&url).await.unwrap();
    assert_eq!(resp1.status(), 200);
    assert_eq!(tracker.get("id", "a").consecutive_failures, 1);
    assert!(!tracker.is_tripped("id", "a"));

    // 第 2 次请求：轮询命中 B（成功 200），不经过 A
    let resp2 = reqwest::get(&url).await.unwrap();
    assert_eq!(resp2.status(), 200);
    assert_eq!(tracker.get("id", "a").consecutive_failures, 1);

    // 第 3 次请求：轮询再次命中 A（500 失败），转移到 B，达到阈值 2 触发熔断
    let resp3 = reqwest::get(&url).await.unwrap();
    assert_eq!(resp3.status(), 200);
    assert_eq!(tracker.get("id", "a").consecutive_failures, 2);
    assert!(tracker.is_tripped("id", "a"), "A 应被标记为已熔断");

    // 第 4 次请求：由于 A 已熔断，直接命中 B，无转移日志
    let resp4 = reqwest::get(&url).await.unwrap();
    assert_eq!(resp4.status(), 200);
    let logs = log_store.list(None, None);
    // 最新一条日志为第 4 次请求
    assert_eq!(logs[0].upstream_url, format!("{url_b}/test"));
    assert!(logs[0].failover_log.is_empty(), "A 已熔断，不应再被尝试");

    // 手动重置恢复 A
    tracker.reset("id", "a");
    assert!(!tracker.is_tripped("id", "a"));
    assert_eq!(tracker.get("id", "a").consecutive_failures, 0);

    let _ = server.shutdown.send(());
    server.join.await.ok();
}

#[tokio::test]
async fn circuit_breaker_response_body_regex_match() {
    let _ = apiproxy_lib::proxy::http_client::init(None);
    use axum::routing::any;
    use axum::Router;

    // 上游 A：HTTP 200，但 body 返回业务错误信息
    let app_a: Router = Router::new().fallback(any(|| async {
        (
            axum::http::StatusCode::OK,
            axum::Json(serde_json::json!({
                "error": {
                    "message": "You exceeded your current quota, please check your plan",
                    "type": "insufficient_quota",
                    "code": "insufficient_quota"
                }
            })),
        )
    }));
    let listener_a = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_a = listener_a.local_addr().unwrap();
    let _a = tokio::spawn(async move {
        let _ = axum::serve(listener_a, app_a).await;
    });

    let (addr_b, _ub) = fake_upstream().await;
    let url_a = format!("http://{}", addr_a);
    let url_b = format!("http://{}", addr_b);

    let mut a = upstream("a", &url_a, true, 1);
    a.health.enabled = true;
    a.health.failure_threshold = 1;
    a.health.body_match = Some(apiproxy_lib::config::BodyMatch {
        mode: apiproxy_lib::config::BodyMatchMode::Regex,
        pattern: "insufficient_quota".into(),
    });

    let b = upstream("b", &url_b, true, 1);

    let tracker = apiproxy_lib::proxy::upstream::UpstreamHealthTracker::new();
    let routes = Arc::new(ArcSwap::from_pointee(vec![ep("/cc", vec![a, b])]));
    let log_store = Arc::new(LogStore::new(50));
    let server = spawn_server(
        "127.0.0.1".into(),
        0,
        routes,
        Arc::new(AtomicU64::new(5)),
        log_store.clone(),
        tracker.clone(),
        None,
    )
    .await
    .unwrap();
    let url = format!("http://{}:{}/cc/v1/chat", server.address, server.port);

    // 发起请求：A 返回 200，但 body 命中正则 "insufficient_quota"，被判定为失败并转移到 B
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    let logs = log_store.list(None, None);
    assert_eq!(logs[0].upstream_url, format!("{url_b}/v1/chat"));
    assert_eq!(logs[0].failover_log.len(), 1);
    assert!(logs[0].failover_log[0].contains("insufficient_quota"));

    // A 达到阈值 1，应已熔断
    assert!(tracker.is_tripped("id", "a"));

    let _ = server.shutdown.send(());
    server.join.await.ok();
}

