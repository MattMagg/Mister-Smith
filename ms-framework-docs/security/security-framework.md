---
title: Security Framework - Revised
type: note
permalink: revision-swarm/security/security-framework-revised
tags:
- '#security'
- '#revision'
- '#foundational'
- '#agent-focused'
---

# Security Framework - Foundational Patterns

## Framework Authority
This document implements specifications from the canonical tech-framework.md located at /Users/mac-main/Mister-Smith/Mister-Smith/tech-framework.md

As stated in the canonical framework: "Agents: use this framework as the canonical source."

## Purpose
Foundational security patterns for agent implementation focusing on basic authentication, authorization, TLS setup, and secrets management. This document provides pseudocode patterns and configurations for learning and implementation by agents.

## Core Security Components

### 1. Basic Authentication Pattern

**Pseudocode Pattern:**
```pseudocode
// Basic JWT Authentication Flow
function authenticate_request(request):
    token = extract_bearer_token(request.headers)
    if not token:
        return error(401, "No authentication provided")
    
    claims = verify_jwt_token(token, public_key)
    if not claims or claims.expired:
        return error(401, "Invalid or expired token")
    
    request.context.user_id = claims.subject
    return success()

// Token Generation Pattern
function generate_auth_token(user_id, expires_in):
    claims = {
        subject: user_id,
        issued_at: current_timestamp(),
        expires_at: current_timestamp() + expires_in
    }
    return sign_jwt(claims, private_key)
```

**Configuration Pattern:**
```yaml
authentication:
  jwt:
    algorithm: RS256
    public_key_path: /path/to/public.pem
    private_key_path: /path/to/private.pem
    token_expiry: 3600  # seconds
```

### 2. Simple Authorization Pattern

**Pseudocode Pattern:**
```pseudocode
// Role-Based Access Control Pattern
function check_permission(user_id, resource, action):
    user_roles = get_user_roles(user_id)
    
    for role in user_roles:
        permissions = get_role_permissions(role)
        if has_permission(permissions, resource, action):
            return allow()
    
    return deny()

// Permission Definition Structure
permissions = {
    "reader": ["read:*"],
    "writer": ["read:*", "write:own"],
    "admin": ["read:*", "write:*", "delete:*"]
}
```

**Configuration Pattern:**
```yaml
authorization:
  type: role_based
  default_role: reader
  roles:
    - name: reader
      permissions: [read]
    - name: writer  
      permissions: [read, write]
    - name: admin
      permissions: [read, write, delete, admin]
```

### 3. TLS Configuration Pattern

**Pseudocode Pattern:**
```pseudocode
// TLS Server Setup
function create_tls_server(cert_path, key_path):
    tls_config = {
        certificate: load_certificate(cert_path),
        private_key: load_private_key(key_path),
        min_version: "TLS_1_2",
        cipher_suites: ["TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384"]
    }
    
    return create_server_with_tls(tls_config)

// TLS Client Configuration
function create_tls_client(ca_cert_path):
    tls_config = {
        ca_certificate: load_certificate(ca_cert_path),
        verify_hostname: true,
        min_version: "TLS_1_2"
    }
    
    return create_client_with_tls(tls_config)
```

**Configuration Pattern:**
```yaml
tls:
  server:
    cert_path: /certs/server.crt
    key_path: /certs/server.key
    min_version: TLS1.2
  client:
    ca_cert_path: /certs/ca.crt
    verify_hostname: true
```

### 4. Basic Secrets Management

**Pseudocode Pattern:**
```pseudocode
// Environment-Based Secrets
function load_secrets():
    secrets = {
        database_url: get_env("DATABASE_URL"),
        api_key: get_env("API_KEY"),
        jwt_secret: get_env("JWT_SECRET")
    }
    
    // Validate required secrets
    for key, value in secrets:
        if not value:
            error("Missing required secret: " + key)
    
    return secrets

// File-Based Secrets Pattern
function load_secrets_from_file(path):
    if not file_exists(path):
        error("Secrets file not found")
    
    // Ensure proper file permissions
    if not check_file_permissions(path, "600"):
        error("Insecure secrets file permissions")
    
    return parse_secrets_file(path)
```

