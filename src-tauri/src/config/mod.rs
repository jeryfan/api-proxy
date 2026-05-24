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

fn default_true() -> bool {
    true
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
    DuplicatePath {
        path: String,
        conflict_with: String,
    },
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
        let e = ep("/cc 1", true);
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
