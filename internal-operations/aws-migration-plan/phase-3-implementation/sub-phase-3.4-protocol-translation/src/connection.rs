// Connection management module for NATS to AWS protocol translation

use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::RwLock;
use aws_config::retry::RetryConfig;
use aws_config::Region;
use bb8::{Pool, PooledConnection};
use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("Pool error: {0}")]
    PoolError(#[from] bb8::RunError<NatsError>),
    
    #[error("AWS configuration error: {0}")]
    AwsConfigError(String),
    
    #[error("NATS connection error: {0}")]
    NatsError(#[from] NatsError),
    
    #[error("No pool available for cluster: {0}")]
    NoPoolForCluster(String),
}

#[derive(Error, Debug, Clone)]
pub enum NatsError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    #[error("Timeout")]
    Timeout,
}

// NATS connection manager for bb8 pool
pub struct NatsConnectionManager {
    urls: Vec<String>,
    auth_token: Option<String>,
    tls_config: Option<TlsConfig>,
}

#[async_trait::async_trait]
impl bb8::ManageConnection for NatsConnectionManager {
    type Connection = NatsConnection;
    type Error = NatsError;
    
    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        // Simulated NATS connection
        // In production, this would use actual NATS client
        Ok(NatsConnection {
            id: uuid::Uuid::new_v4().to_string(),
            urls: self.urls.clone(),
        })
    }
    
    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        // Validate connection is still alive
        conn.ping().await
    }
    
    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

// Simulated NATS connection
pub struct NatsConnection {
    id: String,
    urls: Vec<String>,
}

impl NatsConnection {
    pub async fn ping(&self) -> Result<(), NatsError> {
        // In production, would send actual ping
        Ok(())
    }
    
    pub async fn publish(&self, subject: &str, payload: &[u8]) -> Result<(), NatsError> {
        // In production, would publish to NATS
        Ok(())
    }
    
    pub async fn request(&self, subject: &str, payload: &[u8]) -> Result<Vec<u8>, NatsError> {
        // In production, would make NATS request
        Ok(vec![])
    }
}

// Main connection manager
pub struct ConnectionManager {
    nats_pools: HashMap<String, Pool<NatsConnectionManager>>,
    aws_config: aws_config::Config,
    eventbridge_client: aws_sdk_eventbridge::Client,
    sqs_client: aws_sdk_sqs::Client,
    lambda_client: aws_sdk_lambda::Client,
    dynamodb_client: aws_sdk_dynamodb::Client,
    health_checker: Arc<HealthChecker>,
}

impl ConnectionManager {
    pub async fn new(config: ConnectionConfig) -> Result<Self, ConnectionError> {
        // Initialize NATS connection pools
        let mut nats_pools = HashMap::new();
        
        for (cluster_name, cluster_config) in config.nats_clusters {
            let manager = NatsConnectionManager {
                urls: cluster_config.urls,
                auth_token: cluster_config.auth_token,
                tls_config: cluster_config.tls_config,
            };
            
            let pool = Pool::builder()
                .max_size(cluster_config.pool_size)
                .min_idle(Some(cluster_config.min_idle))
                .connection_timeout(Duration::from_secs(30))
                .idle_timeout(Some(Duration::from_secs(600)))
                .test_on_check_out(true)
                .build(manager)
                .await
                .map_err(|e| ConnectionError::AwsConfigError(e.to_string()))?;
            
            nats_pools.insert(cluster_name, pool);
        }
        
        // Initialize AWS SDK with retry configuration
        let retry_config = RetryConfig::standard()
            .with_max_attempts(config.aws_retry_attempts)
            .with_initial_backoff(Duration::from_millis(config.aws_retry_initial_backoff_ms));
        
        let aws_config = aws_config::from_env()
            .region(Region::new(config.aws_region.clone()))
            .retry_config(retry_config)
            .load()
            .await;
        
        // Create AWS service clients
        let eventbridge_client = aws_sdk_eventbridge::Client::new(&aws_config);
        let sqs_client = aws_sdk_sqs::Client::new(&aws_config);
        let lambda_client = aws_sdk_lambda::Client::new(&aws_config);
        let dynamodb_client = aws_sdk_dynamodb::Client::new(&aws_config);
        
        // Initialize health checker
        let health_checker = Arc::new(HealthChecker::new(
            config.health_check_interval_secs,
            config.health_check_timeout_secs,
        ));
        
        Ok(Self {
            nats_pools,
            aws_config: aws_config.clone(),
            eventbridge_client,
            sqs_client,
            lambda_client,
            dynamodb_client,
            health_checker,
        })
    }
    
