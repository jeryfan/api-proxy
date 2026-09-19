//! 上游选择与熔断健康管理。
//!
//! 选择语义：
//! - 仅 enabled 且未熔断（!is_tripped）的上游参与选择与失败转移；
//! - 按权重展开成列表，端点级游标循环推进；
//! - 失败转移通过 offset 从游标后第 offset 个继续，不额外推进游标；
//! - 配置热更新（replace_routes）不重置游标，重启清空；
//! - 当某上游连续失败达到阈值时自动标记为熔断失效（is_tripped），排除出可用上游池，需手动恢复。

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::config::{Endpoint, Upstream};

pub type UpstreamCursors = Arc<Mutex<HashMap<String, usize>>>;

pub fn new_cursors() -> UpstreamCursors {
    Arc::new(Mutex::new(HashMap::new()))
}

/// 上游实时运行时健康状态。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamHealthState {
    pub endpoint_id: String,
    pub upstream_id: String,
    pub consecutive_failures: u32,
    pub is_tripped: bool,
    pub last_failure_at: Option<i64>,
    pub last_failure_reason: Option<String>,
}

impl UpstreamHealthState {
    pub fn new(endpoint_id: String, upstream_id: String) -> Self {
        Self {
            endpoint_id,
            upstream_id,
            consecutive_failures: 0,
            is_tripped: false,
            last_failure_at: None,
            last_failure_reason: None,
        }
    }
}

/// 上游健康追踪器（并发读写）。
#[derive(Clone, Default)]
pub struct UpstreamHealthTracker {
    inner: Arc<RwLock<HashMap<(String, String), UpstreamHealthState>>>,
}

impl UpstreamHealthTracker {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get(&self, endpoint_id: &str, upstream_id: &str) -> UpstreamHealthState {
        let key = (endpoint_id.to_string(), upstream_id.to_string());
        let map = self.inner.read().unwrap();
        map.get(&key)
            .cloned()
            .unwrap_or_else(|| UpstreamHealthState::new(endpoint_id.to_string(), upstream_id.to_string()))
    }

    pub fn is_tripped(&self, endpoint_id: &str, upstream_id: &str) -> bool {
        let key = (endpoint_id.to_string(), upstream_id.to_string());
        let map = self.inner.read().unwrap();
        map.get(&key).map(|s| s.is_tripped).unwrap_or(false)
    }

    /// 记录一次失败。返回 `(当前状态, 是否刚刚触发了新熔断)`。
    pub fn record_failure(
        &self,
        endpoint_id: &str,
        upstream_id: &str,
        threshold: u32,
        reason: String,
    ) -> (UpstreamHealthState, bool) {
        let key = (endpoint_id.to_string(), upstream_id.to_string());
        let mut map = self.inner.write().unwrap();
        let state = map
            .entry(key)
            .or_insert_with(|| UpstreamHealthState::new(endpoint_id.to_string(), upstream_id.to_string()));

        state.consecutive_failures += 1;
        state.last_failure_at = Some(Utc::now().timestamp_millis());
        state.last_failure_reason = Some(reason);

        let mut newly_tripped = false;
        if !state.is_tripped && state.consecutive_failures >= threshold.max(1) {
            state.is_tripped = true;
            newly_tripped = true;
        }

        (state.clone(), newly_tripped)
    }

    /// 记录一次成功。如果之前有失败计数且尚未熔断，则清零。
    pub fn record_success(
        &self,
        endpoint_id: &str,
        upstream_id: &str,
    ) -> Option<UpstreamHealthState> {
        let key = (endpoint_id.to_string(), upstream_id.to_string());
        let mut map = self.inner.write().unwrap();
        if let Some(state) = map.get_mut(&key) {
            if state.consecutive_failures > 0 && !state.is_tripped {
                state.consecutive_failures = 0;
                return Some(state.clone());
            }
        }
        None
    }

