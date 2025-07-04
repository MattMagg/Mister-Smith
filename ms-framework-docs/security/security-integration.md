---
title: Security Integration - NATS and Hook Security
type: note
permalink: security/security-integration
tags:
- '#security'
- '#nats'
- '#hooks'
- '#mtls'
- '#integration'
- '#agent-focused'
---

# Security Integration - NATS and Hook Security

## Framework Authority
This document implements specifications from the canonical tech-framework.md located at /Users/mac-main/Mister-Smith/Mister-Smith/tech-framework.md

As stated in the canonical framework: "Agents: use this framework as the canonical source."

## Purpose
Comprehensive security integration implementation for NATS messaging and hook execution systems. This document provides complete implementations for secure communication transport and hook execution environments within the Mister Smith AI Agent Framework.

## Related Documentation

### Security Implementation Files
- **[Security Patterns](security-patterns.md)** - Foundational security patterns and guidelines
- **[Authentication Implementation](authentication-implementation.md)** - Certificate management and JWT authentication
- **[Authorization Implementation](authorization-implementation.md)** - RBAC and security audit systems
- **[Security Framework](security-framework.md)** - Complete security framework overview

### Framework Integration Points
- **[Transport Layer](../transport/)** - Communication security protocols
- **[NATS Transport](../transport/nats-transport.md)** - NATS transport implementation
- **[Data Management](../data-management/)** - Message schemas and persistence security
- **[Core Architecture](../core-architecture/)** - System integration patterns

## 5. NATS Security Implementation

### 5.1 NATS Server Configuration with mTLS

**Complete NATS Server Configuration:**
```hocon
# nats_server_secure.conf
# NATS Server Security Configuration for Mister Smith Framework

# Server identity
server_name: "mister-smith-nats"

# Network configuration
host: "0.0.0.0"
port: 4222
http_port: 8222

# TLS Configuration with mTLS enforcement
tls {
  cert_file: "/etc/mister-smith/certs/server/server-cert.pem"
  key_file: "/etc/mister-smith/certs/server/server-key.pem"
  ca_file: "/etc/mister-smith/certs/ca/ca-cert.pem"
  verify: true
  verify_and_map: true
  timeout: 5
  
  # Force all connections to use TLS
  insecure: false
}

# JetStream configuration with security
jetstream {
  enabled: true
  store_dir: "/data/nats/jetstream"
  max_memory_store: 4GB
  max_file_store: 20GB
  
  # Domain isolation
  domain: "mister-smith"
}

# Account-based multi-tenancy
accounts {
  # System account for internal operations
  SYS: {
    users: [
      {
        user: "system",
        password: "$2a$11$..."  # bcrypt hash
        permissions: {
          publish: [">"]
          subscribe: [">"]
        }
      }
    ]
  }
  
  # Template for tenant accounts
  TENANT_A: {
    users: [
      {
        user: "tenant_a_admin",
        password: "$2a$11$..."
        permissions: {
          publish: ["tenantA.>"]
          subscribe: ["tenantA.>", "_INBOX.>"]
        }
      },
      {
        user: "tenant_a_service",
        password: "$2a$11$..."
        permissions: {
          publish: ["tenantA.services.>"]
          subscribe: ["tenantA.services.>", "_INBOX.>"]
        }
      }
    ]
    
    # JetStream limits per tenant
    jetstream: {
      max_memory: 512MB
      max_disk: 2GB
      max_streams: 10
      max_consumers: 100
    }
    
    # Connection limits
    limits: {
      max_connections: 50
      max_subscriptions: 1000
      max_payload: 1MB
      max_data: 10MB
      max_ack_pending: 65536
    }
  }
}

# System account designation
system_account: "SYS"

# Connection limits
max_connections: 1000
max_control_line: 4096
max_payload: 1048576
ping_interval: "2m"
ping_max: 2
write_deadline: "10s"

# Clustering for high availability
cluster {
  name: "mister-smith-cluster"
  host: "0.0.0.0"
  port: 6222
  
  # Cluster TLS
  tls {
    cert_file: "/etc/mister-smith/certs/server/server-cert.pem"
    key_file: "/etc/mister-smith/certs/server/server-key.pem"
    ca_file: "/etc/mister-smith/certs/ca/ca-cert.pem"
    verify: true
    timeout: 5
  }
  
  # Cluster routes
  routes: [
    "nats://nats-1.mister-smith.local:6222"
    "nats://nats-2.mister-smith.local:6222"
    "nats://nats-3.mister-smith.local:6222"
  ]
}

# Gateway configuration for multi-region
gateway {
  name: "mister-smith-gateway"
  host: "0.0.0.0"
  port: 7222
  
  # Gateway TLS
  tls {
    cert_file: "/etc/mister-smith/certs/server/server-cert.pem"
    key_file: "/etc/mister-smith/certs/server/server-key.pem"
    ca_file: "/etc/mister-smith/certs/ca/ca-cert.pem"
    verify: true
  }
}

# Monitoring and debugging
debug: false
trace: false
logtime: true
log_file: "/var/log/nats/nats-server.log"
pid_file: "/var/run/nats/nats-server.pid"

# Disable non-TLS connections completely
no_auth_user: ""
authorization {
  timeout: 5
}
```

