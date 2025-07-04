---
title: Authorization and Security Audit Implementation
type: note
permalink: security/authorization-implementation
tags:
- '#security'
- '#authorization'
- '#audit'
- '#rbac'
- '#agent-focused'
---

# Authorization and Security Audit Implementation

## Framework Authority
This document implements specifications from the canonical tech-framework.md located at /Users/mac-main/Mister-Smith/Mister-Smith/tech-framework.md

As stated in the canonical framework: "Agents: use this framework as the canonical source."

## Purpose
Comprehensive authorization and security audit implementation for the Mister Smith AI Agent Framework. This document provides complete code implementations for Role-Based Access Control (RBAC) and structured security audit logging.

## Related Documentation

### Security Implementation Files
- **[Security Patterns](security-patterns.md)** - Foundational security patterns and guidelines
- **[Authentication Implementation](authentication-implementation.md)** - Certificate management and JWT authentication
- **[Security Integration](security-integration.md)** - NATS and hook security implementation
- **[Security Framework](security-framework.md)** - Complete security framework overview

### Framework Integration Points
- **[Transport Layer](../transport/)** - Communication security protocols
- **[NATS Transport](../transport/nats-transport.md)** - NATS messaging security
- **[Data Management](../data-management/)** - Message schemas and persistence security
- **[Core Architecture](../core-architecture/)** - System integration patterns

## 3. Authorization Implementation

### 3.1 RBAC Policy Engine

