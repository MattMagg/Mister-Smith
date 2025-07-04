---
title: Authentication Implementation - Certificate + JWT
type: implementation
permalink: security/authentication-implementation
tags:
- '#security'
- '#authentication'
- '#implementation'
- '#certificates'
- '#jwt'
- '#agent-focused'
---

# Authentication Implementation - Certificate + JWT

## Framework Authority
This document implements specifications from the canonical security-framework.md located at /Users/mac-main/Mister-Smith/Mister-Smith/ms-framework-docs/security/security-framework.md

Extracted content: Certificate Management Implementation (Section 1) + JWT Authentication Implementation (Section 2)

## Purpose
Complete implementation patterns for certificate management and JWT authentication in the Mister Smith Framework. This document provides production-ready code for certificate lifecycle management, JWT token operations, and authentication middleware.

## 1. Certificate Management Implementation

### 1.1 Certificate Generation Scripts

**Complete Certificate Authority Setup:**
```bash
#!/bin/bash
# generate_ca.sh - Complete CA setup for Mister Smith Framework

set -euo pipefail

CA_DIR="/etc/mister-smith/certs"
CA_KEY_SIZE=4096
CERT_VALIDITY_DAYS=3650
SERVER_CERT_VALIDITY_DAYS=90
CLIENT_CERT_VALIDITY_DAYS=365

# Create directory structure
mkdir -p $CA_DIR/{ca,server,client,crl}
cd $CA_DIR

# Generate CA private key (RSA 4096-bit)
openssl genrsa -out ca/ca-key.pem $CA_KEY_SIZE

# Generate CA certificate (10 years)
openssl req -new -x509 -days $CERT_VALIDITY_DAYS -key ca/ca-key.pem \
    -out ca/ca-cert.pem \
    -subj "/C=US/ST=CA/L=San Francisco/O=Mister Smith Framework/OU=Security/CN=Mister Smith CA"

# Generate server private key
openssl genrsa -out server/server-key.pem 4096

# Generate server certificate signing request
openssl req -new -key server/server-key.pem -out server/server.csr \
    -subj "/C=US/ST=CA/L=San Francisco/O=Mister Smith Framework/OU=Services/CN=mister-smith.local"

# Create server certificate extensions
cat > server/server-ext.cnf << EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = mister-smith.local
DNS.2 = localhost
DNS.3 = *.mister-smith.local
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

# Sign server certificate (90 days)
openssl x509 -req -in server/server.csr -CA ca/ca-cert.pem -CAkey ca/ca-key.pem \
    -out server/server-cert.pem -days $SERVER_CERT_VALIDITY_DAYS \
    -extensions v3_req -extfile server/server-ext.cnf -CAcreateserial

# Generate client private key
openssl genrsa -out client/client-key.pem 4096

# Generate client certificate signing request
openssl req -new -key client/client-key.pem -out client/client.csr \
    -subj "/C=US/ST=CA/L=San Francisco/O=Mister Smith Framework/OU=Clients/CN=mister-smith-client"

# Create client certificate extensions
cat > client/client-ext.cnf << EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature
extendedKeyUsage = clientAuth
EOF

# Sign client certificate (365 days)
openssl x509 -req -in client/client.csr -CA ca/ca-cert.pem -CAkey ca/ca-key.pem \
    -out client/client-cert.pem -days $CLIENT_CERT_VALIDITY_DAYS \
    -extensions v3_req -extfile client/client-ext.cnf -CAcreateserial

# Set proper permissions
chmod 600 ca/ca-key.pem server/server-key.pem client/client-key.pem
chmod 644 ca/ca-cert.pem server/server-cert.pem client/client-cert.pem

echo "Certificates generated successfully in $CA_DIR"
```

