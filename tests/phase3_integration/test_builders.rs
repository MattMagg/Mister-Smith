use mistersmith::supervision::{
    SupervisionTree, SupervisorNode, SupervisionStrategy, RestartPolicy,
    BackoffStrategy, NodeType, NodeId, ChildId
};
use mistersmith::agent::{Agent, AgentId, AgentPool, AgentConfig};
use mistersmith::transport::nats::NatsTransport;
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;

/// Builder for creating test supervision trees
pub struct TestSupervisionBuilder {
    root_strategy: SupervisionStrategy,
    restart_policy: RestartPolicy,
    children: Vec<TestChildSpec>,
}

pub struct TestChildSpec {
    pub id: String,
    pub node_type: NodeType,
    pub strategy: Option<SupervisionStrategy>,
}

impl TestSupervisionBuilder {
    pub fn new() -> Self {
        Self {
            root_strategy: SupervisionStrategy::OneForOne,
            restart_policy: RestartPolicy {
                max_restarts: 3,
                time_window: Duration::from_secs(60),
                backoff_strategy: BackoffStrategy::Exponential {
                    initial: Duration::from_millis(100),
                    max: Duration::from_secs(30),
                    factor: 2.0,
                },
            },
            children: Vec::new(),
        }
    }

    pub fn with_strategy(mut self, strategy: SupervisionStrategy) -> Self {
        self.root_strategy = strategy;
        self
    }

    pub fn with_restart_policy(mut self, policy: RestartPolicy) -> Self {
        self.restart_policy = policy;
        self
    }

    pub fn add_child(mut self, id: &str, node_type: NodeType) -> Self {
        self.children.push(TestChildSpec {
            id: id.to_string(),
            node_type,
            strategy: None,
        });
        self
    }

    pub fn add_supervisor(mut self, id: &str, strategy: SupervisionStrategy) -> Self {
        self.children.push(TestChildSpec {
            id: id.to_string(),
            node_type: NodeType::Supervisor,
            strategy: Some(strategy),
        });
        self
    }

    pub async fn build(self) -> Result<SupervisionTree> {
        let mut tree = SupervisionTree::new();
        
        // Create root supervisor
        let root_id = NodeId::from("test-root");
        tree.set_root_supervisor(
            root_id.clone(),
            self.root_strategy,
            self.restart_policy.clone(),
        ).await?;

        // Add children
        for child in self.children {
            match child.node_type {
                NodeType::Worker => {
                    tree.add_child(
                        root_id.clone(),
                        ChildId::from(child.id),
                        child.node_type,
                    ).await?;
                }
                NodeType::Supervisor => {
                    let child_id = NodeId::from(child.id);
                    tree.add_supervisor(
                        root_id.clone(),
                        child_id,
                        child.strategy.unwrap_or(SupervisionStrategy::OneForOne),
                        self.restart_policy.clone(),
                    ).await?;
                }
                _ => {}
            }
        }

        Ok(tree)
    }
}

/// Builder for creating test agent pools
pub struct TestAgentPoolBuilder {
    max_agents: usize,
    agent_configs: Vec<AgentConfig>,
    mock_claude: bool,
}

impl TestAgentPoolBuilder {
    pub fn new() -> Self {
        Self {
            max_agents: 3,
            agent_configs: Vec::new(),
            mock_claude: true,
        }
    }

    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.max_agents = capacity;
        self
    }

    pub fn add_agent(mut self, id: &str) -> Self {
        self.agent_configs.push(AgentConfig {
            id: AgentId::from(id),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["test".to_string()],
            resource_limits: Default::default(),
        });
        self
    }

    pub fn with_real_claude(mut self) -> Self {
        self.mock_claude = false;
        self
    }

    pub async fn build(self, nats_url: &str) -> Result<AgentPool> {
        let transport = Arc::new(NatsTransport::new(nats_url).await?);
        let mut pool = AgentPool::new(self.max_agents, transport);

        // Pre-populate with configured agents
        for config in self.agent_configs {
            if self.mock_claude {
                // Use mock Claude executor for tests
                pool.spawn_agent_with_executor(
                    config,
                    Arc::new(MockClaudeExecutor::new())
                ).await?;
            } else {
                pool.spawn_agent(config).await?;
            }
        }

        Ok(pool)
    }
}

/// Mock Claude executor for testing
pub struct MockClaudeExecutor {
    pub response_delay: Duration,
    pub should_fail: bool,
}

impl MockClaudeExecutor {
    pub fn new() -> Self {
        Self {
            response_delay: Duration::from_millis(10),
            should_fail: false,
        }
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.response_delay = delay;
        self
    }

    pub fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }
}

#[async_trait::async_trait]
impl mistersmith::runtime::claude_executor::ClaudeExecutor for MockClaudeExecutor {
    async fn execute(&self, _prompt: &str) -> Result<String> {
        tokio::time::sleep(self.response_delay).await;
        
        if self.should_fail {
            Err(anyhow::anyhow!("Mock Claude executor failure"))
        } else {
            Ok("Mock response from Claude".to_string())
        }
    }
}

/// Create a standard test configuration
pub fn test_config() -> mistersmith::config::Config {
    mistersmith::config::Config {
        agent_pool: mistersmith::config::AgentPoolConfig {
            max_agents: 5,
            spawn_timeout: Duration::from_secs(10),
        },
        supervision: mistersmith::config::SupervisionConfig {
            enable: true,
            root_strategy: SupervisionStrategy::OneForOne,
            default_restart_policy: RestartPolicy {
                max_restarts: 3,
                time_window: Duration::from_secs(60),
                backoff_strategy: BackoffStrategy::Exponential {
                    initial: Duration::from_millis(100),
                    max: Duration::from_secs(30),
                    factor: 2.0,
                },
            },
        },
        transport: mistersmith::config::TransportConfig {
            nats_url: "nats://localhost:4222".to_string(),
            connect_timeout: Duration::from_secs(5),
        },
        persistence: mistersmith::config::PersistenceConfig {
            database_url: "postgres://test:test@localhost/test".to_string(),
            pool_size: 5,
        },
    }
}