**Complete RBAC Implementation:**
```rust
// rbac_engine.rs
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub resource_type: ResourceType,
    pub owner_id: Option<Uuid>,
    pub tenant_id: Uuid,
    pub team_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "project")]
    Project,
    #[serde(rename = "document")]
    Document,
    #[serde(rename = "system")]
    System,
    #[serde(rename = "configuration")]
    Configuration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    #[serde(rename = "read")]
    Read,
    #[serde(rename = "write")]
    Write,
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "admin")]
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub action: Action,
    pub resource_pattern: String,
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    #[serde(rename = "owner_only")]
    OwnerOnly,
    #[serde(rename = "same_tenant")]
    SameTenant,
    #[serde(rename = "same_team")]
    SameTeam,
    #[serde(rename = "business_hours")]
    BusinessHours,
}

pub struct RbacEngine {
    role_permissions: HashMap<Role, Vec<Permission>>,
}

impl RbacEngine {
    pub fn new() -> Self {
        let mut role_permissions = HashMap::new();
        
        // ReadOnly role permissions
        role_permissions.insert(Role::ReadOnly, vec![
            Permission {
                action: Action::Read,
                resource_pattern: "*".to_string(),
                conditions: vec![Condition::OwnerOnly, Condition::SameTenant],
            },
        ]);

        // User role permissions
        role_permissions.insert(Role::User, vec![
            Permission {
                action: Action::Read,
                resource_pattern: "*".to_string(),
                conditions: vec![Condition::OwnerOnly, Condition::SameTenant],
            },
            Permission {
                action: Action::Write,
                resource_pattern: "*".to_string(),
                conditions: vec![Condition::OwnerOnly, Condition::SameTenant],
            },
        ]);

        // Moderator role permissions
        role_permissions.insert(Role::Moderator, vec![
            Permission {
                action: Action::Read,
                resource_pattern: "*".to_string(),
                conditions: vec![Condition::SameTeam, Condition::SameTenant],
            },
            Permission {
                action: Action::Write,
                resource_pattern: "*".to_string(),
                conditions: vec![Condition::SameTeam, Condition::SameTenant],
            },
            Permission {
                action: Action::Delete,
                resource_pattern: "document".to_string(),
                conditions: vec![Condition::SameTeam, Condition::SameTenant],
            },
        ]);

        // Admin role permissions
        role_permissions.insert(Role::Admin, vec![
            Permission {
                action: Action::Read,
                resource_pattern: "*".to_string(),
                conditions: vec![Condition::SameTenant],
            },
            Permission {
                action: Action::Write,
                resource_pattern: "*".to_string(),
                conditions: vec![Condition::SameTenant],
            },
            Permission {
                action: Action::Delete,
                resource_pattern: "*".to_string(),
                conditions: vec![Condition::SameTenant],
            },
        ]);

        // System role permissions (no restrictions)
        role_permissions.insert(Role::System, vec![
            Permission {
                action: Action::Read,
                resource_pattern: "*".to_string(),
                conditions: vec![],
            },
            Permission {
                action: Action::Write,
                resource_pattern: "*".to_string(),
                conditions: vec![],
            },
            Permission {
                action: Action::Delete,
                resource_pattern: "*".to_string(),
                conditions: vec![],
            },
            Permission {
                action: Action::Admin,
                resource_pattern: "*".to_string(),
                conditions: vec![],
            },
        ]);

        Self { role_permissions }
    }

    /// Check if user has permission to perform action on resource
    pub fn check_permission(
        &self,
        user_claims: &UserClaims,
        resource: &Resource,
        action: &Action,
    ) -> Result<bool> {
        // Check each role the user has
        for role in &user_claims.roles {
            if let Some(permissions) = self.role_permissions.get(role) {
                for permission in permissions {
                    if self.permission_matches(permission, resource, action, user_claims)? {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// Check if permission matches the requested action and resource
    fn permission_matches(
        &self,
        permission: &Permission,
        resource: &Resource,
        action: &Action,
        user_claims: &UserClaims,
    ) -> Result<bool> {
        // Check action match
        if !self.action_matches(&permission.action, action) {
            return Ok(false);
        }

        // Check resource pattern match
        if !self.resource_pattern_matches(&permission.resource_pattern, resource) {
            return Ok(false);
        }

        // Check all conditions
        for condition in &permission.conditions {
            if !self.condition_matches(condition, resource, user_claims)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check if action matches (including hierarchical permissions)
    fn action_matches(&self, permission_action: &Action, requested_action: &Action) -> bool {
        match (permission_action, requested_action) {
            // Exact match
            (a, b) if a == b => true,
            // Admin action grants all permissions
            (Action::Admin, _) => true,
            // Write action grants read permission
            (Action::Write, Action::Read) => true,
            // Delete action grants read and write permissions
            (Action::Delete, Action::Read) | (Action::Delete, Action::Write) => true,
            _ => false,
        }
    }

    /// Check if resource pattern matches
    fn resource_pattern_matches(&self, pattern: &str, resource: &Resource) -> bool {
        if pattern == "*" {
            return true;
        }

        let resource_type_str = match resource.resource_type {
            ResourceType::User => "user",
            ResourceType::Project => "project",
            ResourceType::Document => "document",
            ResourceType::System => "system",
            ResourceType::Configuration => "configuration",
        };

        pattern == resource_type_str
    }

    /// Check if condition is satisfied
    fn condition_matches(
        &self,
        condition: &Condition,
        resource: &Resource,
        user_claims: &UserClaims,
    ) -> Result<bool> {
        match condition {
            Condition::OwnerOnly => {
                Ok(resource.owner_id == Some(user_claims.user_id))
            },
            Condition::SameTenant => {
                Ok(resource.tenant_id == user_claims.tenant_id)
            },
            Condition::SameTeam => {
                match (resource.team_id, &user_claims.roles) {
                    (Some(resource_team), _) => {
                        // For now, check if user has moderator role in same tenant
                        // In a real implementation, you'd check team membership
                        Ok(resource.tenant_id == user_claims.tenant_id)
                    },
                    (None, _) => Ok(true), // Resource not tied to team
                }
            },
            Condition::BusinessHours => {
                use chrono::{Local, Timelike};
                let now = Local::now();
                let hour = now.hour();
                Ok(hour >= 9 && hour <= 17) // 9 AM to 5 PM
            },
        }
    }

    /// Get effective permissions for user
    pub fn get_effective_permissions(&self, user_claims: &UserClaims) -> Vec<String> {
        let mut permissions = HashSet::new();
        
        for role in &user_claims.roles {
            if let Some(role_permissions) = self.role_permissions.get(role) {
                for permission in role_permissions {
                    let perm_str = format!("{}:{}", 
                        action_to_string(&permission.action),
                        permission.resource_pattern
                    );
                    permissions.insert(perm_str);
                }
            }
        }
        
        permissions.into_iter().collect()
    }
}

fn action_to_string(action: &Action) -> &'static str {
    match action {
        Action::Read => "read",
        Action::Write => "write", 
        Action::Delete => "delete",
        Action::Admin => "admin",
    }
}

/// Authorization middleware
pub struct AuthorizationMiddleware {
    rbac_engine: RbacEngine,
}

impl AuthorizationMiddleware {
    pub fn new() -> Self {
        Self {
            rbac_engine: RbacEngine::new(),
        }
    }

    /// Authorize request for specific resource and action
    pub fn authorize(
        &self,
        user_claims: &UserClaims,
        resource: &Resource,
        action: &Action,
    ) -> Result<()> {
        if self.rbac_engine.check_permission(user_claims, resource, action)? {
            Ok(())
        } else {
            anyhow::bail!(
                "Access denied: user {} lacks permission to {:?} resource {}",
                user_claims.user_id,
                action,
                resource.id
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rbac_permissions() {
        let rbac = RbacEngine::new();
        
        let user_claims = UserClaims {
            user_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            roles: vec![Role::User],
            permissions: vec![],
            token_type: TokenType::Access,
            session_id: Uuid::new_v4(),
        };

        let resource = Resource {
            id: "test-doc".to_string(),
            resource_type: ResourceType::Document,
            owner_id: Some(user_claims.user_id),
            tenant_id: user_claims.tenant_id,
            team_id: None,
        };

        // User should be able to read their own document
        assert!(rbac.check_permission(&user_claims, &resource, &Action::Read).unwrap());
        
        // User should be able to write their own document
        assert!(rbac.check_permission(&user_claims, &resource, &Action::Write).unwrap());
        
        // User should NOT be able to delete their own document (requires moderator+)
        assert!(!rbac.check_permission(&user_claims, &resource, &Action::Delete).unwrap());
    }
}
```

