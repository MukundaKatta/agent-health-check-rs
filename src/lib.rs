/*!
agent-health-check: track health status of LLM agent components.

A tiny, dependency-free registry for tracking the health of the moving parts
of an LLM agent (model providers, vector stores, tool backends, queues, ...).
Each component is assigned a [`Status`] and the [`HealthRegistry`] aggregates
them into overall verdicts such as [`HealthRegistry::all_ok`],
[`HealthRegistry::is_healthy`], and [`HealthRegistry::overall`].

```rust
use agent_health_check::{HealthRegistry, Status};

let mut reg = HealthRegistry::new();
reg.set("llm_provider", Status::Ok);
reg.set("database", Status::Degraded("slow queries".into()));

// `database` is degraded, so not every component is `Ok`.
assert!(!reg.all_ok());

// But nothing is `Down` or `Unknown`, so the agent is still considered healthy.
assert!(reg.is_healthy());

// The worst single status drives the overall verdict.
assert_eq!(reg.overall(), Status::Degraded("slow queries".into()));
```
*/

use std::collections::HashMap;
use std::fmt;

/// Health status of a component.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Ok,
    Degraded(String),
    Down(String),
    Unknown,
}

impl Status {
    /// True if this is [`Status::Ok`].
    pub fn is_ok(&self) -> bool {
        matches!(self, Status::Ok)
    }
    /// True if this is [`Status::Degraded`].
    pub fn is_degraded(&self) -> bool {
        matches!(self, Status::Degraded(_))
    }
    /// True if this is [`Status::Down`].
    pub fn is_down(&self) -> bool {
        matches!(self, Status::Down(_))
    }
    /// True if this is [`Status::Unknown`].
    pub fn is_unknown(&self) -> bool {
        matches!(self, Status::Unknown)
    }

    /// Relative severity of the status, where a higher number is worse.
    ///
    /// `Ok` (0) < `Unknown` (1) < `Degraded` (2) < `Down` (3). This ordering
    /// powers [`HealthRegistry::overall`], which reports the single worst
    /// status across all registered components.
    ///
    /// ```
    /// use agent_health_check::Status;
    /// assert!(Status::Ok.severity() < Status::Unknown.severity());
    /// assert!(Status::Unknown.severity() < Status::Degraded(String::new()).severity());
    /// assert!(Status::Degraded(String::new()).severity() < Status::Down(String::new()).severity());
    /// ```
    pub fn severity(&self) -> u8 {
        match self {
            Status::Ok => 0,
            Status::Unknown => 1,
            Status::Degraded(_) => 2,
            Status::Down(_) => 3,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Ok => write!(f, "ok"),
            Status::Degraded(msg) => write!(f, "degraded: {}", msg),
            Status::Down(msg) => write!(f, "down: {}", msg),
            Status::Unknown => write!(f, "unknown"),
        }
    }
}

/// A count of components grouped by their [`Status`] variant.
///
/// Produced by [`HealthRegistry::summary`]. Useful for emitting metrics or a
/// one-line dashboard such as `"3 ok, 1 degraded, 0 down, 0 unknown"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Summary {
    pub ok: usize,
    pub degraded: usize,
    pub down: usize,
    pub unknown: usize,
}

impl Summary {
    /// Total number of components counted.
    pub fn total(&self) -> usize {
        self.ok + self.degraded + self.down + self.unknown
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ok, {} degraded, {} down, {} unknown",
            self.ok, self.degraded, self.down, self.unknown
        )
    }
}

/// Tracks health of multiple named components.
#[derive(Debug, Default)]
pub struct HealthRegistry {
    components: HashMap<String, Status>,
}

