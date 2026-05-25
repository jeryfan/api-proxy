//! 全局 HTTP 客户端模块
//!
//! 提供支持全局代理配置的 reqwest 客户端。
//! 所有发往上游的 HTTP 请求都应通过本模块取 client。

use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use reqwest::Client;

static GLOBAL_CLIENT: OnceLock<RwLock<Client>> = OnceLock::new();
static CURRENT_PROXY_URL: OnceLock<RwLock<Option<String>>> = OnceLock::new();

/// 初始化全局客户端（应用启动时调用一次）
pub fn init(proxy_url: Option<&str>) -> Result<(), String> {
    let effective = proxy_url.filter(|s| !s.trim().is_empty());
    let client = build_client(effective)?;
    if GLOBAL_CLIENT.set(RwLock::new(client)).is_err() {
        return apply_proxy(proxy_url);
    }
    let _ = CURRENT_PROXY_URL.set(RwLock::new(effective.map(|s| s.to_string())));
    Ok(())
}

/// 仅验证代理 URL，不应用
pub fn validate_proxy(proxy_url: Option<&str>) -> Result<(), String> {
    let effective = proxy_url.filter(|s| !s.trim().is_empty());
    build_client(effective).map(|_| ())
}

/// 应用代理（已 validate 过）
pub fn apply_proxy(proxy_url: Option<&str>) -> Result<(), String> {
    let effective = proxy_url.filter(|s| !s.trim().is_empty());
    let new_client = build_client(effective)?;
    if let Some(lock) = GLOBAL_CLIENT.get() {
        let mut c = lock.write().map_err(|_| "client lock poisoned".to_string())?;
        *c = new_client;
    } else {
        return init(proxy_url);
    }
    if let Some(lock) = CURRENT_PROXY_URL.get() {
        let mut u = lock.write().map_err(|_| "url lock poisoned".to_string())?;
        *u = effective.map(|s| s.to_string());
    }
    Ok(())
}

/// 取当前客户端
pub fn get() -> Client {
    GLOBAL_CLIENT
        .get()
        .and_then(|lock| lock.read().ok())
        .map(|c| c.clone())
        .unwrap_or_else(|| build_client(None).unwrap_or_default())
}

/// 当前代理 URL（用于状态展示）
pub fn get_current_proxy_url() -> Option<String> {
    CURRENT_PROXY_URL
        .get()
        .and_then(|lock| lock.read().ok())
        .and_then(|u| u.clone())
}

/// 把 URL 中的 username:password 中的密码部分换成 ***（用于日志）
pub fn mask_url(url: &str) -> String {
    match url::Url::parse(url) {
        Ok(mut u) => {
            if !u.password().unwrap_or("").is_empty() {
                let _ = u.set_password(Some("***"));
            }
            u.to_string()
        }
        Err(_) => url.to_string(),
    }
}

fn build_client(proxy_url: Option<&str>) -> Result<Client, String> {
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(600))
        .connect_timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(10)
        .tcp_keepalive(Duration::from_secs(60));

    if let Some(url) = proxy_url {
        let parsed = url::Url::parse(url)
            .map_err(|e| format!("代理 URL 无效 '{}': {}", mask_url(url), e))?;
        let scheme = parsed.scheme();
        if !["http", "https", "socks5", "socks5h"].contains(&scheme) {
            return Err(format!(
                "不支持的代理协议 '{}'，支持 http / https / socks5 / socks5h",
                scheme
            ));
        }
        let proxy = reqwest::Proxy::all(url)
            .map_err(|e| format!("构造代理失败 '{}': {}", mask_url(url), e))?;
        builder = builder.proxy(proxy);
    }

    builder
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败：{}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_http_proxy() {
        assert!(validate_proxy(Some("http://127.0.0.1:7890")).is_ok());
    }

    #[test]
    fn validate_accepts_socks5_proxy() {
        assert!(validate_proxy(Some("socks5://127.0.0.1:1080")).is_ok());
        assert!(validate_proxy(Some("socks5h://127.0.0.1:1080")).is_ok());
    }

    #[test]
    fn validate_accepts_https_proxy() {
        assert!(validate_proxy(Some("https://proxy.example.com:443")).is_ok());
    }

    #[test]
    fn validate_accepts_empty_as_direct() {
        assert!(validate_proxy(None).is_ok());
        assert!(validate_proxy(Some("")).is_ok());
        assert!(validate_proxy(Some("   ")).is_ok());
    }

    #[test]
    fn validate_rejects_ftp_scheme() {
        let err = validate_proxy(Some("ftp://x.com")).unwrap_err();
        assert!(err.contains("不支持的代理协议"));
    }

    #[test]
    fn validate_rejects_garbage_url() {
        assert!(validate_proxy(Some("not a url")).is_err());
    }

    #[test]
    fn validate_accepts_url_with_auth() {
        assert!(validate_proxy(Some("http://user:pass@127.0.0.1:7890")).is_ok());
    }

    #[test]
    fn mask_url_hides_password() {
        let m = mask_url("http://alice:s3cret@127.0.0.1:7890");
        assert!(m.contains("alice"));
        assert!(!m.contains("s3cret"));
        assert!(m.contains("***"));
    }

    #[test]
    fn mask_url_passthrough_when_no_password() {
        assert_eq!(mask_url("http://127.0.0.1:7890"), "http://127.0.0.1:7890/");
    }
}