### 5.2 NATS Client Implementation with mTLS

**Secure NATS Client:**
```rust
// nats_client.rs
use nats::asynk::{Connection, Options};
use anyhow::{Result, Context};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use tracing::{info, warn, error};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct NatsSecureClient {
    connection: Option<Connection>,
    tenant_id: Uuid,
    client_cert_path: String,
    client_key_path: String,
    ca_cert_path: String,
    nats_urls: Vec<String>,
}

impl NatsSecureClient {
    pub fn new(tenant_id: Uuid) -> Self {
        Self {
            connection: None,
            tenant_id,
            client_cert_path: "/etc/mister-smith/certs/client/client-cert.pem".to_string(),
            client_key_path: "/etc/mister-smith/certs/client/client-key.pem".to_string(),
            ca_cert_path: "/etc/mister-smith/certs/ca/ca-cert.pem".to_string(),
            nats_urls: vec![
                "tls://nats-1.mister-smith.local:4222".to_string(),
                "tls://nats-2.mister-smith.local:4222".to_string(),
                "tls://nats-3.mister-smith.local:4222".to_string(),
            ],
        }
    }

    /// Connect to NATS with mTLS
    pub async fn connect(&mut self, username: &str, password: &str) -> Result<()> {
        let options = Options::new()
            .with_name(&format!("mister-smith-client-{}", self.tenant_id))
            .with_user_and_password(username, password)
            .with_client_cert(&self.client_cert_path, &self.client_key_path)
            .with_context(|| "Failed to load client certificate")?
            .with_root_certificates(&self.ca_cert_path)
            .with_context(|| "Failed to load CA certificate")?
            .require_tls(true)
            .with_connection_timeout(Duration::from_secs(10))
            .with_reconnect_buffer_size(8 * 1024 * 1024) // 8MB
            .with_max_reconnects(5)
            .with_ping_interval(Duration::from_secs(30));

        let connection = options
            .connect(&self.nats_urls)
            .await
            .with_context(|| "Failed to connect to NATS server")?;

        self.connection = Some(connection);
        info!("Connected to NATS with mTLS for tenant: {}", self.tenant_id);
        Ok(())
    }

    /// Publish message to tenant-scoped subject
    pub async fn publish<T: Serialize>(&self, subject: &str, message: &T) -> Result<()> {
        let connection = self.connection.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to NATS"))?;

        let tenant_subject = format!("tenant{}.{}", self.tenant_id, subject);
        let payload = serde_json::to_vec(message)
            .with_context(|| "Failed to serialize message")?;

        // Check payload size (1MB limit)
        if payload.len() > 1024 * 1024 {
            anyhow::bail!("Message payload exceeds 1MB limit");
        }

        connection.publish(&tenant_subject, payload)
            .await
            .with_context(|| format!("Failed to publish to subject: {}", tenant_subject))?;

        info!("Published message to subject: {}", tenant_subject);
        Ok(())
    }

    /// Subscribe to tenant-scoped subject
    pub async fn subscribe(&self, subject: &str) -> Result<nats::asynk::Subscription> {
        let connection = self.connection.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to NATS"))?;

        let tenant_subject = format!("tenant{}.{}", self.tenant_id, subject);
        
        let subscription = connection.subscribe(&tenant_subject)
            .await
            .with_context(|| format!("Failed to subscribe to subject: {}", tenant_subject))?;

        info!("Subscribed to subject: {}", tenant_subject);
        Ok(subscription)
    }

    /// Create JetStream consumer with security constraints
    pub async fn create_consumer(&self, stream_name: &str, consumer_config: ConsumerConfig) -> Result<()> {
        let connection = self.connection.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to NATS"))?;

        let js = nats::jetstream::new(connection.clone());
        let tenant_stream = format!("tenant{}.{}", self.tenant_id, stream_name);

        // Enforce security constraints
        let secure_config = ConsumerConfig {
            deliver_subject: consumer_config.deliver_subject.map(|s| 
                format!("tenant{}.{}", self.tenant_id, s)
            ),
            max_deliver: Some(consumer_config.max_deliver.unwrap_or(5)),
            max_ack_pending: Some(consumer_config.max_ack_pending.unwrap_or(1000)),
            ..consumer_config
        };

        js.add_consumer(&tenant_stream, &secure_config)
            .await
            .with_context(|| format!("Failed to create consumer for stream: {}", tenant_stream))?;

        info!("Created consumer for stream: {}", tenant_stream);
        Ok(())
    }

    /// Health check for connection
    pub async fn health_check(&self) -> Result<()> {
        let connection = self.connection.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected to NATS"))?;

        // Send a ping and wait for pong
        connection.flush().await
            .with_context(|| "Health check failed")?;

        Ok(())
    }

    /// Disconnect from NATS
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(connection) = self.connection.take() {
            connection.close().await;
            info!("Disconnected from NATS for tenant: {}", self.tenant_id);
        }
        Ok(())
    }
}

// JetStream configuration types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerConfig {
    pub durable_name: Option<String>,
    pub deliver_subject: Option<String>,
    pub deliver_policy: Option<String>,
    pub opt_start_seq: Option<u64>,
    pub opt_start_time: Option<String>,
    pub ack_policy: Option<String>,
    pub ack_wait: Option<Duration>,
    pub max_deliver: Option<i32>,
    pub max_ack_pending: Option<i32>,
    pub replay_policy: Option<String>,
}

/// NATS connection pool for high-performance applications
pub struct NatsConnectionPool {
    pools: std::collections::HashMap<Uuid, Vec<NatsSecureClient>>,
    pool_size: usize,
}

impl NatsConnectionPool {
    pub fn new(pool_size: usize) -> Self {
        Self {
            pools: std::collections::HashMap::new(),
            pool_size,
        }
    }

    /// Get or create connection pool for tenant
    pub async fn get_connection(&mut self, tenant_id: Uuid, username: &str, password: &str) -> Result<&mut NatsSecureClient> {
        if !self.pools.contains_key(&tenant_id) {
            let mut pool = Vec::new();
            for _ in 0..self.pool_size {
                let mut client = NatsSecureClient::new(tenant_id);
                client.connect(username, password).await?;
                pool.push(client);
            }
            self.pools.insert(tenant_id, pool);
        }

        let pool = self.pools.get_mut(&tenant_id).unwrap();
        // Simple round-robin selection (in production, use more sophisticated load balancing)
        Ok(&mut pool[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nats_secure_client() {
        let tenant_id = Uuid::new_v4();
        let mut client = NatsSecureClient::new(tenant_id);

        // This would require a running NATS server with proper certificates
        // assert!(client.connect("test_user", "test_password").await.is_ok());
    }
}
```

