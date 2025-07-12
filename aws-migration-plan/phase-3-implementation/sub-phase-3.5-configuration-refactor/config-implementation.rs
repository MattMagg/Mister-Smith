// Configuration Management Implementation for MisterSmith
// This file demonstrates the new configuration system replacing hardcoded values

use anyhow::{Context, Result};
use aws_sdk_secretsmanager::Client as SecretsClient;
use aws_sdk_ssm::Client as ParameterStoreClient;
use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tracing::{debug, info, warn};

// ===== Core Configuration Structures =====

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub nats: NatsConfig,
    pub http: HttpConfig,
    pub timeouts: TimeoutConfig,
    pub aws: Option<AwsConfig>,
    pub runtime: RuntimeConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NatsConfig {
    #[serde(default = "default_nats_url")]
    pub url: String,
    pub auth: Option<NatsAuth>,
    #[serde(with = "humantime_serde")]
    pub connect_timeout: Duration,
    #[serde(with = "humantime_serde")]
    pub ping_interval: Duration,
    pub max_reconnects: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NatsAuth {
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub nkey: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpConfig {
    #[serde(default = "default_http_port")]
    pub port: u16,
    #[serde(default = "default_sse_port")]
    pub sse_port: u16,
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    pub cors_origins: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeoutConfig {
    #[serde(with = "humantime_serde")]
    pub circuit_breaker_reset: Duration,
    #[serde(with = "humantime_serde")]
    pub process_execution: Duration,
    #[serde(with = "humantime_serde")]
    pub agent_heartbeat: Duration,
    #[serde(with = "humantime_serde")]
    pub task_completion: Duration,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AwsConfig {
    pub region: String,
    pub parameter_store_prefix: String,
    pub secrets_manager_prefix: String,
    pub use_parameter_store: bool,
    pub use_secrets_manager: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuntimeConfig {
    pub environment: String,
    pub log_level: String,
    pub enable_telemetry: bool,
    pub enable_circuit_breaker: bool,
}

// ===== Default Values =====

fn default_nats_url() -> String {
    "nats://localhost:4222".to_string()
}

fn default_http_port() -> u16 {
    8080
}

fn default_sse_port() -> u16 {
    3000
}

fn default_bind_address() -> String {
    "0.0.0.0".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            nats: NatsConfig::default(),
            http: HttpConfig::default(),
            timeouts: TimeoutConfig::default(),
            aws: None,
            runtime: RuntimeConfig::default(),
        }
    }
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: default_nats_url(),
            auth: None,
            connect_timeout: Duration::from_secs(10),
            ping_interval: Duration::from_secs(30),
            max_reconnects: Some(10),
        }
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            port: default_http_port(),
            sse_port: default_sse_port(),
            bind_address: default_bind_address(),
            cors_origins: vec!["*".to_string()],
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            circuit_breaker_reset: Duration::from_millis(100),
            process_execution: Duration::from_secs(10),
            agent_heartbeat: Duration::from_secs(30),
            task_completion: Duration::from_secs(300),
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            environment: "development".to_string(),
            log_level: "info".to_string(),
            enable_telemetry: true,
            enable_circuit_breaker: true,
        }
    }
}

// ===== Configuration Loader =====

pub struct ConfigLoader {
    aws_config: Option<aws_config::SdkConfig>,
    parameter_store_client: Option<ParameterStoreClient>,
    secrets_manager_client: Option<SecretsClient>,
}

impl ConfigLoader {
    pub async fn new() -> Result<Self> {
        // Check if AWS configuration is needed
        let use_aws = env::var("USE_AWS_CONFIG")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        if use_aws {
            info!("Initializing AWS configuration clients");
            let config = aws_config::load_from_env().await;
            let parameter_store = ParameterStoreClient::new(&config);
            let secrets_manager = SecretsClient::new(&config);

            Ok(Self {
                aws_config: Some(config),
                parameter_store_client: Some(parameter_store),
                secrets_manager_client: Some(secrets_manager),
            })
        } else {
            debug!("AWS configuration disabled, using local config only");
            Ok(Self {
                aws_config: None,
                parameter_store_client: None,
                secrets_manager_client: None,
            })
        }
    }

