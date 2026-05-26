use crate::config::{Endpoint, Rule, RuleAction};
use http::header::HeaderName;
use http::HeaderMap;
use http::HeaderValue;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum TransformError {
    #[error("无法解析上游 URL：{0}")]
    InvalidUpstream(String),
    #[error("请求体不是合法 JSON：{0}")]
    InvalidJsonBody(String),
    #[error("非法 header：{0}")]
    InvalidHeader(String),
}

/// 路由匹配：在 enabled 端点中按 path 长度降序找
/// req.uri.path() 等于 endpoint.path 或以 endpoint.path + "/" 开头的端点
pub fn match_endpoint<'a>(routes: &'a [Endpoint], req_path: &str) -> Option<&'a Endpoint> {
    let mut sorted: Vec<&Endpoint> = routes.iter().filter(|e| e.enabled).collect();
    sorted.sort_by_key(|e| std::cmp::Reverse(e.path.len()));
    sorted
        .into_iter()
        .find(|e| req_path == e.path || req_path.starts_with(&format!("{}/", e.path)))
}

/// 构造上游 URL
pub fn build_upstream_url(
    endpoint: &Endpoint,
    req_path: &str,
    req_query: Option<&str>,
) -> Result<Url, TransformError> {
    let mut url = Url::parse(&endpoint.upstream_url)
        .map_err(|e| TransformError::InvalidUpstream(e.to_string()))?;

    if endpoint.fixed_upstream {
        if let Some(q) = req_query {
            url.set_query(Some(q));
        }
        return Ok(url);
    }

    let remaining = if endpoint.strip_prefix {
        let stripped = req_path.strip_prefix(&endpoint.path).unwrap_or(req_path);
        if stripped.is_empty() {
            ""
        } else {
            stripped
        }
    } else {
        req_path
    };
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
        url.query_pairs_mut()
            .clear()
            .extend_pairs(pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())));
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
        .flat_map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .collect::<Vec<_>>()
        })
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
            fixed_upstream: false,
            header_rules: vec![],
            query_rules: vec![],
            body_merge: "".into(),
            sort_index: 0,
            api_format: crate::proxy::format::ApiFormat::Passthrough,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn rule(action: RuleAction, k: &str, v: &str) -> Rule {
        Rule {
            action,
            key: k.into(),
            value: v.into(),
        }
    }

    #[test]
    fn match_picks_longest_prefix_first() {
        let a = ep("/api", "https://a.com", true);
        let b = ep("/api/v1", "https://b.com", true);
        let routes = vec![a.clone(), b.clone()];
        assert_eq!(
            match_endpoint(&routes, "/api/v1/users").unwrap().path,
            "/api/v1"
        );
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
    fn build_url_fixed_upstream_ignores_path() {
        let mut e = ep("/codex", "https://api.kimi.com/coding/v1/chat/completions", true);
        e.fixed_upstream = true;
        let url = build_upstream_url(&e, "/codex/responses", None).unwrap();
        assert_eq!(
            url.as_str(),
            "https://api.kimi.com/coding/v1/chat/completions"
        );
        let url = build_upstream_url(&e, "/codex/anything/else", None).unwrap();
        assert_eq!(
            url.as_str(),
            "https://api.kimi.com/coding/v1/chat/completions"
        );
    }

    #[test]
    fn build_url_fixed_upstream_keeps_query() {
        let mut e = ep("/codex", "https://api.kimi.com/coding/v1/chat/completions", true);
        e.fixed_upstream = true;
        let url = build_upstream_url(&e, "/codex/responses", Some("stream=true")).unwrap();
        assert_eq!(
            url.as_str(),
            "https://api.kimi.com/coding/v1/chat/completions?stream=true"
        );
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
        let values: Vec<&str> = h
            .get_all("x-a")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .collect();
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
