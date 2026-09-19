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

/// 响应体匹配模式：子串包含或正则。仅对**非流式**响应生效。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BodyMatchMode {
    #[default]
    Contains,
    Regex,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BodyMatch {
    #[serde(default)]
    pub mode: BodyMatchMode,
    #[serde(default)]
    pub pattern: String,
}

impl BodyMatch {
    /// 空 pattern 视为不匹配（等价于未配置）。
    pub fn is_match(&self, body: &str) -> bool {
        if self.pattern.is_empty() {
            return false;
        }
        match self.mode {
            BodyMatchMode::Contains => body.contains(&self.pattern),
            BodyMatchMode::Regex => regex::Regex::new(&self.pattern)
                .map(|re| re.is_match(body))
                .unwrap_or(false),
        }
    }
}

fn default_failure_threshold() -> u32 {
    3
}

fn default_status_codes() -> Vec<u16> {
    vec![429, 500, 502, 503, 504]
}

/// 上游健康检查（熔断）配置。默认关闭；关闭时行为与无此字段时完全一致。
/// 熔断打开后**仅能手动恢复**，无自动冷却/半开。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,
    /// 连接失败/超时是否计入失败次数。
    #[serde(default = "default_true")]
    pub count_connect_error: bool,
    /// 命中即计为失败并触发请求内转移的状态码。
    #[serde(default = "default_status_codes")]
    pub status_codes: Vec<u16>,
    /// 响应体匹配（仅非流式）；命中即计为失败。
    #[serde(default)]
    pub body_match: Option<BodyMatch>,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            failure_threshold: default_failure_threshold(),
            count_connect_error: true,
            status_codes: default_status_codes(),
            body_match: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Upstream {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub url: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_weight")]
    pub weight: u32,
    /// 本上游独有的请求头规则，发往该上游时**在端点级规则之后**应用，
    /// 同 key 覆盖端点级规则（典型场景：每个上游各自的 Authorization）。
    #[serde(default)]
    pub header_rules: Vec<Rule>,
    #[serde(default)]
    pub health: HealthConfig,
}

fn default_weight() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub enabled: bool,
    pub path: String,
    pub upstreams: Vec<Upstream>,
    #[serde(default = "default_true")]
    pub strip_prefix: bool,
    #[serde(default)]
    pub fixed_upstream: bool,
    #[serde(default)]
    pub header_rules: Vec<Rule>,
    #[serde(default)]
    pub query_rules: Vec<Rule>,
    #[serde(default)]
    pub sort_index: i32,
    pub created_at: i64,
}

/// 旧版配置迁移：同时接受旧键 `upstreamUrl`（单上游）与新键 `upstreams`，
/// 旧数据读入时转为单元素 upstreams（enabled、weight=1）。旧键只读不写。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EndpointShadow {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    enabled: bool,
    path: String,
    #[serde(default)]
    upstream_url: Option<String>,
    #[serde(default)]
    upstreams: Option<Vec<Upstream>>,
    #[serde(default = "default_true")]
    strip_prefix: bool,
    #[serde(default)]
    fixed_upstream: bool,
    #[serde(default)]
    header_rules: Vec<Rule>,
    #[serde(default)]
    query_rules: Vec<Rule>,
    #[serde(default)]
    sort_index: i32,
    created_at: i64,
}

impl<'de> Deserialize<'de> for Endpoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = EndpointShadow::deserialize(deserializer)?;
        let upstreams = match (s.upstreams, s.upstream_url) {
            (Some(u), _) => u,
            (None, Some(url)) => vec![Upstream {
                id: uuid::Uuid::new_v4().to_string(),
                name: String::new(),
                url,
                enabled: true,
                weight: 1,
                header_rules: vec![],
                health: HealthConfig::default(),
            }],
            (None, None) => vec![],
        };
        Ok(Endpoint {
            id: s.id,
            name: s.name,
            description: s.description,
            enabled: s.enabled,
            path: s.path,
            upstreams,
            strip_prefix: s.strip_prefix,
            fixed_upstream: s.fixed_upstream,
            header_rules: s.header_rules,
            query_rules: s.query_rules,
            sort_index: s.sort_index,
            created_at: s.created_at,
        })
    }
}

