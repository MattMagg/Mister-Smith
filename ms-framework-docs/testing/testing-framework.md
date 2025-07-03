# MISTER SMITH TESTING FRAMEWORK

**Agent-Focused Testing Strategy for Multi-Agent AI Framework**

## TESTING PHILOSOPHY

### Core Principles
- **Comprehensive Coverage**: 90%+ line coverage for core modules, 100% for security-critical components
- **Agent-Centric Testing**: All tests designed for AI agent scenarios and workflows
- **Async-First Architecture**: Native support for asynchronous agent operations
- **Mock-Driven Development**: Isolated testing with comprehensive mock frameworks
- **Performance-Aware**: Continuous benchmarking and performance regression detection
- **Security-Validated**: Mandatory security testing for all authentication and authorization flows

### Testing Hierarchy
```
P0: Critical Path Tests (agent lifecycle, security, data integrity)
P1: Core Functionality Tests (communication, persistence, configuration)
P2: Edge Case and Error Handling Tests
P3: Performance and Scalability Tests
P4: Compatibility and Regression Tests
```

## UNIT TEST PATTERNS

### Agent Testing Template
```rust
use tokio_test;
use mockall::predicate::*;
use crate::agents::{Agent, AgentConfig, AgentStatus};
use crate::testing::{MockMessagingService, TestAgentBuilder};

#[tokio::test]
async fn test_agent_lifecycle() {
    // Arrange
    let mut mock_messaging = MockMessagingService::new();
    mock_messaging
        .expect_send_message()
        .times(1)
        .returning(|_| Ok(()));
    
    let agent = TestAgentBuilder::new()
        .with_id("test-agent-001")
        .with_messaging_service(mock_messaging)
        .build();
    
    // Act
    let result = agent.start().await;
    
    // Assert
    assert!(result.is_ok());
    assert_eq!(agent.status(), AgentStatus::Running);
}

#[test]
fn test_agent_configuration_validation() {
    // Property-based testing for agent configurations
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn agent_config_roundtrip(
            id in "[a-zA-Z0-9-]{1,50}",
            max_memory in 1..1000u64,
            timeout in 1..3600u32
        ) {
            let config = AgentConfig {
                id: id.clone(),
                max_memory_mb: max_memory,
                timeout_seconds: timeout,
            };
            
            let serialized = serde_json::to_string(&config)?;
            let deserialized: AgentConfig = serde_json::from_str(&serialized)?;
            
            assert_eq!(config.id, deserialized.id);
            assert_eq!(config.max_memory_mb, deserialized.max_memory_mb);
            assert_eq!(config.timeout_seconds, deserialized.timeout_seconds);
        }
    }
}
```

### Data Validation Testing Pattern
```rust
use crate::data::{MessageSchema, ValidationResult};
use crate::testing::TestDataGenerator;

#[test]
fn test_message_schema_validation() {
    let generator = TestDataGenerator::new();
    
    // Valid message test
    let valid_message = generator.create_valid_message();
    assert!(MessageSchema::validate(&valid_message).is_valid());
    
    // Invalid message tests
    let invalid_messages = generator.create_invalid_messages();
    for message in invalid_messages {
        let result = MessageSchema::validate(&message);
        assert!(!result.is_valid());
        assert!(!result.errors.is_empty());
    }
}

#[test]
fn test_data_persistence_integrity() {
    use tempfile::TempDir;
    
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    
    let repository = TestRepository::new(&db_path);
    let test_data = TestDataGenerator::new().create_agent_data();
    
    // Test write-read cycle
    repository.save(&test_data).unwrap();
    let retrieved_data = repository.load(&test_data.id).unwrap();
    
    assert_eq!(test_data, retrieved_data);
}
```

