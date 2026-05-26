//! 请求日志环形缓冲区
//!
//! 内存中保存最近 N 条 HTTP 请求的完整记录（method/headers/body/status/error/duration）。
//! 重启清空。线程安全。

use std::collections::VecDeque;
use std::sync::RwLock;

use serde::Serialize;

/// 单条 body 上限（5 MB）。超过截断，并把 `truncated_*` 字段置 true。
pub const BODY_LIMIT_BYTES: usize = 5 * 1024 * 1024;

/// 视为不可读二进制响应的 content-type 前缀；这些响应只记录元信息，body 字段留空。
const BINARY_CONTENT_TYPE_PREFIXES: &[&str] = &[
    "image/",
    "audio/",
    "video/",
    "application/octet-stream",
    "application/pdf",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestLog {
    pub id: String,
    pub endpoint_id: String,
    pub endpoint_name: String,
    pub started_at: i64,
    pub client_addr: Option<String>,

    pub req_method: String,
    pub req_path: String,
    pub req_query: Option<String>,
    pub req_headers: Vec<HeaderEntry>,
    pub req_body_b64: String,
    pub req_body_len: usize,
    pub req_body_truncated: bool,
    pub req_body_binary: bool,

    pub upstream_url: String,
    pub upstream_headers: Vec<HeaderEntry>,
    pub upstream_body_b64: String,
    pub upstream_body_len: usize,
    pub upstream_body_truncated: bool,

    pub status_code: Option<u16>,
    pub resp_headers: Vec<HeaderEntry>,
    pub resp_body_b64: String,
    pub resp_body_len: usize,
    pub resp_body_truncated: bool,
    pub resp_body_binary: bool,
    pub resp_content_type: Option<String>,

    pub duration_ms: u64,
    pub error: Option<String>,
}

pub struct LogStore {
    inner: RwLock<VecDeque<RequestLog>>,
    capacity: usize,
}

impl LogStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: RwLock::new(VecDeque::with_capacity(capacity)),
            capacity,
        }
    }

    pub fn push(&self, log: RequestLog) {
        if let Ok(mut q) = self.inner.write() {
            if q.len() >= self.capacity {
                q.pop_front();
            }
            q.push_back(log);
        }
    }

    pub fn list(&self, endpoint_id: Option<&str>, limit: Option<usize>) -> Vec<RequestLog> {
        let q = match self.inner.read() {
            Ok(g) => g,
            Err(_) => return vec![],
        };
        let iter = q.iter().rev().filter(|l| {
            endpoint_id
                .map(|id| l.endpoint_id == id)
                .unwrap_or(true)
        });
        match limit {
            Some(n) => iter.take(n).cloned().collect(),
            None => iter.cloned().collect(),
        }
    }

    pub fn get(&self, id: &str) -> Option<RequestLog> {
        let q = self.inner.read().ok()?;
        q.iter().find(|l| l.id == id).cloned()
    }

    pub fn clear(&self, endpoint_id: Option<&str>) {
        if let Ok(mut q) = self.inner.write() {
            match endpoint_id {
                None => q.clear(),
                Some(id) => q.retain(|l| l.endpoint_id != id),
            }
        }
    }
}

/// 判断 content-type 是否为不可读二进制（用于跳过 body 记录）
pub fn is_binary_content_type(content_type: &str) -> bool {
    let lower = content_type.to_ascii_lowercase();
    BINARY_CONTENT_TYPE_PREFIXES
        .iter()
        .any(|p| lower.starts_with(p))
}

/// 把字节数组裁切到上限并 base64 编码。返回 (b64, original_len, truncated)。
pub fn encode_body(bytes: &[u8]) -> (String, usize, bool) {
    use base64::{engine::general_purpose, Engine};
    let len = bytes.len();
    let truncated = len > BODY_LIMIT_BYTES;
    let slice = if truncated {
        &bytes[..BODY_LIMIT_BYTES]
    } else {
        bytes
    };
    (general_purpose::STANDARD.encode(slice), len, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str, endpoint_id: &str) -> RequestLog {
        RequestLog {
            id: id.into(),
            endpoint_id: endpoint_id.into(),
            endpoint_name: "n".into(),
            started_at: 0,
            client_addr: None,
            req_method: "GET".into(),
            req_path: "/x".into(),
            req_query: None,
            req_headers: vec![],
            req_body_b64: "".into(),
            req_body_len: 0,
            req_body_truncated: false,
            req_body_binary: false,
            upstream_url: "https://example.com/x".into(),
            upstream_headers: vec![],
            upstream_body_b64: "".into(),
            upstream_body_len: 0,
            upstream_body_truncated: false,
            status_code: Some(200),
            resp_headers: vec![],
            resp_body_b64: "".into(),
            resp_body_len: 0,
            resp_body_truncated: false,
            resp_body_binary: false,
            resp_content_type: None,
            duration_ms: 12,
            error: None,
        }
    }

    #[test]
    fn ring_buffer_evicts_oldest() {
        let store = LogStore::new(3);
        store.push(sample("1", "ep1"));
        store.push(sample("2", "ep1"));
        store.push(sample("3", "ep1"));
        store.push(sample("4", "ep1"));
        let all = store.list(None, None);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].id, "4");
        assert_eq!(all[2].id, "2");
    }

    #[test]
    fn list_filters_by_endpoint() {
        let store = LogStore::new(10);
        store.push(sample("1", "ep1"));
        store.push(sample("2", "ep2"));
        store.push(sample("3", "ep1"));
        let ep1 = store.list(Some("ep1"), None);
        assert_eq!(ep1.len(), 2);
        assert!(ep1.iter().all(|l| l.endpoint_id == "ep1"));
    }

    #[test]
    fn clear_by_endpoint_keeps_others() {
        let store = LogStore::new(10);
        store.push(sample("1", "ep1"));
        store.push(sample("2", "ep2"));
        store.clear(Some("ep1"));
        let all = store.list(None, None);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].endpoint_id, "ep2");
    }

    #[test]
    fn binary_detector() {
        assert!(is_binary_content_type("image/png"));
        assert!(is_binary_content_type("APPLICATION/Octet-Stream; charset=x"));
        assert!(!is_binary_content_type("application/json"));
        assert!(!is_binary_content_type("text/event-stream"));
    }

    #[test]
    fn encode_body_passes_small_body() {
        let (b64, len, trunc) = encode_body(b"hello");
        assert_eq!(len, 5);
        assert!(!trunc);
        assert!(!b64.is_empty());
    }

    #[test]
    fn encode_body_truncates_oversized() {
        let big = vec![b'x'; BODY_LIMIT_BYTES + 1024];
        let (_b64, len, trunc) = encode_body(&big);
        assert_eq!(len, BODY_LIMIT_BYTES + 1024);
        assert!(trunc);
    }
}