    /// 手动重置/恢复指定上游的健康与熔断状态。
    pub fn reset(&self, endpoint_id: &str, upstream_id: &str) -> UpstreamHealthState {
        let key = (endpoint_id.to_string(), upstream_id.to_string());
        let mut map = self.inner.write().unwrap();
        let state = map
            .entry(key)
            .or_insert_with(|| UpstreamHealthState::new(endpoint_id.to_string(), upstream_id.to_string()));
        state.consecutive_failures = 0;
        state.is_tripped = false;
        state.last_failure_reason = None;
        state.clone()
    }

    /// 重置某个端点下所有上游的健康状态。
    pub fn reset_endpoint(&self, endpoint_id: &str) -> Vec<UpstreamHealthState> {
        let mut map = self.inner.write().unwrap();
        let mut reset_list = Vec::new();
        for ((ep_id, _), state) in map.iter_mut() {
            if ep_id == endpoint_id {
                state.consecutive_failures = 0;
                state.is_tripped = false;
                state.last_failure_reason = None;
                reset_list.push(state.clone());
            }
        }
        reset_list
    }

    /// 获取所有上游健康状态快照，以 `endpoint_id:upstream_id` 为 key。
    pub fn all_states(&self) -> HashMap<String, UpstreamHealthState> {
        let map = self.inner.read().unwrap();
        map.iter()
            .map(|((ep_id, up_id), s)| (format!("{ep_id}:{up_id}"), s.clone()))
            .collect()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PlanError {
    NoEnabledUpstream,
    AllUpstreamsTripped,
}

/// 生成一次请求的上游计划：
/// 1. 过滤掉静态未启用（!enabled）或已熔断（is_tripped）的上游；
/// 2. 加权轮询决定起始上游（并推进游标）；
/// 3. 返回以起始上游开头、按展开顺序去重后的全部可用上游。
pub fn plan<'a>(
    endpoint: &'a Endpoint,
    cursors: &UpstreamCursors,
    health_tracker: &UpstreamHealthTracker,
) -> Result<Vec<&'a Upstream>, PlanError> {
    let enabled_list: Vec<&Upstream> = endpoint.upstreams.iter().filter(|u| u.enabled).collect();
    if enabled_list.is_empty() {
        return Err(PlanError::NoEnabledUpstream);
    }

    // 过滤排除已熔断失效的上游
    let available: Vec<&Upstream> = enabled_list
        .into_iter()
        .filter(|u| {
            if !u.health.enabled {
                true
            } else {
                !health_tracker.is_tripped(&endpoint.id, &u.id)
            }
        })
        .collect();

    if available.is_empty() {
        return Err(PlanError::AllUpstreamsTripped);
    }

    let total: usize = available.iter().map(|u| u.weight.max(1) as usize).sum();
    let mut expanded = Vec::with_capacity(total);
    for u in &available {
        for _ in 0..u.weight.max(1) {
            expanded.push(*u);
        }
    }

    let mut map = cursors.lock().map_err(|_| PlanError::NoEnabledUpstream)?;
    let cursor = map.entry(endpoint.id.clone()).or_insert(0);
    let start_expanded = *cursor % expanded.len();
    *cursor = (*cursor + 1) % 1_000_000_000;