### Security Testing Pattern
```rust
use crate::security::{AuthenticationService, AuthorizationPolicy};
use crate::testing::{MockTokenProvider, TestSecurityContext};

#[tokio::test]
async fn test_authentication_flow() {
    let mut mock_token_provider = MockTokenProvider::new();
    mock_token_provider
        .expect_validate_token()
        .with(eq("valid-token"))
        .returning(|_| Ok(true));
    
    let auth_service = AuthenticationService::new(mock_token_provider);
    let result = auth_service.authenticate("valid-token").await;
    
    assert!(result.is_ok());
    assert!(result.unwrap().is_authenticated);
}

#[test]
fn test_authorization_policies() {
    let policy = AuthorizationPolicy::from_rules(vec![
        "agent:read:*",
        "agent:write:own",
        "admin:*:*",
    ]);
    
    // Test agent read access
    assert!(policy.check("agent", "read", "any-resource"));
    
    // Test agent write access (own resources only)
    assert!(policy.check("agent", "write", "agent-123"));
    assert!(!policy.check("agent", "write", "other-agent"));
    
    // Test admin access
    assert!(policy.check("admin", "delete", "any-resource"));
}
```

## INTEGRATION TEST SPECIFICATIONS

### Multi-Agent Communication Testing
```rust
use testcontainers::{clients::Cli, images::generic::GenericImage};
use crate::transport::NatsMessagingService;
use crate::agents::{AgentOrchestrator, AgentType};

#[tokio::test]
async fn test_multi_agent_coordination() {
    // Start NATS test container
    let docker = Cli::default();
    let nats_container = docker.run(
        GenericImage::new("nats", "latest")
            .with_exposed_port(4222)
    );
    
    let nats_url = format!(
        "nats://localhost:{}", 
        nats_container.get_host_port_ipv4(4222)
    );
    
    // Setup orchestrator with test agents
    let messaging = NatsMessagingService::new(&nats_url).await.unwrap();
    let mut orchestrator = AgentOrchestrator::new(messaging);
    
    // Spawn test agents
    let analyst_id = orchestrator.spawn_agent(AgentType::Analyst).await.unwrap();
    let architect_id = orchestrator.spawn_agent(AgentType::Architect).await.unwrap();
    
    // Test communication flow
    let task = TestTask::new("analyze-system-requirements");
    let result = orchestrator.execute_workflow(task).await.unwrap();
    
    assert!(result.is_complete());
    assert_eq!(result.participating_agents().len(), 2);
}
```

### Database Integration Testing
```rust
use sqlx::{PgPool, Postgres, migrate::MigrateDatabase};
use crate::data::{DatabaseManager, MigrationRunner};

#[tokio::test]
async fn test_database_migrations() {
    let db_url = "postgres://test:test@localhost/test_db";
    
    // Create test database
    if !Postgres::database_exists(db_url).await.unwrap_or(false) {
        Postgres::create_database(db_url).await.unwrap();
    }
    
    let pool = PgPool::connect(db_url).await.unwrap();
    let migration_runner = MigrationRunner::new(&pool);
    
    // Run migrations
    migration_runner.run_all().await.unwrap();
    
    // Verify schema
    let schema_version = migration_runner.current_version().await.unwrap();
    assert!(schema_version > 0);
    
    // Test data operations
    let db_manager = DatabaseManager::new(pool);
    let test_agent = TestAgentBuilder::new().build();
    
    db_manager.save_agent(&test_agent).await.unwrap();
    let retrieved = db_manager.load_agent(&test_agent.id).await.unwrap();
    
    assert_eq!(test_agent.id, retrieved.id);
}
```

### Claude CLI Integration Testing
```rust
use crate::claude::{ClaudeCliService, ClaudeCommand};
use crate::testing::{MockClaudeProcess, TestTaskBuilder};

#[tokio::test]
async fn test_claude_cli_integration() {
    let mut mock_process = MockClaudeProcess::new();
    mock_process
        .expect_execute()
        .returning(|cmd| Ok(format!("Executed: {}", cmd)));
    
    let claude_service = ClaudeCliService::new(mock_process);
    let task = TestTaskBuilder::new()
        .with_type("code-analysis")
        .with_files(vec!["src/main.rs"])
        .build();
    
    let result = claude_service.execute_task(task).await.unwrap();
    
    assert!(result.is_success());
    assert!(!result.output().is_empty());
}
```

## MOCK OBJECT PATTERNS

### Messaging Service Mock
```rust
use mockall::automock;
use async_trait::async_trait;

#[automock]
#[async_trait]
pub trait MessagingService {
    async fn send_message(&self, topic: &str, message: &[u8]) -> Result<(), MessagingError>;
    async fn subscribe(&self, topic: &str) -> Result<MessageStream, MessagingError>;
    async fn unsubscribe(&self, subscription_id: &str) -> Result<(), MessagingError>;
}

// Usage in tests
let mut mock = MockMessagingService::new();
mock.expect_send_message()
    .with(eq("agent.tasks"), always())
    .times(1)
    .returning(|_, _| Ok(()));
```