    pub async fn get_nats_connection(
        &self,
        cluster: &str,
    ) -> Result<PooledConnection<'_, NatsConnectionManager>, ConnectionError> {
        self.nats_pools
            .get(cluster)
            .ok_or_else(|| ConnectionError::NoPoolForCluster(cluster.to_string()))?
            .get()
            .await
            .map_err(Into::into)
    }
    
    pub fn eventbridge(&self) -> &aws_sdk_eventbridge::Client {
        &self.eventbridge_client
    }
    
    pub fn sqs(&self) -> &aws_sdk_sqs::Client {
        &self.sqs_client
    }
    
    pub fn lambda(&self) -> &aws_sdk_lambda::Client {
        &self.lambda_client
    }
    
    pub fn dynamodb(&self) -> &aws_sdk_dynamodb::Client {
        &self.dynamodb_client
    }
    
    pub async fn start_health_monitoring(&self) {
        let health_checker = self.health_checker.clone();
        let nats_pools = self.nats_pools.clone();
        let eventbridge = self.eventbridge_client.clone();
        let sqs = self.sqs_client.clone();
        let lambda = self.lambda_client.clone();
        let dynamodb = self.dynamodb_client.clone();
        
        tokio::spawn(async move {
            health_checker
                .monitor_connections(nats_pools, eventbridge, sqs, lambda, dynamodb)
                .await;
        });
    }
}

// Health checker
pub struct HealthChecker {
    check_interval: Duration,
    check_timeout: Duration,
    health_status: Arc<RwLock<HealthStatus>>,
}