    let mut distinct: Vec<&Upstream> = Vec::new();
    for u in &expanded {
        if !distinct.iter().any(|d| d.id == u.id) {
            distinct.push(u);
        }
    }
    // 起始上游 = 展开列表游标位置（权重决定概率），映射到去重列表的下标做旋转。
    let first_id = expanded[start_expanded].id.as_str();
    let start = distinct
        .iter()
        .position(|u| u.id == first_id)
        .unwrap_or(0);
    let n = distinct.len();
    Ok((0..n).map(|i| distinct[(start + i) % n]).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn upstream(id: &str, weight: u32) -> Upstream {
        Upstream {
            id: id.into(),
            name: String::new(),
            url: format!("https://{id}.example.com"),
            enabled: true,
            weight,
            header_rules: vec![],
            health: crate::config::HealthConfig::default(),
        }
    }

    fn endpoint(upstreams: Vec<Upstream>) -> Endpoint {
        Endpoint {
            id: "ep".into(),
            name: "ep".into(),
            description: "".into(),
            enabled: true,
            path: "/cc".into(),
            upstreams,
            strip_prefix: true,
            fixed_upstream: false,
            header_rules: vec![],
            query_rules: vec![],
            sort_index: 0,
            created_at: 0,
        }
    }

    fn heads(e: &Endpoint, c: &UpstreamCursors, h: &UpstreamHealthTracker, n: usize) -> Vec<String> {
        (0..n)
            .map(|_| plan(e, c, h).unwrap().first().unwrap().id.clone())
            .collect()
    }

    #[test]
    fn plan_none_when_all_disabled() {
        let e = endpoint(vec![{
            let mut u = upstream("a", 1);
            u.enabled = false;
            u
        }]);
        assert_eq!(
            plan(&e, &new_cursors(), &UpstreamHealthTracker::new()),
            Err(PlanError::NoEnabledUpstream)
        );
    }

    #[test]
    fn plan_error_when_all_tripped() {
        let mut u1 = upstream("a", 1);
        u1.health.enabled = true;
        let mut u2 = upstream("b", 1);
        u2.health.enabled = true;
        let e = endpoint(vec![u1, u2]);
        let tracker = UpstreamHealthTracker::new();
        tracker.record_failure("ep", "a", 1, "test err".into());
        tracker.record_failure("ep", "b", 1, "test err".into());

        assert_eq!(
            plan(&e, &new_cursors(), &tracker),
            Err(PlanError::AllUpstreamsTripped)
        );
    }

    #[test]
    fn round_robin_alternates_equal_weights() {
        let e = endpoint(vec![upstream("a", 1), upstream("b", 1)]);
        let tracker = UpstreamHealthTracker::new();
        assert_eq!(heads(&e, &new_cursors(), &tracker, 4), ["a", "b", "a", "b"]);
    }

    #[test]
    fn weighted_distribution() {
        let e = endpoint(vec![upstream("a", 3), upstream("b", 1)]);
        let tracker = UpstreamHealthTracker::new();
        assert_eq!(
            heads(&e, &new_cursors(), &tracker, 8),
            ["a", "a", "a", "b", "a", "a", "a", "b"]
        );
    }

    #[test]
    fn plan_rotates_without_duplicate() {
        let e = endpoint(vec![upstream("a", 1), upstream("b", 1)]);
        let c = new_cursors();
        let tracker = UpstreamHealthTracker::new();
        // 第一次请求起始 a：转移顺序 a → b，无重复。
        let p = plan(&e, &c, &tracker).unwrap();
        assert_eq!(p.iter().map(|u| u.id.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        // 第二次请求起始 b：转移顺序 b → a。
        let p = plan(&e, &c, &tracker).unwrap();
        assert_eq!(p.iter().map(|u| u.id.as_str()).collect::<Vec<_>>(), ["b", "a"]);
    }

    #[test]
    fn disabled_upstream_not_selected() {
        let mut b = upstream("b", 1);
        b.enabled = false;
        let e = endpoint(vec![upstream("a", 1), b]);
        let c = new_cursors();
        let tracker = UpstreamHealthTracker::new();
        for _ in 0..4 {
            let p = plan(&e, &c, &tracker).unwrap();
            assert_eq!(p.len(), 1);
            assert_eq!(p[0].id, "a");
        }
    }

    #[test]
    fn tripped_upstream_excluded_and_recovered() {
        let mut a = upstream("a", 1);
        a.health.enabled = true;
        let mut b = upstream("b", 1);
        b.health.enabled = true;
        let e = endpoint(vec![a, b]);
        let c = new_cursors();
        let tracker = UpstreamHealthTracker::new();

        // 连续失败 2 次，达到阈值 2
        let (_, tripped) = tracker.record_failure("ep", "a", 2, "timeout".into());
        assert!(!tripped);
        let (_, tripped2) = tracker.record_failure("ep", "a", 2, "timeout".into());
        assert!(tripped2);

        // 现在 a 已熔断，候选池中只剩 b
        let p = plan(&e, &c, &tracker).unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].id, "b");

        // 手动恢复 a
        tracker.reset("ep", "a");
        let p2 = plan(&e, &c, &tracker).unwrap();
        assert_eq!(p2.len(), 2);
    }
}