### Database Repository Mock
```rust
#[automock]
#[async_trait]
pub trait AgentRepository {
    async fn save(&self, agent: &Agent) -> Result<(), RepositoryError>;
    async fn load(&self, id: &str) -> Result<Option<Agent>, RepositoryError>;
    async fn list_active(&self) -> Result<Vec<Agent>, RepositoryError>;
    async fn delete(&self, id: &str) -> Result<(), RepositoryError>;
}
```

### External Service Mock
```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, body_json};

#[tokio::test]
async fn test_external_api_integration() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/api/analyze"))
        .and(body_json(serde_json::json!({
            "type": "code-analysis",
            "files": ["src/main.rs"]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            serde_json::json!({
                "status": "success",
                "results": ["Analysis complete"]
            })
        ))
        .mount(&mock_server)
        .await;
    
    let client = ExternalApiClient::new(&mock_server.uri());
    let result = client.analyze_code(vec!["src/main.rs"]).await.unwrap();
    
    assert_eq!(result.status, "success");
}
```

## TEST DATA GENERATION

### Property-Based Test Data
```rust
use proptest::prelude::*;

pub struct TestDataGenerator;

impl TestDataGenerator {
    pub fn agent_id_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9-]{8,32}"
    }
    
    pub fn agent_config_strategy() -> impl Strategy<Value = AgentConfig> {
        (
            Self::agent_id_strategy(),
            1..1000u64,
            1..3600u32,
        ).prop_map(|(id, memory, timeout)| AgentConfig {
            id,
            max_memory_mb: memory,
            timeout_seconds: timeout,
        })
    }
    
    pub fn message_strategy() -> impl Strategy<Value = Message> {
        (
            Self::agent_id_strategy(),
            Self::agent_id_strategy(),
            any::<MessageType>(),
            any::<Vec<u8>>(),
        ).prop_map(|(sender, receiver, msg_type, payload)| Message {
            sender,
            receiver,
            message_type: msg_type,
            payload,
            timestamp: SystemTime::now(),
        })
    }
}
```

### Test Builder Pattern
```rust
pub struct TestAgentBuilder {
    id: Option<String>,
    config: Option<AgentConfig>,
    messaging: Option<Box<dyn MessagingService>>,
}

impl TestAgentBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            config: None,
            messaging: None,
        }
    }
    
    pub fn with_id(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
    }
    
    pub fn with_config(mut self, config: AgentConfig) -> Self {
        self.config = Some(config);
        self
    }
    
    pub fn with_messaging_service(mut self, service: Box<dyn MessagingService>) -> Self {
        self.messaging = Some(service);
        self
    }
    
    pub fn build(self) -> Agent {
        let id = self.id.unwrap_or_else(|| "test-agent".to_string());
        let config = self.config.unwrap_or_default();
        let messaging = self.messaging.unwrap_or_else(|| Box::new(MockMessagingService::new()));
        
        Agent::new(id, config, messaging)
    }
}
```

## PERFORMANCE TEST SPECIFICATIONS

### Benchmarking Framework
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use crate::agents::AgentOrchestrator;

fn bench_agent_spawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent_spawn");
    
    for agent_count in [1, 10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("spawn_agents", agent_count),
            agent_count,
            |b, &count| {
                b.iter(|| {
                    let orchestrator = AgentOrchestrator::new_test();
                    let agents = (0..count)
                        .map(|i| orchestrator.spawn_agent(AgentType::Worker))
                        .collect::<Vec<_>>();
                    black_box(agents);
                });
            },
        );
    }
    group.finish();
}

fn bench_message_throughput(c: &mut Criterion) {
    let messaging = NatsMessagingService::new_test();
    
    c.bench_function("message_throughput", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let message = format!("test-message-{}", i);
                messaging.send_message("test.topic", message.as_bytes());
            }
        });
    });
}

criterion_group!(benches, bench_agent_spawn, bench_message_throughput);
criterion_main!(benches);
```

### Load Testing Specifications
```rust
use tokio::time::{Duration, Instant};
use futures::future::join_all;