fn default_true() -> bool {
    true
}

/// 校验一条 header 规则：key 必须是合法 header 名；非 remove 动作的 value 必须是合法 header 值。
fn validate_rule(rule: &Rule) -> Result<(), String> {
    http::HeaderName::from_bytes(rule.key.as_bytes())
        .map_err(|_| format!("header 名非法：{}", rule.key))?;
    if rule.action != RuleAction::Remove {
        http::HeaderValue::from_str(&rule.value)
            .map_err(|_| format!("header 值非法：{}", rule.value))?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalConfig {
    pub listen_address: String,
    pub listen_port: u16,
    pub request_timeout_secs: u64,
    pub theme: String,
    pub close_to_tray: bool,
    pub auto_start_server: bool,
    #[serde(default)]
    pub proxy_url: String,
    #[serde(default = "default_log_buffer_capacity")]
    pub log_buffer_capacity: u32,
}

fn default_log_buffer_capacity() -> u32 {
    10
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
            proxy_url: String::new(),
            log_buffer_capacity: 10,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub running: bool,
    pub listen_address: Option<String>,
    pub listen_port: Option<u16>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("名称不能为空且不超过 60 字符")]
    InvalidName,
    #[error("路径无效：{0}")]
    InvalidPath(String),
    #[error("上游地址无效：{0}")]
    InvalidUrl(String),
    #[error("至少需要一个上游")]
    NoUpstream,
    #[error("至少启用一个上游")]
    NoEnabledUpstream,
    #[error("上游权重必须在 1-100：{0}")]
    InvalidWeight(u32),
    /// 上游请求头规则非法。
    #[error("上游请求头规则非法：{0}")]
    InvalidHeaderRule(String),
    /// 上游健康配置非法。
    #[error("上游熔断阈值必须 >= 1：{0}")]
    InvalidFailureThreshold(u32),
    #[error("上游响应体正则匹配无效：{0}")]
    InvalidBodyRegex(String),
    #[error("端点路径 '{path}' 与已有端点 '{conflict_with}' 冲突")]
    DuplicatePath {
        path: String,
        conflict_with: String,
    },
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
        for u in self.upstreams.iter_mut() {
            u.name = u.name.trim().to_string();
            u.url = u.url.trim().to_string();
            if u.id.is_empty() {
                u.id = uuid::Uuid::new_v4().to_string();
            }
        }
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
        if self.upstreams.is_empty() {
            return Err(ConfigError::NoUpstream);
        }
        if !self.upstreams.iter().any(|u| u.enabled) {
            return Err(ConfigError::NoEnabledUpstream);
        }
        for r in &self.header_rules {
            validate_rule(r).map_err(ConfigError::InvalidHeaderRule)?;
        }
        for u in &self.upstreams {
            match Url::parse(&u.url) {
                Ok(p) if matches!(p.scheme(), "http" | "https") => {}
                _ => return Err(ConfigError::InvalidUrl(u.url.clone())),
            }
            if u.weight == 0 || u.weight > 100 {
                return Err(ConfigError::InvalidWeight(u.weight));
            }
            for r in &u.header_rules {
                validate_rule(r).map_err(ConfigError::InvalidHeaderRule)?;
            }
            // 健康配置校验
            if u.health.enabled {
                if u.health.failure_threshold < 1 {
                    return Err(ConfigError::InvalidFailureThreshold(u.health.failure_threshold));
                }
                if let Some(ref bm) = u.health.body_match {
                    if !bm.pattern.is_empty() && bm.mode == BodyMatchMode::Regex {
                        regex::Regex::new(&bm.pattern).map_err(|_| {
                            ConfigError::InvalidBodyRegex(bm.pattern.clone())
                        })?;
                    }
                }
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

    fn upstream(url: &str, enabled: bool, weight: u32) -> Upstream {
        Upstream {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            url: url.into(),
            enabled,
            weight,
            header_rules: vec![],
            health: HealthConfig::default(),
        }
    }

    fn ep(path: &str, enabled: bool) -> Endpoint {
        Endpoint {
            id: "id".into(),
            name: "ep".into(),
            description: "".into(),
            enabled,
            path: path.into(),
            upstreams: vec![upstream("https://example.com", true, 1)],
            strip_prefix: true,
            fixed_upstream: false,
            header_rules: vec![],
            query_rules: vec![],
            sort_index: 0,
            created_at: 0,
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
        let e = ep("/cc 1", true);
        assert!(matches!(e.validate(), Err(ConfigError::InvalidPath(_))));
    }

    #[test]
    fn validate_rejects_bad_url() {
        let mut e = ep("/cc", true);
        e.upstreams[0].url = "ftp://x.com".into();
        assert!(matches!(e.validate(), Err(ConfigError::InvalidUrl(_))));
    }

    #[test]
    fn validate_rejects_no_upstream() {
        let mut e = ep("/cc", true);
        e.upstreams = vec![];
        assert!(matches!(e.validate(), Err(ConfigError::NoUpstream)));
    }

    #[test]
    fn validate_rejects_all_disabled() {
        let mut e = ep("/cc", true);
        e.upstreams[0].enabled = false;
        assert!(matches!(e.validate(), Err(ConfigError::NoEnabledUpstream)));
    }

    #[test]
    fn validate_rejects_bad_weight() {
        let mut e = ep("/cc", true);
        e.upstreams[0].weight = 0;
        assert!(matches!(e.validate(), Err(ConfigError::InvalidWeight(0))));
        e.upstreams[0].weight = 101;
        assert!(matches!(e.validate(), Err(ConfigError::InvalidWeight(101))));
    }

    #[test]
    fn validate_rejects_invalid_upstream_header_rule() {
        let mut e = ep("/cc", true);
        e.upstreams[0].header_rules = vec![Rule {
            action: RuleAction::Set,
            key: "bad header".into(),
            value: "1".into(),
        }];
        assert!(matches!(
            e.validate(),
            Err(ConfigError::InvalidHeaderRule(_))
        ));
        e.upstreams[0].header_rules = vec![Rule {
            action: RuleAction::Set,
            key: "x-ok".into(),
            value: "1".into(),
        }];
        assert!(e.validate().is_ok());
    }

    #[test]
    fn validate_rejects_invalid_endpoint_header_rule() {
        let mut e = ep("/cc", true);
        e.header_rules = vec![Rule {
            action: RuleAction::Set,
            key: "bad header".into(),
            value: "1".into(),
        }];
        assert!(matches!(
            e.validate(),
            Err(ConfigError::InvalidHeaderRule(_))
        ));
    }

    #[test]
    fn migrate_legacy_single_upstream() {
        let json = serde_json::json!({
            "id": "id",
            "name": "ep",
            "description": "",
            "enabled": true,
            "path": "/cc",
            "upstreamUrl": "https://legacy.example.com/v1",
            "stripPrefix": true,
            "fixedUpstream": false,
            "headerRules": [],
            "queryRules": [],
            "sortIndex": 0,
            "createdAt": 0,
        });
        let e: Endpoint = serde_json::from_value(json).unwrap();
        assert_eq!(e.upstreams.len(), 1);
        assert_eq!(e.upstreams[0].url, "https://legacy.example.com/v1");
        assert!(e.upstreams[0].enabled);
        assert_eq!(e.upstreams[0].weight, 1);
        assert!(!e.upstreams[0].id.is_empty());
    }

    #[test]
    fn deserialize_new_multi_upstreams() {
        let json = serde_json::json!({
            "id": "id",
            "name": "ep",
            "enabled": true,
            "path": "/cc",
            "upstreams": [
                {"id": "u1", "url": "https://a.com", "enabled": true, "weight": 3},
                {"id": "u2", "url": "https://b.com", "enabled": false, "weight": 1},
            ],
            "createdAt": 0,
        });
        let e: Endpoint = serde_json::from_value(json).unwrap();
        assert_eq!(e.upstreams.len(), 2);
        assert_eq!(e.upstreams[0].weight, 3);
        assert!(!e.upstreams[1].enabled);
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
