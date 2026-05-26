//! 请求日志环形缓冲区
//!
//! 内存中保存最近 N 条 HTTP 请求的完整记录（method/headers/body/status/error/duration）。
//! 容量可在运行时调整。重启清空。线程安全。

use std::collections::VecDeque;
use std::sync::RwLock;

use serde::Serialize;

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
    pub req_body_binary: bool,

    pub upstream_url: String,
    pub upstream_headers: Vec<HeaderEntry>,
    pub upstream_body_b64: String,
    pub upstream_body_len: usize,

    pub status_code: Option<u16>,
    pub resp_headers: Vec<HeaderEntry>,
    pub upstream_resp_headers: Vec<HeaderEntry>,
    pub resp_body_b64: String,
    pub resp_body_len: usize,
    pub resp_body_binary: bool,
    pub resp_content_type: Option<String>,
    pub upstream_resp_body_b64: String,
    pub upstream_resp_body_len: usize,

    pub duration_ms: u64,
    pub error: Option<String>,
}

struct Inner {
    buf: VecDeque<RequestLog>,
    capacity: usize,
}

pub struct LogStore {
    inner: RwLock<Inner>,
}

impl LogStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: RwLock::new(Inner {
                buf: VecDeque::with_capacity(capacity.max(1)),
                capacity: capacity.max(1),
            }),
        }
    }

    pub fn set_capacity(&self, new_capacity: usize) {
        let cap = new_capacity.max(1);
        if let Ok(mut g) = self.inner.write() {
            g.capacity = cap;
            while g.buf.len() > cap {
                g.buf.pop_front();
            }
        }
    }

    pub fn push(&self, log: RequestLog) {
        if let Ok(mut g) = self.inner.write() {
            while g.buf.len() >= g.capacity {
                g.buf.pop_front();
            }
            g.buf.push_back(log);
        }
    }

    pub fn list(&self, endpoint_id: Option<&str>, limit: Option<usize>) -> Vec<RequestLog> {
        let g = match self.inner.read() {
            Ok(g) => g,
            Err(_) => return vec![],
        };
        let iter = g
            .buf
            .iter()
            .rev()
            .filter(|l| endpoint_id.map(|id| l.endpoint_id == id).unwrap_or(true));
        match limit {
            Some(n) => iter.take(n).cloned().collect(),
            None => iter.cloned().collect(),
        }
    }

    pub fn get(&self, id: &str) -> Option<RequestLog> {
        let g = self.inner.read().ok()?;
        g.buf.iter().find(|l| l.id == id).cloned()
    }

    pub fn clear(&self, endpoint_id: Option<&str>) {
        if let Ok(mut g) = self.inner.write() {
            match endpoint_id {
                None => g.buf.clear(),
                Some(id) => g.buf.retain(|l| l.endpoint_id != id),
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

/// 将字节数组完整 base64 编码。返回 (b64, len)。不截断。
pub fn encode_body(bytes: &[u8]) -> (String, usize) {
    use base64::{engine::general_purpose, Engine};
    (general_purpose::STANDARD.encode(bytes), bytes.len())
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
            req_body_binary: false,
            upstream_url: "https://example.com/x".into(),
            upstream_headers: vec![],
            upstream_body_b64: "".into(),
            upstream_body_len: 0,
            status_code: Some(200),
            resp_headers: vec![],
            upstream_resp_headers: vec![],
            resp_body_b64: "".into(),
            resp_body_len: 0,
            resp_body_binary: false,
            resp_content_type: None,
            upstream_resp_body_b64: "".into(),
            upstream_resp_body_len: 0,
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
    fn set_capacity_shrinks_buffer() {
        let store = LogStore::new(5);
        for i in 0..5 {
            store.push(sample(&i.to_string(), "ep1"));
        }
        store.set_capacity(2);
        let all = store.list(None, None);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, "4");
        assert_eq!(all[1].id, "3");
    }

    #[test]
    fn set_capacity_grows_buffer() {
        let store = LogStore::new(2);
        for i in 0..5 {
            store.push(sample(&i.to_string(), "ep1"));
        }
        store.set_capacity(10);
        for i in 5..10 {
            store.push(sample(&i.to_string(), "ep1"));
        }
        let all = store.list(None, None);
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn binary_detector() {
        assert!(is_binary_content_type("image/png"));
        assert!(is_binary_content_type("APPLICATION/Octet-Stream; charset=x"));
        assert!(!is_binary_content_type("application/json"));
        assert!(!is_binary_content_type("text/event-stream"));
    }

    #[test]
    fn encode_body_passes_full() {
        let big = vec![b'x'; 10 * 1024 * 1024];
        let (_b64, len) = encode_body(&big);
        assert_eq!(len, 10 * 1024 * 1024);
    }
}