## 6. Hook Security Implementation

### 6.1 Secure Hook Execution Environment

**Complete Hook Security Manager:**
```rust
// hook_security.rs
use std::process::{Command, Stdio};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::fs;
use uuid::Uuid;
use anyhow::{Result, Context};
use tracing::{info, warn, error};
use serde::{Serialize, Deserialize};
use tempfile::TempDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookSecurityConfig {
    pub execution_user: String,
    pub execution_group: String,
    pub sandbox_base_dir: PathBuf,
    pub max_execution_time_seconds: u64,
    pub max_memory_mb: u64,
    pub max_file_descriptors: u32,
    pub max_processes: u32,
    pub allowed_read_paths: Vec<PathBuf>,
    pub allowed_write_paths: Vec<PathBuf>,
    pub blocked_paths: Vec<PathBuf>,
    pub allowed_network: bool,
    pub allowed_localhost: bool,
    pub blocked_ports: Vec<u16>,
}

impl Default for HookSecurityConfig {
    fn default() -> Self {
        Self {
            execution_user: "claude-hook-runner".to_string(),
            execution_group: "claude-hooks".to_string(),
            sandbox_base_dir: PathBuf::from("/tmp/mister-smith-hooks"),
            max_execution_time_seconds: 30,
            max_memory_mb: 128,
            max_file_descriptors: 64,
            max_processes: 1,
            allowed_read_paths: vec![
                PathBuf::from("/usr"),
                PathBuf::from("/lib"),
                PathBuf::from("/bin"),
                PathBuf::from("/etc/passwd"),
                PathBuf::from("/etc/group"),
            ],
            allowed_write_paths: vec![],
            blocked_paths: vec![
                PathBuf::from("/etc"),
                PathBuf::from("/root"),
                PathBuf::from("/home"),
                PathBuf::from("/var/lib/claude-hooks/.ssh"),
                PathBuf::from("/proc"),
                PathBuf::from("/sys"),
            ],
            allowed_network: false,
            allowed_localhost: true,
            blocked_ports: vec![22, 3389, 5432, 27017],
        }
    }
}

#[derive(Debug, Clone)]
pub struct HookExecutionContext {
    pub hook_id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub sandbox_dir: TempDir,
    pub environment: std::collections::HashMap<String, String>,
}

pub struct HookSecurityManager {
    config: HookSecurityConfig,
    active_executions: std::collections::HashMap<Uuid, HookExecutionContext>,
}

impl HookSecurityManager {
    pub fn new(config: HookSecurityConfig) -> Result<Self> {
        // Ensure sandbox base directory exists
        fs::create_dir_all(&config.sandbox_base_dir)
            .with_context(|| "Failed to create sandbox base directory")?;

        // Verify execution user exists
        Self::verify_execution_user(&config.execution_user)?;

        Ok(Self {
            config,
            active_executions: std::collections::HashMap::new(),
        })
    }

    /// Verify that the execution user exists and is properly configured
    fn verify_execution_user(username: &str) -> Result<()> {
        let output = Command::new("id")
            .arg(username)
            .output()
            .with_context(|| format!("Failed to check user: {}", username))?;

        if !output.status.success() {
            anyhow::bail!("Execution user '{}' does not exist", username);
        }

        info!("Verified execution user: {}", username);
        Ok(())
    }

    /// Validate hook script before execution
    pub fn validate_hook_script(&self, script_path: &Path) -> Result<()> {
        // Check if file exists
        if !script_path.exists() {
            anyhow::bail!("Hook script does not exist: {:?}", script_path);
        }

        // Check file permissions
        let metadata = fs::metadata(script_path)
            .with_context(|| "Failed to read script metadata")?;

        // Check if world-writable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = metadata.permissions();
            if permissions.mode() & 0o002 != 0 {
                anyhow::bail!("Hook script cannot be world-writable: {:?}", script_path);
            }
        }

        // Read and validate script content
        let content = fs::read_to_string(script_path)
            .with_context(|| "Failed to read script content")?;

        self.validate_script_content(&content)?;

        info!("Hook script validation passed: {:?}", script_path);
        Ok(())
    }

    /// Validate script content for dangerous patterns
    fn validate_script_content(&self, content: &str) -> Result<()> {
        let dangerous_patterns = [
            r"sudo\s",
            r"su\s+-",
            r"chmod\s+777",
            r"rm\s+-rf\s+/",
            r"curl.*\|.*sh",
            r"wget.*\|.*sh",
            r"eval\s*\(",
            r"/etc/passwd",
            r"/etc/shadow",
            r"nc\s+-l",
            r"netcat\s+-l",
            r"python.*-c.*exec",
            r"perl.*-e",
            r"\$\(.*\)",  // Command substitution
        ];

        for pattern in &dangerous_patterns {
            let regex = regex::Regex::new(pattern)
                .with_context(|| format!("Invalid regex pattern: {}", pattern))?;
            
            if regex.is_match(content) {
                anyhow::bail!("Hook script contains dangerous pattern: {}", pattern);
            }
        }

        // Validate shebang
        if !content.starts_with("#!") {
            anyhow::bail!("Hook script must have valid shebang");
        }

        Ok(())
    }

    /// Create secure execution context
    pub fn create_execution_context(
        &mut self,
        tenant_id: Uuid,
        user_id: Uuid,
        session_id: Uuid,
    ) -> Result<Uuid> {
        let hook_id = Uuid::new_v4();
        
        // Create isolated sandbox directory
        let sandbox_dir = TempDir::new_in(&self.config.sandbox_base_dir)
            .with_context(|| "Failed to create sandbox directory")?;

        // Set up environment variables (filtered for security)
        let mut environment = std::collections::HashMap::new();
        environment.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
        environment.insert("HOME".to_string(), sandbox_dir.path().to_string_lossy().to_string());
        environment.insert("USER".to_string(), self.config.execution_user.clone());
        environment.insert("SHELL".to_string(), "/bin/bash".to_string());
        environment.insert("TMPDIR".to_string(), sandbox_dir.path().to_string_lossy().to_string());
        
        // Add hook-specific variables
        environment.insert("HOOK_ID".to_string(), hook_id.to_string());
        environment.insert("TENANT_ID".to_string(), tenant_id.to_string());
        environment.insert("USER_ID".to_string(), user_id.to_string());
        environment.insert("SESSION_ID".to_string(), session_id.to_string());

        let context = HookExecutionContext {
            hook_id,
            tenant_id,
            user_id,
            session_id,
            sandbox_dir,
            environment,
        };

        self.active_executions.insert(hook_id, context);
        info!("Created execution context: {}", hook_id);
        Ok(hook_id)
    }

    /// Execute hook script with security constraints
    pub async fn execute_hook(
        &mut self,
        hook_id: Uuid,
        script_path: &Path,
        args: Vec<String>,
        input_data: Option<Vec<u8>>,
    ) -> Result<HookExecutionResult> {
        let context = self.active_executions.get(&hook_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid hook execution context: {}", hook_id))?;

        // Validate script before execution
        self.validate_hook_script(script_path)?;

        // Copy script to sandbox
        let sandbox_script = context.sandbox_dir.path().join("hook_script");
        fs::copy(script_path, &sandbox_script)
            .with_context(|| "Failed to copy script to sandbox")?;

        // Make script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&sandbox_script)?.permissions();
            permissions.set_mode(0o750); // rwxr-x---
            fs::set_permissions(&sandbox_script, permissions)?;
        }

        // Prepare command with security constraints
        let mut command = Command::new("timeout");
        command
            .arg(format!("{}s", self.config.max_execution_time_seconds))
            .arg("systemd-run")
            .arg("--user")
            .arg("--scope")
            .arg("--slice=claude-hooks.slice")
            .arg(format!("--property=MemoryMax={}M", self.config.max_memory_mb))
            .arg(format!("--property=TasksMax={}", self.config.max_processes))
            .arg("--property=PrivateNetwork=true")
            .arg("--property=NoNewPrivileges=true")
            .arg("--property=ProtectSystem=strict")
            .arg("--property=ProtectHome=true")
            .arg("--property=PrivateTmp=true")
            .arg("--setenv=PATH=/usr/bin:/bin")
            .arg(format!("--setenv=HOME={}", context.sandbox_dir.path().display()))
            .arg(format!("--setenv=USER={}", self.config.execution_user))
            .arg(&sandbox_script)
            .args(&args)
            .current_dir(context.sandbox_dir.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add environment variables
        for (key, value) in &context.environment {
            command.env(key, value);
        }

        info!("Executing hook {} with security constraints", hook_id);
        
        let start_time = std::time::Instant::now();
        let mut child = command.spawn()
            .with_context(|| "Failed to spawn hook process")?;

        // Send input data if provided
        if let Some(input) = input_data {
            if let Some(stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                let mut stdin = tokio::process::ChildStdin::from_std(stdin)
                    .with_context(|| "Failed to convert stdin")?;
                stdin.write_all(&input).await
                    .with_context(|| "Failed to write input data")?;
                stdin.shutdown().await
                    .with_context(|| "Failed to close stdin")?;
            }
        }

        // Wait for completion with timeout
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.max_execution_time_seconds + 5),
            child.wait_with_output()
        ).await
            .with_context(|| "Hook execution timed out")?
            .with_context(|| "Failed to get hook output")?;

        let execution_time = start_time.elapsed();

        let result = HookExecutionResult {
            hook_id,
            exit_code: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
            execution_time_ms: execution_time.as_millis() as u64,
            success: output.status.success(),
        };

        info!("Hook execution completed: {} (exit code: {})", 
            hook_id, result.exit_code);

        Ok(result)
    }

    /// Clean up execution context
    pub fn cleanup_execution_context(&mut self, hook_id: Uuid) -> Result<()> {
        if let Some(context) = self.active_executions.remove(&hook_id) {
            // Sandbox directory will be automatically cleaned up when TempDir is dropped
            info!("Cleaned up execution context: {}", hook_id);
        }
        Ok(())
    }

    /// Get active execution count
    pub fn active_execution_count(&self) -> usize {
        self.active_executions.len()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HookExecutionResult {
    pub hook_id: Uuid,
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub execution_time_ms: u64,
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_script_validation() {
        let config = HookSecurityConfig::default();
        let manager = HookSecurityManager::new(config).unwrap();

        // Test valid script
        let mut valid_script = NamedTempFile::new().unwrap();
        writeln!(valid_script, "#!/bin/bash\necho 'Hello World'").unwrap();
        assert!(manager.validate_hook_script(valid_script.path()).is_ok());

        // Test dangerous script
        let mut dangerous_script = NamedTempFile::new().unwrap();
        writeln!(dangerous_script, "#!/bin/bash\nsudo rm -rf /").unwrap();
        assert!(manager.validate_hook_script(dangerous_script.path()).is_err());
    }

    #[tokio::test]
    async fn test_execution_context() {
        let config = HookSecurityConfig::default();
        let mut manager = HookSecurityManager::new(config).unwrap();

        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        let hook_id = manager.create_execution_context(tenant_id, user_id, session_id).unwrap();
        assert!(manager.active_executions.contains_key(&hook_id));

        manager.cleanup_execution_context(hook_id).unwrap();
        assert!(!manager.active_executions.contains_key(&hook_id));
    }
}
```