**Certificate Rotation Script:**
```bash
#!/bin/bash
# rotate_certs.sh - Zero-downtime certificate rotation

set -euo pipefail

CA_DIR="/etc/mister-smith/certs"
BACKUP_DIR="/etc/mister-smith/certs/backup/$(date +%Y%m%d_%H%M%S)"

# Create backup
mkdir -p $BACKUP_DIR
cp -r $CA_DIR/* $BACKUP_DIR/

# Check certificate expiration (warn at 30 days)
check_expiration() {
    local cert_file=$1
    local threshold_days=30
    
    expiry_date=$(openssl x509 -in $cert_file -noout -enddate | cut -d= -f2)
    expiry_timestamp=$(date -d "$expiry_date" +%s)
    current_timestamp=$(date +%s)
    days_until_expiry=$(( (expiry_timestamp - current_timestamp) / 86400 ))
    
    if [ $days_until_expiry -le $threshold_days ]; then
        echo "WARNING: Certificate $cert_file expires in $days_until_expiry days"
        return 1
    fi
    return 0
}

# Rotate server certificate
rotate_server_cert() {
    echo "Rotating server certificate..."
    
    # Generate new server key and certificate
    openssl genrsa -out $CA_DIR/server/server-key-new.pem 4096
    openssl req -new -key $CA_DIR/server/server-key-new.pem -out $CA_DIR/server/server-new.csr \
        -subj "/C=US/ST=CA/L=San Francisco/O=Mister Smith Framework/OU=Services/CN=mister-smith.local"
    
    openssl x509 -req -in $CA_DIR/server/server-new.csr -CA $CA_DIR/ca/ca-cert.pem \
        -CAkey $CA_DIR/ca/ca-key.pem -out $CA_DIR/server/server-cert-new.pem \
        -days 90 -extensions v3_req -extfile $CA_DIR/server/server-ext.cnf -CAcreateserial
    
    # Atomic replacement
    mv $CA_DIR/server/server-cert-new.pem $CA_DIR/server/server-cert.pem
    mv $CA_DIR/server/server-key-new.pem $CA_DIR/server/server-key.pem
    
    # Send SIGHUP to services for hot reload
    systemctl reload mister-smith-api
    systemctl reload nats-server
    
    echo "Server certificate rotated successfully"
}

# Check and rotate if needed
if ! check_expiration $CA_DIR/server/server-cert.pem; then
    rotate_server_cert
fi
```

### 1.2 Rustls Certificate Management Implementation

