//! End-to-end tests that exercise the public API the way a downstream
//! consumer of the crate would, going through the published surface only.

use agent_health_check::{HealthRegistry, Status, Summary};

#[test]
fn typical_agent_lifecycle() {
    let mut reg = HealthRegistry::new();

    // Everything starts unknown until probed.
    reg.set("llm_provider", Status::Unknown);
    reg.set("vector_store", Status::Unknown);
    reg.set("tool_runner", Status::Unknown);
    assert!(!reg.is_healthy());
    assert_eq!(reg.overall(), Status::Unknown);
    assert_eq!(
        reg.unknown_components(),
        vec!["llm_provider", "tool_runner", "vector_store"]
    );

    // Probes come back healthy.
    reg.set("llm_provider", Status::Ok);
    reg.set("vector_store", Status::Ok);
    reg.set("tool_runner", Status::Ok);
    assert!(reg.all_ok());
    assert!(reg.is_healthy());
    assert_eq!(reg.overall(), Status::Ok);

    // The vector store slows down.
    reg.set("vector_store", Status::Degraded("p99 latency high".into()));
    assert!(!reg.all_ok());
    assert!(reg.is_healthy()); // degraded is still "healthy enough"
    assert_eq!(reg.degraded_components(), vec!["vector_store"]);
    assert_eq!(reg.overall(), Status::Degraded("p99 latency high".into()));

    // The LLM provider goes down: now overall is the worst (Down).
    reg.set("llm_provider", Status::Down("503 from upstream".into()));
    assert!(!reg.is_healthy());
    assert_eq!(reg.down_components(), vec!["llm_provider"]);
    assert_eq!(reg.overall(), Status::Down("503 from upstream".into()));
}

#[test]
fn summary_roundtrip() {
    let mut reg = HealthRegistry::new();
    reg.set("a", Status::Ok);
    reg.set("b", Status::Ok);
    reg.set("c", Status::Degraded("slow".into()));
    reg.set("d", Status::Down("offline".into()));

    let summary: Summary = reg.summary();
    assert_eq!(summary.total(), reg.len());
    assert_eq!(summary.ok, 2);
    assert_eq!(summary.degraded, 1);
    assert_eq!(summary.down, 1);
    assert_eq!(summary.unknown, 0);
    assert_eq!(summary.to_string(), "2 ok, 1 degraded, 1 down, 0 unknown");
}

#[test]
fn remove_and_reset() {
    let mut reg = HealthRegistry::new();
    reg.set("a", Status::Ok);
    reg.set("b", Status::Down("x".into()));

    assert!(reg.contains("b"));
    reg.remove("b");
    assert!(!reg.contains("b"));
    assert_eq!(reg.overall(), Status::Ok);

    reg.reset_all();
    assert_eq!(reg.get("a"), Some(&Status::Unknown));
    assert_eq!(reg.overall(), Status::Unknown);
}

#[test]
fn empty_registry_is_vacuously_ok() {
    let reg = HealthRegistry::new();
    assert!(reg.is_empty());
    assert!(reg.all_ok());
    assert!(reg.is_healthy());
    assert_eq!(reg.overall(), Status::Ok);
    assert_eq!(reg.summary().total(), 0);
}