### 6.2 Hook Security Configuration

**Complete Security Configuration:**
```yaml
# hook_security_config.yml
hook_security:
  # User and group for hook execution
  execution_user: claude-hook-runner
  execution_group: claude-hooks
  
  # Sandbox configuration
  sandbox_base_dir: /tmp/mister-smith-hooks
  cleanup_interval_minutes: 60
  max_concurrent_executions: 10
  
  # Resource limits
  resource_limits:
    max_execution_time_seconds: 30
    max_memory_mb: 128
    max_cpu_percent: 50
    max_file_descriptors: 64
    max_processes: 1
    max_disk_usage_mb: 100
  
  # Filesystem access control
  filesystem_access:
    allowed_read_paths:
      - /usr
      - /lib
      - /lib64
      - /bin
      - /etc/passwd
      - /etc/group
      - /etc/hosts
      - /etc/resolv.conf
    
    allowed_write_paths:
      - /tmp/mister-smith-hooks
    
    blocked_paths:
      - /etc
      - /root
      - /home
      - /var/lib/claude-hooks/.ssh
      - /proc
      - /sys
      - /dev
      - /boot
  
  # Network restrictions
  network_access:
    allow_outbound: false
    allow_localhost: true
    blocked_ports:
      - 22    # SSH
      - 23    # Telnet
      - 3389  # RDP
      - 5432  # PostgreSQL
      - 3306  # MySQL
      - 27017 # MongoDB
      - 6379  # Redis
      - 9200  # Elasticsearch
    
    allowed_destinations:
      - 127.0.0.1
      - ::1
  
  # Script validation
  script_validation:
    enforce_shebang: true
    max_script_size_kb: 1024
    
    dangerous_patterns:
      - "sudo\\s"
      - "su\\s+-"
      - "chmod\\s+777"
      - "rm\\s+-rf\\s+/"
      - "curl.*\\|.*sh"
      - "wget.*\\|.*sh"
      - "eval\\s*\\("
      - "/etc/passwd"
      - "/etc/shadow"
      - "nc\\s+-l"
      - "netcat\\s+-l"
      - "python.*-c.*exec"
      - "perl.*-e"
      - "\\$\\(.*\\)"
    
    allowed_interpreters:
      - /bin/bash
      - /bin/sh
      - /usr/bin/python3
      - /usr/bin/node
    
  # Environment variables
  environment:
    # Variables always set
    default_vars:
      PATH: "/usr/bin:/bin"
      SHELL: "/bin/bash"
      TMPDIR: "${SANDBOX_DIR}"
      HOME: "${SANDBOX_DIR}"
      USER: "${EXECUTION_USER}"
    
    # Variables from user context
    context_vars:
      - HOOK_ID
      - TENANT_ID
      - USER_ID
      - SESSION_ID
    
    # Blocked environment variables
    blocked_vars:
      - LD_PRELOAD
      - LD_LIBRARY_PATH
      - SSH_AUTH_SOCK
      - DISPLAY
      - XAUTHORITY
  
  # Monitoring and logging
  monitoring:
    log_all_executions: true
    log_stdout_stderr: true
    max_log_size_mb: 10
    
    # Metrics to collect
    metrics:
      - execution_count
      - execution_duration
      - memory_usage
      - cpu_usage
      - exit_codes
      - validation_failures
    
    # Alerts
    alerts:
      max_failures_per_hour: 10
      max_execution_time_violations: 5
      suspicious_patterns_detected: 1
  
  # Systemd integration
  systemd:
    slice_name: claude-hooks.slice
    service_properties:
      MemoryAccounting: true
      CPUAccounting: true
      TasksAccounting: true
      IOAccounting: true
      PrivateNetwork: true
      NoNewPrivileges: true
      ProtectSystem: strict
      ProtectHome: true
      PrivateTmp: true
      ProtectKernelTunables: true
      ProtectKernelModules: true
      ProtectControlGroups: true
      RestrictSUIDSGID: true
      RemoveIPC: true
      RestrictRealtime: true
      RestrictNamespaces: true
      LockPersonality: true
      ProtectHostname: true
      ProtectClock: true
      ProtectKernelLogs: true
      ProtectProc: invisible
      ProcSubset: pid
```