#[derive(Debug, Clone, Default)]
pub struct HealthStatus {
    pub nats_clusters: HashMap<String, ServiceHealth>,
    pub aws_services: HashMap<String, ServiceHealth>,
    pub last_check: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct ServiceHealth {
    pub healthy: bool,
    pub last_error: Option<String>,
    pub consecutive_failures: u32,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

impl HealthChecker {
    pub fn new(check_interval_secs: u64, check_timeout_secs: u64) -> Self {
        Self {
            check_interval: Duration::from_secs(check_interval_secs),
            check_timeout: Duration::from_secs(check_timeout_secs),
            health_status: Arc::new(RwLock::new(HealthStatus::default())),
        }
    }
    
    pub async fn monitor_connections(
        &self,
        nats_pools: HashMap<String, Pool<NatsConnectionManager>>,
        eventbridge: aws_sdk_eventbridge::Client,
        sqs: aws_sdk_sqs::Client,
        lambda: aws_sdk_lambda::Client,
        dynamodb: aws_sdk_dynamodb::Client,
    ) {
        let mut interval = tokio::time::interval(self.check_interval);
        
        loop {
            interval.tick().await;
            
            let mut status = self.health_status.write().await;
            
            // Check NATS connections
            for (cluster_name, pool) in &nats_pools {
                let health = match tokio::time::timeout(
                    self.check_timeout,
                    pool.get()
                ).await {
                    Ok(Ok(mut conn)) => {
                        match conn.ping().await {
                            Ok(_) => ServiceHealth {
                                healthy: true,
                                last_error: None,
                                consecutive_failures: 0,
                                last_check: chrono::Utc::now(),
                            },
                            Err(e) => ServiceHealth {
                                healthy: false,
                                last_error: Some(e.to_string()),
                                consecutive_failures: status.nats_clusters
                                    .get(cluster_name)
                                    .map(|h| h.consecutive_failures + 1)
                                    .unwrap_or(1),
                                last_check: chrono::Utc::now(),
                            }
                        }
                    }
                    _ => ServiceHealth {
                        healthy: false,
                        last_error: Some("Connection timeout".to_string()),
                        consecutive_failures: status.nats_clusters
                            .get(cluster_name)
                            .map(|h| h.consecutive_failures + 1)
                            .unwrap_or(1),
                        last_check: chrono::Utc::now(),
                    }
                };
                
                status.nats_clusters.insert(cluster_name.clone(), health);
            }
            
            // Check AWS services
            self.check_aws_service(&mut status, "eventbridge", || async {
                eventbridge.list_rules().limit(1).send().await.map(|_| ())
            }).await;
            
            self.check_aws_service(&mut status, "sqs", || async {
                sqs.list_queues().max_results(1).send().await.map(|_| ())
            }).await;
            
            self.check_aws_service(&mut status, "lambda", || async {
                lambda.list_functions().max_items(1).send().await.map(|_| ())
            }).await;
            
            self.check_aws_service(&mut status, "dynamodb", || async {
                dynamodb.list_tables().limit(1).send().await.map(|_| ())
            }).await;
            
            status.last_check = Some(chrono::Utc::now());
        }
    }
    
    async fn check_aws_service<F, Fut>(
        &self,
        status: &mut HealthStatus,
        service_name: &str,
        check_fn: F,
    ) where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    {
        let health = match tokio::time::timeout(self.check_timeout, check_fn()).await {
            Ok(Ok(_)) => ServiceHealth {
                healthy: true,
                last_error: None,
                consecutive_failures: 0,
                last_check: chrono::Utc::now(),
            },
            Ok(Err(e)) => ServiceHealth {
                healthy: false,
                last_error: Some(e.to_string()),
                consecutive_failures: status.aws_services
                    .get(service_name)
                    .map(|h| h.consecutive_failures + 1)
                    .unwrap_or(1),
                last_check: chrono::Utc::now(),
            },
            Err(_) => ServiceHealth {
                healthy: false,
                last_error: Some("Health check timeout".to_string()),
                consecutive_failures: status.aws_services
                    .get(service_name)
                    .map(|h| h.consecutive_failures + 1)
                    .unwrap_or(1),
                last_check: chrono::Utc::now(),
            }
        };
        
        status.aws_services.insert(service_name.to_string(), health);
    }
    
    pub async fn get_status(&self) -> HealthStatus {
        self.health_status.read().await.clone()
    }
}

// Configuration structures
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionConfig {
    pub nats_clusters: HashMap<String, NatsClusterConfig>,
    pub aws_region: String,
    pub aws_retry_attempts: u32,
    pub aws_retry_initial_backoff_ms: u64,
    pub health_check_interval_secs: u64,
    pub health_check_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NatsClusterConfig {
    pub urls: Vec<String>,
    pub auth_token: Option<String>,
    pub tls_config: Option<TlsConfig>,
    pub pool_size: u32,
    pub min_idle: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TlsConfig {
    pub ca_cert_path: Option<String>,
    pub client_cert_path: Option<String>,
    pub client_key_path: Option<String>,
}

// Reconnection manager
pub struct ReconnectionManager {
    connection_manager: Arc<ConnectionManager>,
    retry_policy: ExponentialBackoff,
}

#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    initial_interval: Duration,
    max_interval: Duration,
    multiplier: f64,
    max_retries: Option<u32>,
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self {
            initial_interval: Duration::from_millis(100),
            max_interval: Duration::from_secs(60),
            multiplier: 2.0,
            max_retries: None,
        }
    }
}

impl ReconnectionManager {
    pub fn new(connection_manager: Arc<ConnectionManager>) -> Self {
        Self {
            connection_manager,
            retry_policy: ExponentialBackoff::default(),
        }
    }
    
    pub async fn maintain_connections(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            let health_status = self.connection_manager
                .health_checker
                .get_status()
                .await;
            
            // Check for unhealthy NATS connections
            for (cluster_name, health) in &health_status.nats_clusters {
                if !health.healthy && health.consecutive_failures > 2 {
                    tracing::warn!(
                        "NATS cluster {} is unhealthy, attempting reconnection",
                        cluster_name
                    );
                    
                    // Trigger pool refresh
                    if let Some(pool) = self.connection_manager.nats_pools.get(cluster_name) {
                        // Force connection validation which will trigger reconnects
                        let _ = pool.get().await;
                    }
                }
            }
            
            // AWS SDK handles reconnection automatically
            // Just log unhealthy services
            for (service_name, health) in &health_status.aws_services {
                if !health.healthy && health.consecutive_failures > 2 {
                    tracing::warn!(
                        "AWS service {} is experiencing issues: {:?}",
                        service_name,
                        health.last_error
                    );
                }
            }
        }
    }
}