#[tokio::test]
async fn test_concurrent_agent_operations() {
    let orchestrator = AgentOrchestrator::new_test();
    let start_time = Instant::now();
    
    // Spawn 100 agents concurrently
    let spawn_tasks: Vec<_> = (0..100)
        .map(|_| orchestrator.spawn_agent(AgentType::Worker))
        .collect();
    
    let agents = join_all(spawn_tasks).await;
    let spawn_duration = start_time.elapsed();
    
    // Verify all agents spawned successfully
    assert_eq!(agents.len(), 100);
    assert!(spawn_duration < Duration::from_secs(5));
    
    // Test concurrent task execution
    let tasks: Vec<_> = agents.iter()
        .map(|agent| agent.execute_task(TestTask::new("simple-computation")))
        .collect();
    
    let results = join_all(tasks).await;
    let total_duration = start_time.elapsed();
    
    // Verify all tasks completed successfully
    assert!(results.iter().all(|r| r.is_ok()));
    assert!(total_duration < Duration::from_secs(30));
}
```

## TESTING STANDARDS

### Coverage Requirements
- **Line Coverage**: Minimum 90% for all modules
- **Branch Coverage**: 85% for conditional logic
- **Function Coverage**: 100% for public APIs
- **Integration Coverage**: All component interactions tested

### Performance Budgets
- **Unit Tests**: < 100ms execution time
- **Integration Tests**: < 5s execution time
- **End-to-End Tests**: < 30s execution time
- **Memory Usage**: < 100MB per test suite

### Quality Gates
- All tests must pass before merge
- No test flakiness tolerance (0% flaky tests)
- Performance regression detection (<5% slowdown threshold)
- Security vulnerability scanning in test dependencies

### Test Organization
```
tests/
├── unit/                    # Unit tests co-located with source
├── integration/             # Integration tests
│   ├── agents/             # Agent integration tests
│   ├── messaging/          # Messaging integration tests
│   └── database/           # Database integration tests
├── end_to_end/             # Full workflow tests
├── benchmarks/             # Performance benchmarks
├── security/               # Security-specific tests
├── fixtures/               # Test data and fixtures
└── mocks/                  # Mock implementations
```

## CI/CD INTEGRATION

### GitHub Actions Workflow
```yaml
name: Testing Pipeline

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run Unit Tests
        run: cargo test --lib --bins
      - name: Generate Coverage
        run: |
          cargo install grcov
          cargo test --no-fail-fast
          grcov . --binary-path ./target/debug/ -s . -t lcov --branch --ignore-not-existing -o lcov.info

  integration-tests:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:13
        env:
          POSTGRES_PASSWORD: test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      nats:
        image: nats:latest
        ports:
          - 4222:4222
    steps:
      - uses: actions/checkout@v3
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
      - name: Run Integration Tests
        run: cargo test --test integration
        env:
          DATABASE_URL: postgres://postgres:test@localhost/test
          NATS_URL: nats://localhost:4222

  security-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Security Audit
        run: |
          cargo install cargo-audit
          cargo audit
      - name: Dependency Vulnerability Scan
        run: |
          cargo install cargo-deny
          cargo deny check
```

### Test Reporting
```rust
use serde_json::json;
use std::fs::File;
use std::io::Write;

pub struct TestReporter;

impl TestReporter {
    pub fn generate_report(results: &TestResults) -> serde_json::Value {
        json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "summary": {
                "total_tests": results.total_count(),
                "passed": results.passed_count(),
                "failed": results.failed_count(),
                "skipped": results.skipped_count(),
                "coverage": results.coverage_percentage()
            },
            "categories": {
                "unit_tests": results.unit_test_summary(),
                "integration_tests": results.integration_test_summary(),
                "performance_tests": results.performance_test_summary(),
                "security_tests": results.security_test_summary()
            },
            "performance_metrics": {
                "execution_time": results.total_execution_time(),
                "memory_usage": results.peak_memory_usage(),
                "agent_spawn_time": results.agent_spawn_benchmark(),
                "message_throughput": results.message_throughput_benchmark()
            }
        })
    }
}
```

---

*Mister Smith Testing Framework - Comprehensive testing strategy for multi-agent AI systems*
*Agent-focused, performance-aware, security-validated testing approach*