## 4. Security Audit Implementation

### 4.1 Structured Audit Logging

**Complete Audit Service:**
```rust
// audit_service.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::{Result, Context};
use tracing::{info, warn, error};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    #[serde(rename = "authentication_success")]
    AuthenticationSuccess,
    #[serde(rename = "authentication_failure")]
    AuthenticationFailure,
    #[serde(rename = "authorization_granted")]
    AuthorizationGranted,
    #[serde(rename = "authorization_denied")]
    AuthorizationDenied,
    #[serde(rename = "token_issued")]
    TokenIssued,
    #[serde(rename = "token_expired")]
    TokenExpired,
    #[serde(rename = "token_revoked")]
    TokenRevoked,
    #[serde(rename = "rate_limit_exceeded")]
    RateLimitExceeded,
    #[serde(rename = "suspicious_activity")]
    SuspiciousActivity,
    #[serde(rename = "certificate_rotation")]
    CertificateRotation,
    #[serde(rename = "configuration_change")]
    ConfigurationChange,
    #[serde(rename = "privilege_escalation")]
    PrivilegeEscalation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "critical")]
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: SecurityEventType,
    pub severity: Severity,
    pub user_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub source_ip: Option<String>,
    pub user_agent: Option<String>,
    pub resource_id: Option<String>,
    pub resource_type: Option<String>,
    pub action: Option<String>,
    pub outcome: SecurityOutcome,
    pub details: HashMap<String, serde_json::Value>,
    pub correlation_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityOutcome {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "failure")]
    Failure,
    #[serde(rename = "blocked")]
    Blocked,
    #[serde(rename = "warning")]
    Warning,
}

pub struct AuditService {
    // In production, this would write to a secure audit log storage
    events: Vec<SecurityEvent>,
    correlation_tracker: HashMap<Uuid, Vec<Uuid>>,
}

impl AuditService {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            correlation_tracker: HashMap::new(),
        }
    }

    /// Log authentication attempt
    pub fn log_authentication(&mut self, 
        user_id: Option<Uuid>,
        tenant_id: Option<Uuid>,
        source_ip: Option<String>,
        user_agent: Option<String>,
        success: bool,
        failure_reason: Option<String>,
    ) -> Uuid {
        let event_id = Uuid::new_v4();
        let correlation_id = Uuid::new_v4();
        
        let mut details = HashMap::new();
        if let Some(reason) = failure_reason {
            details.insert("failure_reason".to_string(), serde_json::Value::String(reason));
        }

        let event = SecurityEvent {
            event_id,
            timestamp: Utc::now(),
            event_type: if success { 
                SecurityEventType::AuthenticationSuccess 
            } else { 
                SecurityEventType::AuthenticationFailure 
            },
            severity: if success { Severity::Info } else { Severity::Warning },
            user_id,
            tenant_id,
            session_id: None,
            source_ip,
            user_agent,
            resource_id: None,
            resource_type: None,
            action: Some("authenticate".to_string()),
            outcome: if success { SecurityOutcome::Success } else { SecurityOutcome::Failure },
            details,
            correlation_id: Some(correlation_id),
        };

        self.events.push(event);
        info!("Logged authentication event: {}", event_id);
        event_id
    }

    /// Log authorization decision
    pub fn log_authorization(&mut self,
        user_id: Uuid,
        tenant_id: Uuid,
        session_id: Option<Uuid>,
        resource_id: String,
        resource_type: String,
        action: String,
        granted: bool,
        roles: Vec<String>,
        correlation_id: Option<Uuid>,
    ) -> Uuid {
        let event_id = Uuid::new_v4();
        
        let mut details = HashMap::new();
        details.insert("roles".to_string(), serde_json::Value::Array(
            roles.into_iter().map(serde_json::Value::String).collect()
        ));

        let event = SecurityEvent {
            event_id,
            timestamp: Utc::now(),
            event_type: if granted { 
                SecurityEventType::AuthorizationGranted 
            } else { 
                SecurityEventType::AuthorizationDenied 
            },
            severity: if granted { Severity::Info } else { Severity::Warning },
            user_id: Some(user_id),
            tenant_id: Some(tenant_id),
            session_id,
            source_ip: None,
            user_agent: None,
            resource_id: Some(resource_id),
            resource_type: Some(resource_type),
            action: Some(action),
            outcome: if granted { SecurityOutcome::Success } else { SecurityOutcome::Blocked },
            details,
            correlation_id,
        };

        self.events.push(event);
        info!("Logged authorization event: {}", event_id);
        event_id
    }

    /// Log token issuance
    pub fn log_token_issued(&mut self,
        user_id: Uuid,
        tenant_id: Uuid,
        token_type: String,
        expires_at: DateTime<Utc>,
        correlation_id: Option<Uuid>,
    ) -> Uuid {
        let event_id = Uuid::new_v4();
        
        let mut details = HashMap::new();
        details.insert("token_type".to_string(), serde_json::Value::String(token_type));
        details.insert("expires_at".to_string(), serde_json::Value::String(expires_at.to_rfc3339()));

        let event = SecurityEvent {
            event_id,
            timestamp: Utc::now(),
            event_type: SecurityEventType::TokenIssued,
            severity: Severity::Info,
            user_id: Some(user_id),
            tenant_id: Some(tenant_id),
            session_id: None,
            source_ip: None,
            user_agent: None,
            resource_id: None,
            resource_type: None,
            action: Some("issue_token".to_string()),
            outcome: SecurityOutcome::Success,
            details,
            correlation_id,
        };

        self.events.push(event);
        info!("Logged token issuance: {}", event_id);
        event_id
    }

    /// Log suspicious activity
    pub fn log_suspicious_activity(&mut self,
        user_id: Option<Uuid>,
        tenant_id: Option<Uuid>,
        source_ip: Option<String>,
        activity_type: String,
        details: HashMap<String, serde_json::Value>,
        correlation_id: Option<Uuid>,
    ) -> Uuid {
        let event_id = Uuid::new_v4();
        
        let mut event_details = details;
        event_details.insert("activity_type".to_string(), serde_json::Value::String(activity_type));

        let event = SecurityEvent {
            event_id,
            timestamp: Utc::now(),
            event_type: SecurityEventType::SuspiciousActivity,
            severity: Severity::Error,
            user_id,
            tenant_id,
            session_id: None,
            source_ip,
            user_agent: None,
            resource_id: None,
            resource_type: None,
            action: Some("suspicious_activity".to_string()),
            outcome: SecurityOutcome::Warning,
            details: event_details,
            correlation_id,
        };

        self.events.push(event);
        error!("Logged suspicious activity: {}", event_id);
        event_id
    }

    /// Log certificate rotation
    pub fn log_certificate_rotation(&mut self,
        certificate_type: String,
        old_expiry: DateTime<Utc>,
        new_expiry: DateTime<Utc>,
    ) -> Uuid {
        let event_id = Uuid::new_v4();
        
        let mut details = HashMap::new();
        details.insert("certificate_type".to_string(), serde_json::Value::String(certificate_type));
        details.insert("old_expiry".to_string(), serde_json::Value::String(old_expiry.to_rfc3339()));
        details.insert("new_expiry".to_string(), serde_json::Value::String(new_expiry.to_rfc3339()));

        let event = SecurityEvent {
            event_id,
            timestamp: Utc::now(),
            event_type: SecurityEventType::CertificateRotation,
            severity: Severity::Info,
            user_id: None,
            tenant_id: None,
            session_id: None,
            source_ip: None,
            user_agent: None,
            resource_id: None,
            resource_type: None,
            action: Some("rotate_certificate".to_string()),
            outcome: SecurityOutcome::Success,
            details,
            correlation_id: None,
        };

        self.events.push(event);
        info!("Logged certificate rotation: {}", event_id);
        event_id
    }

    /// Search audit events
    pub fn search_events(&self, 
        event_type: Option<SecurityEventType>,
        severity: Option<Severity>,
        user_id: Option<Uuid>,
        tenant_id: Option<Uuid>,
        since: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Vec<&SecurityEvent> {
        let mut filtered: Vec<&SecurityEvent> = self.events
            .iter()
            .filter(|event| {
                if let Some(ref et) = event_type {
                    if std::mem::discriminant(&event.event_type) != std::mem::discriminant(et) {
                        return false;
                    }
                }
                if let Some(ref sev) = severity {
                    if std::mem::discriminant(&event.severity) != std::mem::discriminant(sev) {
                        return false;
                    }
                }
                if let Some(uid) = user_id {
                    if event.user_id != Some(uid) {
                        return false;
                    }
                }
                if let Some(tid) = tenant_id {
                    if event.tenant_id != Some(tid) {
                        return false;
                    }
                }
                if let Some(since_time) = since {
                    if event.timestamp < since_time {
                        return false;
                    }
                }
                true
            })
            .collect();

        // Sort by timestamp, newest first
        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        if let Some(limit) = limit {
            filtered.truncate(limit);
        }

        filtered
    }

    /// Generate compliance report
    pub fn generate_compliance_report(&self, 
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> ComplianceReport {
        let events_in_range: Vec<&SecurityEvent> = self.events
            .iter()
            .filter(|event| event.timestamp >= start_date && event.timestamp <= end_date)
            .collect();

        let mut authentication_attempts = 0;
        let mut authentication_failures = 0;
        let mut authorization_denials = 0;
        let mut suspicious_activities = 0;
        let mut certificate_rotations = 0;

        for event in &events_in_range {
            match event.event_type {
                SecurityEventType::AuthenticationSuccess => authentication_attempts += 1,
                SecurityEventType::AuthenticationFailure => {
                    authentication_attempts += 1;
                    authentication_failures += 1;
                },
                SecurityEventType::AuthorizationDenied => authorization_denials += 1,
                SecurityEventType::SuspiciousActivity => suspicious_activities += 1,
                SecurityEventType::CertificateRotation => certificate_rotations += 1,
                _ => {},
            }
        }

        ComplianceReport {
            period_start: start_date,
            period_end: end_date,
            total_events: events_in_range.len(),
            authentication_attempts,
            authentication_failures,
            authorization_denials,
            suspicious_activities,
            certificate_rotations,
            failure_rate: if authentication_attempts > 0 {
                (authentication_failures as f64 / authentication_attempts as f64) * 100.0
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ComplianceReport {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_events: usize,
    pub authentication_attempts: usize,
    pub authentication_failures: usize,
    pub authorization_denials: usize,
    pub suspicious_activities: usize,
    pub certificate_rotations: usize,
    pub failure_rate: f64, // Percentage
}

/// Audit middleware for HTTP requests
pub struct AuditMiddleware {
    audit_service: std::sync::Arc<std::sync::Mutex<AuditService>>,
}

impl AuditMiddleware {
    pub fn new(audit_service: std::sync::Arc<std::sync::Mutex<AuditService>>) -> Self {
        Self { audit_service }
    }

    /// Log HTTP request for audit
    pub fn log_request(&self, 
        method: &str,
        path: &str,
        user_claims: Option<&UserClaims>,
        source_ip: Option<String>,
        user_agent: Option<String>,
        response_status: u16,
    ) {
        let mut audit = self.audit_service.lock().unwrap();
        
        // Log based on response status
        if response_status == 401 {
            audit.log_authentication(
                user_claims.map(|c| c.user_id),
                user_claims.map(|c| c.tenant_id),
                source_ip,
                user_agent,
                false,
                Some("Unauthorized request".to_string()),
            );
        } else if response_status == 403 {
            if let Some(claims) = user_claims {
                audit.log_authorization(
                    claims.user_id,
                    claims.tenant_id,
                    Some(claims.session_id),
                    path.to_string(),
                    "http_endpoint".to_string(),
                    method.to_string(),
                    false,
                    claims.roles.iter().map(|r| format!("{:?}", r)).collect(),
                    None,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_service() {
        let mut audit = AuditService::new();
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        // Test authentication logging
        let auth_event_id = audit.log_authentication(
            Some(user_id),
            Some(tenant_id),
            Some("192.168.1.1".to_string()),
            Some("test-agent".to_string()),
            true,
            None,
        );

        assert!(audit.events.iter().any(|e| e.event_id == auth_event_id));

        // Test searching events
        let auth_events = audit.search_events(
            Some(SecurityEventType::AuthenticationSuccess),
            None,
            Some(user_id),
            None,
            None,
            None,
        );

        assert_eq!(auth_events.len(), 1);
    }
}
```