**Complete Certificate Manager:**
```rust
// certificate_manager.rs
use rustls::{Certificate, PrivateKey, ServerConfig, ClientConfig};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use tokio::time::{Duration, interval};
use tracing::{info, warn, error};
use anyhow::{Result, Context};

#[derive(Clone)]
pub struct CertificateManager {
    ca_cert_path: String,
    server_cert_path: String,
    server_key_path: String,
    client_cert_path: String,
    client_key_path: String,
}

impl CertificateManager {
    pub fn new() -> Self {
        Self {
            ca_cert_path: "/etc/mister-smith/certs/ca/ca-cert.pem".to_string(),
            server_cert_path: "/etc/mister-smith/certs/server/server-cert.pem".to_string(),
            server_key_path: "/etc/mister-smith/certs/server/server-key.pem".to_string(),
            client_cert_path: "/etc/mister-smith/certs/client/client-cert.pem".to_string(),
            client_key_path: "/etc/mister-smith/certs/client/client-key.pem".to_string(),
        }
    }

    /// Load certificates from PEM files
    pub fn load_certificates(&self, path: &str) -> Result<Vec<Certificate>> {
        let file = File::open(path)
            .with_context(|| format!("Failed to open certificate file: {}", path))?;
        let mut reader = BufReader::new(file);
        
        let certs = certs(&mut reader)
            .with_context(|| "Failed to parse certificates")?
            .into_iter()
            .map(Certificate)
            .collect();

        if certs.is_empty() {
            anyhow::bail!("No certificates found in file: {}", path);
        }

        info!("Loaded {} certificates from {}", certs.len(), path);
        Ok(certs)
    }

    /// Load private key from PEM file
    pub fn load_private_key(&self, path: &str) -> Result<PrivateKey> {
        let file = File::open(path)
            .with_context(|| format!("Failed to open private key file: {}", path))?;
        let mut reader = BufReader::new(file);

        // Try PKCS8 format first
        if let Ok(mut keys) = pkcs8_private_keys(&mut reader) {
            if !keys.is_empty() {
                info!("Loaded PKCS8 private key from {}", path);
                return Ok(PrivateKey(keys.remove(0)));
            }
        }

        // Reset reader and try RSA format
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        
        let mut keys = rsa_private_keys(&mut reader)
            .with_context(|| "Failed to parse RSA private key")?;

        if keys.is_empty() {
            anyhow::bail!("No private keys found in file: {}", path);
        }

        info!("Loaded RSA private key from {}", path);
        Ok(PrivateKey(keys.remove(0)))
    }

    /// Create TLS server configuration with mTLS
    pub fn create_server_config(&self) -> Result<Arc<ServerConfig>> {
        let certs = self.load_certificates(&self.server_cert_path)?;
        let key = self.load_private_key(&self.server_key_path)?;
        let ca_certs = self.load_certificates(&self.ca_cert_path)?;

        let mut root_store = rustls::RootCertStore::empty();
        for cert in ca_certs {
            root_store.add(&cert)
                .with_context(|| "Failed to add CA certificate to root store")?;
        }

        let client_cert_verifier = rustls::server::AllowAnyAuthenticatedClient::new(root_store);

        let config = ServerConfig::builder()
            .with_cipher_suites(&[
                rustls::cipher_suite::TLS13_AES_256_GCM_SHA384,
                rustls::cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
                rustls::cipher_suite::TLS13_AES_128_GCM_SHA256,
            ])
            .with_kx_groups(&[
                &rustls::kx_group::X25519,
                &rustls::kx_group::SECP384R1,
                &rustls::kx_group::SECP256R1,
            ])
            .with_protocol_versions(&[&rustls::version::TLS13])
            .with_context(|| "Failed to configure TLS parameters")?
            .with_client_cert_verifier(client_cert_verifier)
            .with_single_cert(certs, key)
            .with_context(|| "Failed to configure server certificate")?;

        info!("Created TLS server configuration with mTLS");
        Ok(Arc::new(config))
    }

    /// Create TLS client configuration
    pub fn create_client_config(&self) -> Result<Arc<ClientConfig>> {
        let certs = self.load_certificates(&self.client_cert_path)?;
        let key = self.load_private_key(&self.client_key_path)?;
        let ca_certs = self.load_certificates(&self.ca_cert_path)?;

        let mut root_store = rustls::RootCertStore::empty();
        for cert in ca_certs {
            root_store.add(&cert)
                .with_context(|| "Failed to add CA certificate to root store")?;
        }

        let config = ClientConfig::builder()
            .with_cipher_suites(&[
                rustls::cipher_suite::TLS13_AES_256_GCM_SHA384,
                rustls::cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
                rustls::cipher_suite::TLS13_AES_128_GCM_SHA256,
            ])
            .with_kx_groups(&[
                &rustls::kx_group::X25519,
                &rustls::kx_group::SECP384R1,
                &rustls::kx_group::SECP256R1,
            ])
            .with_protocol_versions(&[&rustls::version::TLS13])
            .with_context(|| "Failed to configure TLS parameters")?
            .with_root_certificates(root_store)
            .with_single_cert(certs, key)
            .with_context(|| "Failed to configure client certificate")?;

        info!("Created TLS client configuration");
        Ok(Arc::new(config))
    }

    /// Check certificate expiration
    pub fn check_certificate_expiration(&self, cert_path: &str) -> Result<Duration> {
        use x509_parser::prelude::*;
        
        let cert_data = std::fs::read(cert_path)
            .with_context(|| format!("Failed to read certificate: {}", cert_path))?;
            
        let pem = pem::parse(&cert_data)
            .with_context(|| "Failed to parse PEM certificate")?;
            
        let x509 = X509Certificate::from_der(&pem.contents)
            .with_context(|| "Failed to parse X509 certificate")?;

        let expiry_time = x509.1.validity().not_after.timestamp() as u64;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if expiry_time <= current_time {
            anyhow::bail!("Certificate has expired: {}", cert_path);
        }

        let remaining = Duration::from_secs(expiry_time - current_time);
        
        if remaining.as_secs() < 30 * 24 * 60 * 60 { // 30 days
            warn!("Certificate expires in {} days: {}", 
                remaining.as_secs() / (24 * 60 * 60), cert_path);
        }

        Ok(remaining)
    }

    /// Start certificate monitoring task
    pub async fn start_monitoring(&self) {
        let manager = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_hours(24));
            
            loop {
                interval.tick().await;
                
                // Check server certificate expiration
                if let Err(e) = manager.check_certificate_expiration(&manager.server_cert_path) {
                    error!("Server certificate check failed: {}", e);
                }
                
                // Check client certificate expiration
                if let Err(e) = manager.check_certificate_expiration(&manager.client_cert_path) {
                    error!("Client certificate check failed: {}", e);
                }
            }
        });
        
        info!("Started certificate monitoring task");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_certificate_manager() {
        let temp_dir = TempDir::new().unwrap();
        // Add comprehensive tests for certificate operations
    }
}
```

