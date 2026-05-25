/*!
agent-health-check: track health status of LLM agent components.

```rust
use agent_health_check::{HealthRegistry, Status};

let mut reg = HealthRegistry::new();
reg.set("llm_provider", Status::Ok);
reg.set("database", Status::Degraded("slow queries".into()));
assert!(reg.all_ok());  // false — database is degraded
assert!(!reg.is_healthy());
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
    pub fn is_ok(&self) -> bool { matches!(self, Status::Ok) }
    pub fn is_degraded(&self) -> bool { matches!(self, Status::Degraded(_)) }
    pub fn is_down(&self) -> bool { matches!(self, Status::Down(_)) }
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

/// Tracks health of multiple named components.
#[derive(Debug, Default)]
pub struct HealthRegistry {
    components: HashMap<String, Status>,
}

impl HealthRegistry {
    pub fn new() -> Self { Self::default() }

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
        self.components.values().all(|s| !s.is_down() && !matches!(s, Status::Unknown))
    }

    /// Names of all Down components.
    pub fn down_components(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.components.iter()
            .filter(|(_, s)| s.is_down())
            .map(|(n, _)| n.as_str())
            .collect();
        v.sort();
        v
    }

    /// Names of all Degraded components.
    pub fn degraded_components(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.components.iter()
            .filter(|(_, s)| s.is_degraded())
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

    pub fn len(&self) -> usize { self.components.len() }
    pub fn is_empty(&self) -> bool { self.components.is_empty() }

    /// Remove a component.
    pub fn remove(&mut self, name: &str) { self.components.remove(name); }

    /// Reset all to Unknown.
    pub fn reset_all(&mut self) {
        for v in self.components.values_mut() { *v = Status::Unknown; }
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
}