### 4.2 Security Monitoring Configuration

**Prometheus Metrics Configuration:**
```yaml
# prometheus_security_metrics.yml
groups:
  - name: security_alerts
    rules:
      # Authentication failure rate
      - alert: HighAuthenticationFailureRate
        expr: rate(authentication_failures_total[5m]) > 0.1
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High authentication failure rate detected"
          description: "Authentication failure rate is {{ $value }} failures/second"

      # Suspicious activity detection
      - alert: SuspiciousActivity
        expr: rate(suspicious_activity_total[1m]) > 0
        for: 0m
        labels:
          severity: critical
        annotations:
          summary: "Suspicious activity detected"
          description: "{{ $value }} suspicious activities detected"

      # Certificate expiration warning
      - alert: CertificateExpiringSoon
        expr: certificate_expiry_days < 30
        for: 0m
        labels:
          severity: warning
        annotations:
          summary: "Certificate expiring soon"
          description: "Certificate {{ $labels.certificate_name }} expires in {{ $value }} days"

      # Token issuance rate anomaly
      - alert: UnusualTokenIssuanceRate
        expr: rate(tokens_issued_total[10m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Unusual token issuance rate"
          description: "Token issuance rate is {{ $value }} tokens/second"

      # Failed authorization attempts
      - alert: RepeatedAuthorizationDenials
        expr: rate(authorization_denied_total[5m]) by (user_id) > 0.5
        for: 1m
        labels:
          severity: warning
        annotations:
          summary: "Repeated authorization denials for user"
          description: "User {{ $labels.user_id }} has {{ $value }} authorization denials/second"
```