**Configuration Pattern:**
```yaml
secrets:
  source: environment  # or 'file'
  file_path: /secrets/app.secrets
  required:
    - DATABASE_URL
    - JWT_SECRET
    - API_KEY
```

### 5. Basic Security Middleware

**Pseudocode Pattern:**
```pseudocode
// Security Headers Middleware
function security_headers_middleware(request, response, next):
    response.headers.add("X-Content-Type-Options", "nosniff")
    response.headers.add("X-Frame-Options", "DENY")
    response.headers.add("X-XSS-Protection", "1; mode=block")
    response.headers.add("Strict-Transport-Security", "max-age=31536000")
    
    return next(request, response)

// Rate Limiting Pattern
function rate_limit_middleware(request, response, next):
    client_id = get_client_identifier(request)
    
    if exceeded_rate_limit(client_id):
        return error(429, "Rate limit exceeded")
    
    increment_request_count(client_id)
    return next(request, response)
```

### 6. Basic Audit Logging

**Pseudocode Pattern:**
```pseudocode
// Security Event Logging
function log_security_event(event_type, details):
    event = {
        timestamp: current_timestamp(),
        event_type: event_type,
        details: details,
        source_ip: get_request_ip(),
        user_id: get_current_user_id()
    }
    
    append_to_audit_log(event)

// Common Security Events to Log
security_events = [
    "authentication_success",
    "authentication_failure", 
    "authorization_denied",
    "invalid_token",
    "rate_limit_exceeded",
    "suspicious_activity"
]
```

### 7. NATS Security Patterns

**Pseudocode Pattern - mTLS Configuration:**
```pseudocode
// Initialize secure NATS connection with mTLS
function init_secure_nats(tenant_id):
    // Load certificates from secure storage
    ca_cert = load_file("/secrets/ca.crt")
    client_cert = load_file("/secrets/client.crt")
    client_key = load_file("/secrets/client.key")
    
    // Configure connection with tenant-specific identity
    options = {
        require_tls: true,
        ca_certificate: ca_cert,
        client_certificate: client_cert,
        client_key: client_key,
        client_name: "tenant_" + tenant_id + "_agent"
    }
    
    return connect_nats(NATS_URL, options)

// Apply rate limiting and backpressure
function configure_nats_limits(connection):
    connection.subscription_capacity = 1000  // Bounded buffer
    connection.ping_interval = 10  // seconds
    connection.reconnect_buffer_size = 8388608  // 8MB
```

**Configuration Pattern - Server mTLS:**
```hocon
# NATS server mTLS configuration
tls {
  cert_file: "./certs/nats-server.crt"
  key_file:  "./certs/nats-server.key"
  ca_file:   "./certs/ca.crt"
  verify: true  # Enforce client certificates
}

cluster {
  tls {
    cert_file: "./certs/cluster.crt"
    key_file:  "./certs/cluster.key"
    ca_file:   "./certs/ca.crt"
  }
}
```

**Account-Based Tenant Isolation Pattern:**
```yaml
# NATS account isolation pattern
account_isolation:
  principle: "Each tenant gets isolated NATS account"
  benefits:
    - Complete namespace separation
    - No subject prefix complexity
    - Built-in multi-tenancy support
  implementation: |
    nsc add account --name tenantA
    nsc edit account --name tenantA \
      --js-mem-storage 512M \
      --js-disk-storage 1G \
      --js-streams 10 \
      --js-consumer 50
```

**Fine-Grained ACL Configuration:**
```json
// Per-user permission model
{
  "users": [
    {
      "user": "admin",
      "permissions": {
        "publish": [ ">" ],     // Full access
        "subscribe": [ ">" ]
      }
    },
    {
      "user": "tenantA_bot",
      "permissions": {
        "publish": { "allow": ["tenantA.>"] },
        "subscribe": { "allow": ["tenantA.>"] }
      }
    }
  ]
}
```