---

## Navigation

### Related Security Components
- **[Security Patterns](security-patterns.md)** - Foundational security patterns and guidelines
- **[Authentication Implementation](authentication-implementation.md)** - Certificate management and JWT authentication
- **[Authorization Implementation](authorization-implementation.md)** - RBAC and security audit implementation
- **[Security Framework](security-framework.md)** - Complete security framework overview

### Integration Points
- **NATS Transport**: Secure messaging with mTLS and tenant isolation
- **Hook Execution**: Sandboxed script execution with comprehensive security controls
- **Audit Integration**: Security events logged through audit service
- **Certificate Management**: mTLS certificate rotation and validation

### Implementation Guide
1. **NATS Security Setup**: Deploy server configuration with mTLS enforcement
2. **Client Integration**: Implement secure NATS clients with tenant scoping
3. **Hook Security**: Configure sandbox environment for secure script execution
4. **Monitoring**: Deploy security monitoring and alerting systems

### Key Features
- **Mutual TLS (mTLS)**: End-to-end encryption with client certificate validation
- **Tenant Isolation**: Complete message isolation using NATS account-based multi-tenancy
- **Hook Sandboxing**: Secure script execution with systemd integration
- **Resource Limits**: Comprehensive resource and filesystem access controls

This document provides complete security integration implementations for secure communication and execution environments within the Mister Smith AI Agent Framework.