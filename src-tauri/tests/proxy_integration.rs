use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;

use apiproxy_lib::config::{Endpoint, Rule, RuleAction};
use apiproxy_lib::proxy::server::spawn_server;

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
    let _ = apiproxy_lib::proxy::http_client::init(None);
    let (upstream_addr, _u) = fake_upstream().await;
    let upstream = format!("http://{}/v1", upstream_addr);
    let routes = Arc::new(ArcSwap::from_pointee(vec![ep("/cc", &upstream, "")]));
    let timeout = Arc::new(AtomicU64::new(10));
    let client = reqwest::Client::new();
    let server = spawn_server("127.0.0.1".into(), 0, routes, timeout)
        .await
        .unwrap();
    let url = format!("http://{}:{}/cc/hello", server.address, server.port);

    let resp = reqwest::get(&url).await.unwrap();
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["method"], "GET");
    assert_eq!(json["path"], "/v1/hello");

    let _ = client;
    let _ = server.shutdown.send(());
    server.join.await.ok();
}

#[tokio::test]
async fn forwards_post_with_body_merge_and_header() {
    let _ = apiproxy_lib::proxy::http_client::init(None);
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
    let server = spawn_server("127.0.0.1".into(), 0, routes, timeout)
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
    assert_eq!(json["headers"]["x-test"], "1");
    let body_obj: serde_json::Value = serde_json::from_str(json["body"].as_str().unwrap()).unwrap();
    assert_eq!(body_obj["q"], "hi");
    assert_eq!(body_obj["model"], "x"); // patch wins

    let _ = server.shutdown.send(());
    server.join.await.ok();
}

#[tokio::test]
async fn returns_404_when_no_match() {
    let _ = apiproxy_lib::proxy::http_client::init(None);
    let routes: Arc<ArcSwap<Vec<Endpoint>>> = Arc::new(ArcSwap::from_pointee(vec![]));
    let timeout = Arc::new(AtomicU64::new(5));
    let server = spawn_server("127.0.0.1".into(), 0, routes, timeout)
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

    let upstream = format!("http://{}", upstream_addr);
    let routes = Arc::new(ArcSwap::from_pointee(vec![ep("/cc", &upstream, "")]));
    let timeout = Arc::new(AtomicU64::new(1));
    let server = spawn_server("127.0.0.1".into(), 0, routes, timeout)
        .await
        .unwrap();
    let url = format!("http://{}:{}/cc/slow", server.address, server.port);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 502);
    let _ = server.shutdown.send(());
    server.join.await.ok();
}