    pub async fn load_config(&self) -> Result<AppConfig> {
        let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        info!("Loading configuration for environment: {}", environment);

        // Start with base configuration
        let mut builder = Config::builder()
            // Load default configuration
            .set_default("runtime.environment", &environment)?
            // Load from configuration file if exists
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!("config/{}", environment)).required(false))
            // Override with environment variables
            .add_source(
                Environment::with_prefix("MISTERSMITH")
                    .separator("__")
                    .list_separator(","),
            );

        // Load from AWS if configured
        if let Some(ps_client) = &self.parameter_store_client {
            let aws_params = self.load_from_parameter_store(ps_client, &environment).await?;
            for (key, value) in aws_params {
                builder = builder.set_override(&key, value)?;
            }
        }

        // Build configuration
        let mut config: AppConfig = builder.build()?.try_deserialize()?;

        // Load secrets from AWS Secrets Manager if configured
        if let Some(sm_client) = &self.secrets_manager_client {
            self.load_secrets(&mut config, sm_client, &environment).await?;
        }

        // Apply environment variable overrides
        self.apply_env_overrides(&mut config);

        // Validate configuration
        self.validate_config(&config)?;

        info!("Configuration loaded successfully");
        Ok(config)
    }

    async fn load_from_parameter_store(
        &self,
        client: &ParameterStoreClient,
        environment: &str,
    ) -> Result<HashMap<String, String>> {
        let prefix = format!("/mistersmith/{}/", environment);
        info!("Loading parameters from AWS Parameter Store with prefix: {}", prefix);

        let mut params = HashMap::new();

        // Get parameters by path
        let response = client
            .get_parameters_by_path()
            .path(&prefix)
            .recursive(true)
            .with_decryption(true)
            .send()
            .await
            .context("Failed to load parameters from Parameter Store")?;

        for parameter in response.parameters.unwrap_or_default() {
            if let (Some(name), Some(value)) = (parameter.name, parameter.value) {
                // Convert parameter path to config key
                let key = name
                    .strip_prefix(&prefix)
                    .unwrap_or(&name)
                    .replace('/', ".");
                params.insert(key, value);
            }
        }

        debug!("Loaded {} parameters from Parameter Store", params.len());
        Ok(params)
    }

    async fn load_secrets(
        &self,
        config: &mut AppConfig,
        client: &SecretsClient,
        environment: &str,
    ) -> Result<()> {
        let secret_id = format!("mistersmith/{}/credentials", environment);
        info!("Loading secrets from AWS Secrets Manager: {}", secret_id);

        match client
            .get_secret_value()
            .secret_id(&secret_id)
            .send()
            .await
        {
            Ok(response) => {
                if let Some(secret_string) = response.secret_string {
                    let secrets: HashMap<String, String> = serde_json::from_str(&secret_string)
                        .context("Failed to parse secrets JSON")?;

                    // Apply secrets to configuration
                    if let Some(nats_username) = secrets.get("nats_username") {
                        config.nats.auth.get_or_insert(NatsAuth::default()).username =
                            Some(nats_username.clone());
                    }
                    if let Some(nats_password) = secrets.get("nats_password") {
                        config.nats.auth.get_or_insert(NatsAuth::default()).password =
                            Some(nats_password.clone());
                    }

                    debug!("Secrets loaded successfully");
                }
            }
            Err(e) => {
                warn!("Failed to load secrets: {}. Using configuration without secrets.", e);
            }
        }

        Ok(())
    }

    fn apply_env_overrides(&self, config: &mut AppConfig) {
        // NATS overrides
        if let Ok(nats_url) = env::var("NATS_URL") {
            info!("Overriding NATS URL from environment variable");
            config.nats.url = nats_url;
        }

        // HTTP overrides
        if let Ok(http_port) = env::var("HTTP_PORT") {
            if let Ok(port) = http_port.parse::<u16>() {
                info!("Overriding HTTP port from environment variable: {}", port);
                config.http.port = port;
            }
        }

        if let Ok(sse_port) = env::var("SSE_PORT") {
            if let Ok(port) = sse_port.parse::<u16>() {
                info!("Overriding SSE port from environment variable: {}", port);
                config.http.sse_port = port;
            }
        }

        // Timeout overrides
        if let Ok(cb_timeout) = env::var("CIRCUIT_BREAKER_TIMEOUT") {
            if let Ok(duration) = humantime::parse_duration(&cb_timeout) {
                info!("Overriding circuit breaker timeout from environment variable");
                config.timeouts.circuit_breaker_reset = duration;
            }
        }

        if let Ok(process_timeout) = env::var("PROCESS_TIMEOUT") {
            if let Ok(duration) = humantime::parse_duration(&process_timeout) {
                info!("Overriding process timeout from environment variable");
                config.timeouts.process_execution = duration;
            }
        }
    }

    fn validate_config(&self, config: &AppConfig) -> Result<()> {
        // Validate NATS URL
        if config.nats.url.is_empty() {
            return Err(anyhow::anyhow!("NATS URL cannot be empty"));
        }

        // Validate ports
        if config.http.port == 0 {
            return Err(anyhow::anyhow!("HTTP port cannot be 0"));
        }

        if config.http.sse_port == 0 {
            return Err(anyhow::anyhow!("SSE port cannot be 0"));
        }

        // Validate timeouts
        if config.timeouts.circuit_breaker_reset.as_millis() < 10 {
            return Err(anyhow::anyhow!(
                "Circuit breaker reset timeout must be at least 10ms"
            ));
        }

        Ok(())
    }
}

// ===== Usage Example =====

pub async fn load_configuration() -> Result<AppConfig> {
    let loader = ConfigLoader::new().await?;
    loader.load_config().await
}

// ===== Migration Helper =====

/// Helper function to migrate from hardcoded values to configuration
pub fn get_nats_url(config: Option<&AppConfig>) -> String {
    config
        .map(|c| c.nats.url.clone())
        .or_else(|| env::var("NATS_URL").ok())
        .unwrap_or_else(default_nats_url)
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.nats.url, "nats://localhost:4222");
        assert_eq!(config.http.port, 8080);
        assert_eq!(config.http.sse_port, 3000);
    }

    #[test]
    fn test_env_override() {
        env::set_var("NATS_URL", "nats://prod-nats:4222");
        let mut config = AppConfig::default();
        let loader = ConfigLoader {
            aws_config: None,
            parameter_store_client: None,
            secrets_manager_client: None,
        };
        loader.apply_env_overrides(&mut config);
        assert_eq!(config.nats.url, "nats://prod-nats:4222");
        env::remove_var("NATS_URL");
    }

    #[test]
    fn test_config_validation() {
        let loader = ConfigLoader {
            aws_config: None,
            parameter_store_client: None,
            secrets_manager_client: None,
        };

        // Valid config should pass
        let valid_config = AppConfig::default();
        assert!(loader.validate_config(&valid_config).is_ok());

        // Invalid config should fail
        let mut invalid_config = AppConfig::default();
        invalid_config.nats.url = String::new();
        assert!(loader.validate_config(&invalid_config).is_err());
    }
}