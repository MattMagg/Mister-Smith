// Phase 3 Integration Test Suite
// Comprehensive tests for multi-agent orchestration

mod multi_agent_spawn_tests;
mod supervision_strategy_tests;
mod failure_recovery_tests;
mod persistence_integration_tests;
mod nats_events_tests;
mod task_routing_tests;
mod phase2_compatibility_tests;
mod performance_tests;
mod chaos_tests;

// Test utilities
mod test_fixtures;
mod test_builders;
mod chaos_injector;
mod event_recorder;

use mistersmith::supervision::SupervisionTree;
use mistersmith::agent::AgentPool;
use testcontainers::{clients::Cli, images::postgres::Postgres, images::generic::GenericImage};

/// Standard test environment setup
pub async fn setup_test_env() -> TestEnvironment {
    // Initialize logging for tests
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mistersmith=debug,test=debug")
        .try_init();

    // Start test containers
    let docker = Cli::default();
    let postgres_container = docker.run(Postgres::default());
    let nats_container = docker.run(GenericImage::new("nats", "latest")
        .with_exposed_port(4222));

    // Get connection URLs
    let database_url = format!("postgres://postgres:postgres@localhost:{}/postgres",
        postgres_container.get_host_port_ipv4(5432));
    let nats_url = format!("nats://localhost:{}",
        nats_container.get_host_port_ipv4(4222));

    TestEnvironment {
        database_url,
        nats_url,
        _postgres: postgres_container,
        _nats: nats_container,
    }
}

pub struct TestEnvironment {
    pub database_url: String,
    pub nats_url: String,
    _postgres: testcontainers::Container<'static, Postgres>,
    _nats: testcontainers::Container<'static, GenericImage>,
}

/// Common test assertions
pub mod assertions {
    use std::time::Duration;
    use tokio::time::timeout;

    pub async fn assert_within_timeout<F, T>(duration: Duration, future: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        timeout(duration, future)
            .await
            .expect("Operation timed out")
    }

    pub fn assert_eventually<F>(condition: F, max_attempts: u32, delay: Duration)
    where
        F: Fn() -> bool,
    {
        for _ in 0..max_attempts {
            if condition() {
                return;
            }
            std::thread::sleep(delay);
        }
        panic!("Condition not met within {} attempts", max_attempts);
    }
}