**Resource Quota Enforcement:**
```yaml
# Per-account resource limits
jetstream_limits:
  per_account:
    max_memory: 512M
    max_disk: 1G
    max_streams: 10
    max_consumers: 100
  per_stream:
    max_bytes: configurable
    max_msgs: configurable
    max_age: configurable
    discard_policy: old_on_full
  connection_limits:
    max_connections: 100
    max_subscriptions: 1000
    max_payload_size: 1MB
```

**Key Rotation Pattern:**
```pseudocode
// Zero-downtime key rotation state machine
key_rotation_states = [
    "KEY_A_ACTIVE",
    "STAGING_NEW_KEY",
    "RELOADING_CONFIG",
    "KEY_B_ACTIVE"
]

// Signal handler for hot reload
function handle_sighup_signal():
    if signal_received == SIGHUP:
        // Atomically swap API keys in memory
        rotate_keys()
        reload_tls_certificates()
        update_active_connections()
```

**Critical NATS Security Patterns:**
1. **Never share accounts between tenants** - Use NATS accounts for true isolation
2. **Always enforce mTLS** - Both client and cluster connections must verify certificates
3. **Apply least privilege** - Restrict subjects to minimum required patterns
4. **Set resource quotas** - Prevent any tenant from exhausting cluster resources
5. **Rotate secrets regularly** - Use SIGHUP for zero-downtime key rotation
6. **Monitor wildcard usage** - Detect and prevent unauthorized subject access

## Implementation Guidelines

### Authentication Flow
1. Extract authentication token from request
2. Verify token signature and expiration
3. Extract user identity from token claims
4. Attach identity to request context

### Authorization Flow
1. Identify resource and action from request
2. Retrieve user roles/permissions
3. Check if user has required permission
4. Allow or deny based on permission check

### TLS Setup Flow
1. Generate or obtain TLS certificates
2. Configure minimum TLS version (1.2+)
3. Select secure cipher suites
4. Enable hostname verification for clients

### Secrets Management Flow
1. Define required secrets
2. Load from environment or secure file
3. Validate all required secrets present
4. Use secrets for service configuration

### NATS Security Flow
1. Generate or obtain mTLS certificates for NATS
2. Create isolated accounts for each tenant
3. Configure ACLs for subject-based access control
4. Set resource quotas to prevent resource exhaustion
5. Implement key rotation handlers for hot reload

## Security Checklist for Agents

- [ ] Implement authentication before processing requests
- [ ] Check authorization for protected resources
- [ ] Enable TLS for all network communication
- [ ] Store secrets securely (environment variables or encrypted files)
- [ ] Add security headers to HTTP responses
- [ ] Implement rate limiting for API endpoints
- [ ] Log security-relevant events for audit trails
- [ ] Validate and sanitize all input data
- [ ] Use secure random number generation for tokens
- [ ] Set appropriate timeouts for authentication tokens
- [ ] Configure NATS with mTLS for secure messaging
- [ ] Implement account-based isolation for multi-tenant NATS
- [ ] Set resource quotas for NATS accounts and streams
- [ ] Configure fine-grained ACLs for NATS subjects
- [ ] Implement key rotation for zero-downtime secret updates

## Configuration Templates

### Basic Security Configuration
```yaml
security:
  authentication:
    enabled: true
    type: jwt
    token_expiry: 3600
  
  authorization:
    enabled: true
    type: role_based
    default_role: reader
  
  tls:
    enabled: true
    min_version: TLS1.2
  
  rate_limiting:
    enabled: true
    requests_per_minute: 60
  
  audit_logging:
    enabled: true
    log_path: /logs/security.log
  
  nats:
    enabled: true
    mtls:
      cert_path: /certs/nats-client.crt
      key_path: /certs/nats-client.key
      ca_path: /certs/ca.crt
    account_isolation: true
    resource_quotas:
      max_memory: 512M
      max_disk: 1G
      max_connections: 100
```

## 11. Hook Execution Sandbox Pattern

### 11.1 Non-Root User Execution Pattern