## 2. JWT Authentication Implementation

### 2.1 JWT Service Implementation

**Complete JWT Authentication Service:**
```rust
// jwt_service.rs
use jwt_simple::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use anyhow::{Result, Context};
use tracing::{info, warn, error};
use uuid::Uuid;

/// Standard security parameters - DO NOT MODIFY
const ACCESS_TOKEN_DURATION: u64 = 15 * 60; // 15 minutes
const REFRESH_TOKEN_DURATION: u64 = 7 * 24 * 60 * 60; // 7 days
const API_KEY_DURATION: u64 = 90 * 24 * 60 * 60; // 90 days

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "readonly")]
    ReadOnly,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "moderator")]
    Moderator,
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "system")]
    System,
}

impl Role {
    pub fn permissions(&self) -> Vec<&'static str> {
        match self {
            Role::ReadOnly => vec!["read:own"],
            Role::User => vec!["read:own", "write:own"],
            Role::Moderator => vec!["read:own", "write:own", "read:team", "write:team"],
            Role::Admin => vec!["read:*", "write:*", "delete:*"],
            Role::System => vec!["read:*", "write:*", "delete:*", "admin:*"],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserClaims {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub roles: Vec<Role>,
    pub permissions: Vec<String>,
    pub token_type: TokenType,
    pub session_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenType {
    #[serde(rename = "access")]
    Access,
    #[serde(rename = "refresh")]
    Refresh,
    #[serde(rename = "api_key")]
    ApiKey,
}

pub struct JwtService {
    access_key: ES384KeyPair,
    refresh_key: ES384KeyPair,
    api_key: ES384KeyPair,
    issuer: String,
    audience: String,
}

impl JwtService {
    pub fn new() -> Result<Self> {
        // Generate separate key pairs for different token types
        let access_key = ES384KeyPair::generate();
        let refresh_key = ES384KeyPair::generate();
        let api_key = ES384KeyPair::generate();

        Ok(Self {
            access_key,
            refresh_key,
            api_key,
            issuer: "mister-smith-framework".to_string(),
            audience: "mister-smith-services".to_string(),
        })
    }

    /// Generate access token (15 minutes expiration)
    pub fn generate_access_token(&self, user_id: Uuid, tenant_id: Uuid, roles: Vec<Role>) -> Result<String> {
        let permissions = roles.iter()
            .flat_map(|role| role.permissions())
            .map(|p| p.to_string())
            .collect();

        let user_claims = UserClaims {
            user_id,
            tenant_id,
            roles,
            permissions,
            token_type: TokenType::Access,
            session_id: Uuid::new_v4(),
        };

        let claims = Claims::with_custom_claims(user_claims, Duration::from_secs(ACCESS_TOKEN_DURATION))
            .with_issuer(&self.issuer)
            .with_audience(&self.audience)
            .with_subject(&user_id.to_string());

        let token = self.access_key.sign(claims)
            .with_context(|| "Failed to sign access token")?;

        info!("Generated access token for user: {}", user_id);
        Ok(token)
    }

    /// Generate refresh token (7 days expiration)
    pub fn generate_refresh_token(&self, user_id: Uuid, tenant_id: Uuid, session_id: Uuid) -> Result<String> {
        let user_claims = UserClaims {
            user_id,
            tenant_id,
            roles: vec![], // Refresh tokens don't carry permissions
            permissions: vec![],
            token_type: TokenType::Refresh,
            session_id,
        };

        let claims = Claims::with_custom_claims(user_claims, Duration::from_secs(REFRESH_TOKEN_DURATION))
            .with_issuer(&self.issuer)
            .with_audience(&self.audience)
            .with_subject(&user_id.to_string());

        let token = self.refresh_key.sign(claims)
            .with_context(|| "Failed to sign refresh token")?;

        info!("Generated refresh token for user: {}", user_id);
        Ok(token)
    }

    /// Generate API key (90 days expiration)
    pub fn generate_api_key(&self, user_id: Uuid, tenant_id: Uuid, roles: Vec<Role>) -> Result<String> {
        let permissions = roles.iter()
            .flat_map(|role| role.permissions())
            .map(|p| p.to_string())
            .collect();

        let user_claims = UserClaims {
            user_id,
            tenant_id,
            roles,
            permissions,
            token_type: TokenType::ApiKey,
            session_id: Uuid::new_v4(),
        };

        let claims = Claims::with_custom_claims(user_claims, Duration::from_secs(API_KEY_DURATION))
            .with_issuer(&self.issuer)
            .with_audience(&self.audience)
            .with_subject(&user_id.to_string());

        let token = self.api_key.sign(claims)
            .with_context(|| "Failed to sign API key")?;

        info!("Generated API key for user: {}", user_id);
        Ok(token)
    }

    /// Verify access token
    pub fn verify_access_token(&self, token: &str) -> Result<Claims<UserClaims>> {
        let public_key = self.access_key.public_key();
        
        let mut options = VerificationOptions::default();
        options.allowed_issuers = Some(HashSet::from([self.issuer.clone()]));
        options.allowed_audiences = Some(HashSet::from([self.audience.clone()]));

        let claims = public_key.verify_token::<UserClaims>(token, Some(options))
            .with_context(|| "Failed to verify access token")?;

        // Verify token type
        if !matches!(claims.custom.token_type, TokenType::Access) {
            anyhow::bail!("Invalid token type for access token");
        }

        Ok(claims)
    }

    /// Verify refresh token
    pub fn verify_refresh_token(&self, token: &str) -> Result<Claims<UserClaims>> {
        let public_key = self.refresh_key.public_key();
        
        let mut options = VerificationOptions::default();
        options.allowed_issuers = Some(HashSet::from([self.issuer.clone()]));
        options.allowed_audiences = Some(HashSet::from([self.audience.clone()]));

        let claims = public_key.verify_token::<UserClaims>(token, Some(options))
            .with_context(|| "Failed to verify refresh token")?;

        // Verify token type
        if !matches!(claims.custom.token_type, TokenType::Refresh) {
            anyhow::bail!("Invalid token type for refresh token");
        }

        Ok(claims)
    }

    /// Verify API key
    pub fn verify_api_key(&self, token: &str) -> Result<Claims<UserClaims>> {
        let public_key = self.api_key.public_key();
        
        let mut options = VerificationOptions::default();
        options.allowed_issuers = Some(HashSet::from([self.issuer.clone()]));
        options.allowed_audiences = Some(HashSet::from([self.audience.clone()]));
        // API keys have longer validity, so allow more clock skew
        options.time_tolerance = Some(Duration::from_mins(30));

        let claims = public_key.verify_token::<UserClaims>(token, Some(options))
            .with_context(|| "Failed to verify API key")?;

        // Verify token type
        if !matches!(claims.custom.token_type, TokenType::ApiKey) {
            anyhow::bail!("Invalid token type for API key");
        }

        Ok(claims)
    }

    /// Check if user has permission for resource and action
    pub fn check_permission(&self, claims: &Claims<UserClaims>, resource: &str, action: &str) -> bool {
        let required_permission = format!("{}:{}", action, resource);
        let wildcard_permission = format!("{}:*", action);
        let super_wildcard = "admin:*";

        claims.custom.permissions.iter().any(|perm| {
            perm == &required_permission || 
            perm == &wildcard_permission || 
            perm == super_wildcard
        })
    }

    /// Refresh access token using refresh token
    pub fn refresh_access_token(&self, refresh_token: &str, new_roles: Option<Vec<Role>>) -> Result<String> {
        let refresh_claims = self.verify_refresh_token(refresh_token)?;
        
        // Use provided roles or fetch from user store
        let roles = new_roles.unwrap_or_else(|| vec![Role::User]);
        
        self.generate_access_token(
            refresh_claims.custom.user_id,
            refresh_claims.custom.tenant_id,
            roles
        )
    }
}

/// JWT Authentication Middleware
pub struct JwtMiddleware {
    jwt_service: JwtService,
}

impl JwtMiddleware {
    pub fn new(jwt_service: JwtService) -> Self {
        Self { jwt_service }
    }

    /// Extract and verify JWT from Authorization header
    pub fn authenticate_request(&self, auth_header: Option<&str>) -> Result<Claims<UserClaims>> {
        let auth_header = auth_header
            .ok_or_else(|| anyhow::anyhow!("Missing Authorization header"))?;

        let token = auth_header.strip_prefix("Bearer ")
            .ok_or_else(|| anyhow::anyhow!("Invalid Authorization header format"))?;

        // Try access token first
        if let Ok(claims) = self.jwt_service.verify_access_token(token) {
            return Ok(claims);
        }

        // Try API key if access token fails
        self.jwt_service.verify_api_key(token)
            .with_context(|| "Invalid or expired token")
    }

    /// Check authorization for specific resource and action
    pub fn authorize_request(&self, claims: &Claims<UserClaims>, resource: &str, action: &str) -> Result<()> {
        if !self.jwt_service.check_permission(claims, resource, action) {
            anyhow::bail!("Insufficient permissions for {} on {}", action, resource);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jwt_lifecycle() {
        let jwt_service = JwtService::new().unwrap();
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let roles = vec![Role::User];

        // Test access token generation and verification
        let access_token = jwt_service.generate_access_token(user_id, tenant_id, roles.clone()).unwrap();
        let claims = jwt_service.verify_access_token(&access_token).unwrap();
        assert_eq!(claims.custom.user_id, user_id);

        // Test permission checking
        assert!(jwt_service.check_permission(&claims, "own", "read"));
        assert!(!jwt_service.check_permission(&claims, "all", "delete"));
    }
}
```