impl HealthRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set status for a component.
    pub fn set(&mut self, name: &str, status: Status) {
        self.components.insert(name.to_string(), status);
    }

    /// Get status for a component.
    pub fn get(&self, name: &str) -> Option<&Status> {
        self.components.get(name)
    }

    /// True if all registered components are Ok.
    pub fn all_ok(&self) -> bool {
        self.components.values().all(|s| s.is_ok())
    }

    /// True if no components are Down or Unknown.
    pub fn is_healthy(&self) -> bool {
        self.components
            .values()
            .all(|s| !s.is_down() && !matches!(s, Status::Unknown))
    }

    /// Names of all Down components.
    pub fn down_components(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self
            .components
            .iter()
            .filter(|(_, s)| s.is_down())
            .map(|(n, _)| n.as_str())
            .collect();
        v.sort();
        v
    }

    /// Names of all Degraded components.
    pub fn degraded_components(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self
            .components
            .iter()
            .filter(|(_, s)| s.is_degraded())
            .map(|(n, _)| n.as_str())
            .collect();
        v.sort();
        v
    }

    /// Names of all Unknown components (sorted).
    pub fn unknown_components(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self
            .components
            .iter()
            .filter(|(_, s)| s.is_unknown())
            .map(|(n, _)| n.as_str())
            .collect();
        v.sort();
        v
    }

    /// All component names (sorted).
    pub fn components(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.components.keys().map(|s| s.as_str()).collect();
        v.sort();
        v
    }

    /// True if a component with `name` is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.components.contains_key(name)
    }

    /// Iterate over `(name, status)` pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Status)> {
        self.components.iter().map(|(n, s)| (n.as_str(), s))
    }

    /// The single worst status across all components, by [`Status::severity`].
    ///
    /// An empty registry reports [`Status::Ok`]. When multiple components share
    /// the worst severity, the returned value is one of them (the message of a
    /// `Degraded`/`Down` is taken from whichever was encountered first).
    ///
    /// ```
    /// use agent_health_check::{HealthRegistry, Status};
    /// let mut r = HealthRegistry::new();
    /// r.set("a", Status::Ok);
    /// r.set("b", Status::Degraded("slow".into()));
    /// r.set("c", Status::Down("offline".into()));
    /// assert_eq!(r.overall(), Status::Down("offline".into()));
    /// ```
    pub fn overall(&self) -> Status {
        self.components
            .values()
            .max_by_key(|s| s.severity())
            .cloned()
            .unwrap_or(Status::Ok)
    }

    /// Count components grouped by status variant. See [`Summary`].
    pub fn summary(&self) -> Summary {
        let mut s = Summary::default();
        for status in self.components.values() {
            match status {
                Status::Ok => s.ok += 1,
                Status::Degraded(_) => s.degraded += 1,
                Status::Down(_) => s.down += 1,
                Status::Unknown => s.unknown += 1,
            }
        }
        s
    }

    /// Number of registered components.
    pub fn len(&self) -> usize {
        self.components.len()
    }
    /// True if no components are registered.
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Remove a component.
    pub fn remove(&mut self, name: &str) {
        self.components.remove(name);
    }

    /// Reset all to Unknown.
    pub fn reset_all(&mut self) {
        for v in self.components.values_mut() {
            *v = Status::Unknown;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let mut r = HealthRegistry::new();
        r.set("db", Status::Ok);
        assert_eq!(r.get("db"), Some(&Status::Ok));
    }

    #[test]
    fn all_ok_true() {
        let mut r = HealthRegistry::new();
        r.set("a", Status::Ok);
        r.set("b", Status::Ok);
        assert!(r.all_ok());
    }

    #[test]
    fn all_ok_false_with_degraded() {
        let mut r = HealthRegistry::new();
        r.set("a", Status::Ok);
        r.set("b", Status::Degraded("slow".into()));
        assert!(!r.all_ok());
    }

    #[test]
    fn is_healthy_false_when_down() {
        let mut r = HealthRegistry::new();
        r.set("a", Status::Down("error".into()));
        assert!(!r.is_healthy());
    }

    #[test]
    fn is_healthy_true_with_degraded() {
        let mut r = HealthRegistry::new();
        r.set("a", Status::Degraded("slow".into()));
        assert!(r.is_healthy());
    }

    #[test]
    fn down_components_list() {
        let mut r = HealthRegistry::new();
        r.set("db", Status::Down("conn failed".into()));
        r.set("api", Status::Ok);
        assert_eq!(r.down_components(), vec!["db"]);
    }

    #[test]
    fn degraded_components_list() {
        let mut r = HealthRegistry::new();
        r.set("cache", Status::Degraded("miss rate high".into()));
        r.set("api", Status::Ok);
        assert_eq!(r.degraded_components(), vec!["cache"]);
    }

    #[test]
    fn all_ok_empty_registry() {
        let r = HealthRegistry::new();
        assert!(r.all_ok()); // vacuously true
    }

    #[test]
    fn is_healthy_false_with_unknown() {
        let mut r = HealthRegistry::new();
        r.set("x", Status::Unknown);
        assert!(!r.is_healthy());
    }

    #[test]
    fn status_display() {
        assert_eq!(Status::Ok.to_string(), "ok");
        assert!(Status::Degraded("slow".into()).to_string().contains("slow"));
        assert!(Status::Down("error".into()).to_string().contains("error"));
        assert_eq!(Status::Unknown.to_string(), "unknown");
    }

    #[test]
    fn remove_component() {
        let mut r = HealthRegistry::new();
        r.set("x", Status::Ok);
        r.remove("x");
        assert!(r.get("x").is_none());
    }

    #[test]
    fn reset_all_to_unknown() {
        let mut r = HealthRegistry::new();
        r.set("a", Status::Ok);
        r.set("b", Status::Ok);
        r.reset_all();
        assert_eq!(r.get("a"), Some(&Status::Unknown));
        assert_eq!(r.get("b"), Some(&Status::Unknown));
    }

    #[test]
    fn len_and_empty() {
        let mut r = HealthRegistry::new();
        assert!(r.is_empty());
        r.set("x", Status::Ok);
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn components_sorted() {
        let mut r = HealthRegistry::new();
        r.set("z", Status::Ok);
        r.set("a", Status::Ok);
        r.set("m", Status::Ok);
        assert_eq!(r.components(), vec!["a", "m", "z"]);
    }

    #[test]
    fn missing_key_none() {
        let r = HealthRegistry::new();
        assert_eq!(r.get("nope"), None);
    }

    #[test]
    fn severity_ordering() {
        assert!(Status::Ok.severity() < Status::Unknown.severity());
        assert!(Status::Unknown.severity() < Status::Degraded("x".into()).severity());
        assert!(Status::Degraded("x".into()).severity() < Status::Down("x".into()).severity());
    }

    #[test]
    fn is_unknown_predicate() {
        assert!(Status::Unknown.is_unknown());
        assert!(!Status::Ok.is_unknown());
    }

    #[test]
    fn overall_empty_is_ok() {
        let r = HealthRegistry::new();
        assert_eq!(r.overall(), Status::Ok);
    }

    #[test]
    fn overall_reports_worst() {
        let mut r = HealthRegistry::new();
        r.set("a", Status::Ok);
        r.set("b", Status::Degraded("slow".into()));
        r.set("c", Status::Down("offline".into()));
        assert_eq!(r.overall(), Status::Down("offline".into()));
    }

    #[test]
    fn overall_unknown_beats_ok() {
        let mut r = HealthRegistry::new();
        r.set("a", Status::Ok);
        r.set("b", Status::Unknown);
        assert_eq!(r.overall(), Status::Unknown);
    }

    #[test]
    fn summary_counts() {
        let mut r = HealthRegistry::new();
        r.set("a", Status::Ok);
        r.set("b", Status::Ok);
        r.set("c", Status::Degraded("slow".into()));
        r.set("d", Status::Down("offline".into()));
        r.set("e", Status::Unknown);
        let s = r.summary();
        assert_eq!(s.ok, 2);
        assert_eq!(s.degraded, 1);
        assert_eq!(s.down, 1);
        assert_eq!(s.unknown, 1);
        assert_eq!(s.total(), 5);
    }

    #[test]
    fn summary_display() {
        let mut r = HealthRegistry::new();
        r.set("a", Status::Ok);
        assert_eq!(
            r.summary().to_string(),
            "1 ok, 0 degraded, 0 down, 0 unknown"
        );
    }

    #[test]
    fn unknown_components_list() {
        let mut r = HealthRegistry::new();
        r.set("a", Status::Unknown);
        r.set("b", Status::Ok);
        r.set("c", Status::Unknown);
        assert_eq!(r.unknown_components(), vec!["a", "c"]);
    }

    #[test]
    fn contains_component() {
        let mut r = HealthRegistry::new();
        r.set("a", Status::Ok);
        assert!(r.contains("a"));
        assert!(!r.contains("b"));
    }

    #[test]
    fn iter_yields_all() {
        let mut r = HealthRegistry::new();
        r.set("a", Status::Ok);
        r.set("b", Status::Down("x".into()));
        let mut seen: Vec<&str> = r.iter().map(|(n, _)| n).collect();
        seen.sort();
        assert_eq!(seen, vec!["a", "b"]);
        assert_eq!(r.iter().count(), 2);
    }

    #[test]
    fn set_overwrites_existing() {
        let mut r = HealthRegistry::new();
        r.set("a", Status::Ok);
        r.set("a", Status::Down("crashed".into()));
        assert_eq!(r.get("a"), Some(&Status::Down("crashed".into())));
        assert_eq!(r.len(), 1);
    }
}