**Pseudocode Pattern:**
```pseudocode
// Hook execution with privilege isolation
function execute_hook_safely(hook_script, payload):
    // Ensure hook runs under non-root user
    execution_user = get_non_privileged_user()  // e.g., "claude-hook-runner"

    // Create isolated execution environment
    sandbox_config = {
        user: execution_user,
        working_directory: "/tmp/hook-sandbox",
        environment_variables: filter_safe_env_vars(),
        resource_limits: {
            max_memory: "128M",
            max_cpu_time: "30s",
            max_file_descriptors: 64,
            max_processes: 1
        },
        filesystem_access: {
            read_only: ["/usr", "/lib", "/bin"],
            read_write: ["/tmp/hook-sandbox"],
            no_access: ["/etc", "/root", "/home"]
        }
    }

    // Execute with timeout and resource constraints
    result = execute_with_sandbox(hook_script, payload, sandbox_config)

    // Clean up sandbox environment
    cleanup_sandbox_directory()

    return result

// User privilege management
function setup_hook_user():
    // Create dedicated user for hook execution
    create_user("claude-hook-runner", {
        home_directory: "/var/lib/claude-hooks",
        shell: "/bin/bash",
        groups: ["claude-hooks"],
        no_login: false,
        system_user: true
    })

    // Set up hook directory permissions
    set_directory_permissions("/var/lib/claude-hooks", {
        owner: "claude-hook-runner",
        group: "claude-hooks",
        permissions: "750"
    })
```

**Configuration Pattern:**
```yaml
hook_security:
  execution_user: claude-hook-runner
  sandbox_directory: /tmp/hook-sandbox
  resource_limits:
    max_memory_mb: 128
    max_cpu_seconds: 30
    max_file_descriptors: 64
    max_processes: 1

  filesystem_isolation:
    read_only_paths:
      - /usr
      - /lib
      - /bin
      - /etc/passwd
    read_write_paths:
      - /tmp/hook-sandbox
    blocked_paths:
      - /etc
      - /root
      - /home
      - /var/lib/claude-hooks/.ssh

  network_isolation:
    allow_outbound: false
    allow_localhost: true
    blocked_ports: [22, 3389, 5432, 27017]
```

### 11.2 Hook Script Validation Pattern

**Pseudocode Pattern:**
```pseudocode
// Validate hook scripts before execution
function validate_hook_script(script_path):
    // Check file permissions
    file_info = get_file_info(script_path)
    if file_info.owner != "claude-hook-runner":
        return error("Hook script must be owned by claude-hook-runner")

    if file_info.permissions & WORLD_WRITABLE:
        return error("Hook script cannot be world-writable")

    // Validate script content
    script_content = read_file(script_path)

    // Check for dangerous patterns
    dangerous_patterns = [
        "sudo", "su -", "chmod 777", "rm -rf /",
        "curl.*|.*sh", "wget.*|.*sh", "eval",
        "/etc/passwd", "/etc/shadow"
    ]

    for pattern in dangerous_patterns:
        if matches_pattern(script_content, pattern):
            return error("Hook script contains dangerous pattern: " + pattern)

    // Validate shebang
    if not script_content.starts_with("#!/"):
        return error("Hook script must have valid shebang")

    return success()

// Hook directory security
function secure_hook_directory(hook_dir):
    // Ensure proper ownership and permissions
    set_ownership(hook_dir, "claude-hook-runner", "claude-hooks")
    set_permissions(hook_dir, "750")  // rwxr-x---

    // Validate all hook scripts
    for script in list_files(hook_dir):
        validate_hook_script(script)
        set_permissions(script, "750")  // rwxr-x---
```

## Pattern Implementation Notes

- All patterns are foundational and can be extended as needed
- Focus on understanding core security concepts before adding complexity
- Use standard libraries for cryptographic operations
- Test security configurations in isolated environments
- Follow the principle of least privilege for all access control

---

This document provides foundational security patterns for agent implementation. For complete system architecture and advanced patterns, refer to the canonical tech-framework.md.