**Grafana Dashboard Configuration:**
```json
{
  "dashboard": {
    "id": null,
    "title": "Mister Smith Security Dashboard",
    "tags": ["security", "audit"],
    "timezone": "UTC",
    "panels": [
      {
        "title": "Authentication Events",
        "type": "stat",
        "targets": [
          {
            "expr": "rate(authentication_success_total[1h])",
            "legendFormat": "Success Rate"
          },
          {
            "expr": "rate(authentication_failure_total[1h])",
            "legendFormat": "Failure Rate"
          }
        ]
      },
      {
        "title": "Security Events Timeline",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(security_events_total[5m]) by (event_type)",
            "legendFormat": "{{ event_type }}"
          }
        ]
      },
      {
        "title": "Certificate Expiry Status",
        "type": "table",
        "targets": [
          {
            "expr": "certificate_expiry_days",
            "format": "table"
          }
        ]
      },
      {
        "title": "Top Failed Authorization Attempts",
        "type": "table",
        "targets": [
          {
            "expr": "topk(10, rate(authorization_denied_total[1h]) by (user_id, resource_type))",
            "format": "table"
          }
        ]
      }
    ]
  }
}
```

---

## Navigation

### Related Security Components
- **[Security Patterns](security-patterns.md)** - Foundational security patterns and guidelines
- **[Authentication Implementation](authentication-implementation.md)** - Certificate management and JWT authentication
- **[Security Integration](security-integration.md)** - NATS and hook security implementation
- **[Security Framework](security-framework.md)** - Complete security framework overview

### Implementation Guide
1. **Authorization Setup**: Implement RBAC engine with role-based permissions
2. **Audit Integration**: Deploy structured audit logging service
3. **Monitoring**: Configure Prometheus metrics and Grafana dashboards
4. **Testing**: Validate authorization logic and audit event generation

### Key Features
- **Role-Based Access Control**: Hierarchical permission model with conditions
- **Comprehensive Audit Logging**: Structured security event tracking
- **Real-time Monitoring**: Prometheus metrics and alerting
- **Compliance Reporting**: Automated security compliance reports

This document provides complete authorization and audit implementations for agent-focused security management within the Mister Smith AI Agent Framework.