## Navigation and Cross-References

### Related Security Documents
- **[Security Patterns](security-patterns.md)** - Foundational security patterns and guidelines
- **[Authorization Implementation](authorization-implementation.md)** - RBAC and security audit implementation
- **[Security Integration](security-integration.md)** - NATS and hook security implementation
- **[Security Framework](security-framework.md)** - Complete security patterns and configurations
- **[Authentication Specifications](authentication-specifications.md)** - Authentication requirements and specifications
- **[Authorization Specifications](authorization-specifications.md)** - Authorization patterns and RBAC specifications

### Implementation Integration Points
1. **Certificate Management** → Integrates with TLS transport layer and NATS mTLS configuration
2. **JWT Authentication** → Provides authentication for HTTP APIs and NATS messaging
3. **Cross-Component Security** → Foundation for authorization middleware and audit logging
4. **Transport Security** → Integrates with [NATS Transport](../transport/nats-transport.md) for secure messaging
5. **Data Management** → Secures persistence operations in [Data Management](../data-management/) components

### Next Steps
1. Review authorization implementation patterns in security-framework.md (Section 3+)
2. Integrate certificate management with transport layer implementations
3. Configure JWT authentication middleware in HTTP services
4. Set up certificate monitoring and rotation automation

---

*Agent 20 - Phase 1, Group 1C - Framework Modularization Operation*
*Extracted from security-framework.md sections 1-2*
*Zero information loss mandate maintained*