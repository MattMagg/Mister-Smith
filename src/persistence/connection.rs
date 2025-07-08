//! Database connection pool management for MisterSmith
//! 
//! Provides async PostgreSQL connection pooling with health checks and retry logic.

use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{sync::Arc, time::Duration};
use tracing::{debug, error, info, warn};

/// Database connection configuration
#[derive(Debug, Clone)]
pub struct DbConfig {
    /// PostgreSQL connection URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of idle connections to maintain
    pub min_connections: u32,
    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,
    /// Idle timeout in seconds before closing a connection
    pub idle_timeout_secs: u64,
    /// Maximum lifetime of a connection in seconds
    pub max_lifetime_secs: u64,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost/mistersmith".to_string(),
            max_connections: 30,
            min_connections: 5,
            connect_timeout_secs: 10,
            idle_timeout_secs: 300,
            max_lifetime_secs: 3600,
        }
    }
}

/// Database connection pool wrapper
#[derive(Clone)]
pub struct DbPool {
    pool: Arc<Pool<Postgres>>,
    config: DbConfig,
}

impl DbPool {
    /// Create a new database connection pool
    pub async fn new(config: DbConfig) -> Result<Self> {
        info!("Initializing database connection pool");
        
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.connect_timeout_secs))
            .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
            .max_lifetime(Duration::from_secs(config.max_lifetime_secs))
            .connect(&config.url)
            .await
            .context("Failed to create database connection pool")?;
        
        // Test the connection
        sqlx::query("SELECT 1")
            .fetch_one(&pool)
            .await
            .context("Failed to verify database connection")?;
        
        info!(
            "Database connection pool initialized with {} connections",
            config.max_connections
        );
        
        Ok(Self {
            pool: Arc::new(pool),
            config,
        })
    }
    
    /// Get a reference to the underlying connection pool
    pub fn pool(&self) -> &Pool<Postgres> {
        &self.pool
    }
    
    /// Check if the database connection is healthy
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&*self.pool)
            .await
            .context("Database health check failed")?;
        Ok(())
    }
    
    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            size: self.pool.size() as u32,
            idle: self.pool.num_idle() as u32,
            max_connections: self.config.max_connections,
        }
    }
    
    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations");
        
        // In production, we'd use sqlx migrate! macro
        // For now, we'll implement a simple migration runner
        let migrations_dir = std::path::Path::new("migrations");
        if !migrations_dir.exists() {
            warn!("Migrations directory not found");
            return Ok(());
        }
        
        // Read and execute migration files in order
        let mut entries: Vec<_> = std::fs::read_dir(migrations_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "sql")
                    .unwrap_or(false)
            })
            .collect();
        
        entries.sort_by_key(|e| e.path());
        
        for entry in entries {
            let path = entry.path();
            let filename = path.file_name().unwrap().to_string_lossy();
            
            debug!("Checking migration: {}", filename);
            
            // Extract version number from filename (e.g., "003_supervision_persistence.sql" -> 3)
            let version: i32 = filename
                .split('_')
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            
            if version == 0 {
                warn!("Skipping migration with invalid version: {}", filename);
                continue;
            }
            
            // Check if migration was already applied
            let applied = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM schema_versions WHERE version = $1)"
            )
            .bind(version)
            .fetch_one(&*self.pool)
            .await
            .unwrap_or(false);
            
            if applied {
                debug!("Migration {} already applied", filename);
                continue;
            }
            
            info!("Applying migration: {}", filename);
            
            let sql = std::fs::read_to_string(&path)
                .context(format!("Failed to read migration file: {:?}", path))?;
            
            // Execute migration in a transaction
            let mut tx = self.pool.begin().await?;
            
            // Split SQL into statements (simple approach, doesn't handle all cases)
            for statement in sql.split(';').filter(|s| !s.trim().is_empty()) {
                sqlx::query(statement)
                    .execute(&mut *tx)
                    .await
                    .context(format!("Failed to execute migration statement in {}", filename))?;
            }
            
            tx.commit()
                .await
                .context(format!("Failed to commit migration {}", filename))?;
            
            info!("Successfully applied migration: {}", filename);
        }
        
        Ok(())
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Current size of the pool
    pub size: u32,
    /// Number of idle connections
    pub idle: u32,
    /// Maximum connections configured
    pub max_connections: u32,
}

/// Create a database connection pool with retry logic
pub async fn create_pool_with_retry(
    config: DbConfig,
    max_retries: u32,
    retry_delay_ms: u64,
) -> Result<DbPool> {
    let mut attempts = 0;
    
    loop {
        match DbPool::new(config.clone()).await {
            Ok(pool) => return Ok(pool),
            Err(e) => {
                attempts += 1;
                if attempts >= max_retries {
                    error!("Failed to create database pool after {} attempts", attempts);
                    return Err(e);
                }
                
                warn!(
                    "Database connection attempt {} failed: {}. Retrying in {}ms...",
                    attempts, e, retry_delay_ms
                );
                
                tokio::time::sleep(Duration::from_millis(retry_delay_ms)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = DbConfig::default();
        assert_eq!(config.max_connections, 30);
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.connect_timeout_secs, 10);
    }
    
    #[test]
    fn test_pool_stats() {
        // This would require a real database connection to test properly
        // For now, just verify the struct can be created
        let stats = PoolStats {
            size: 10,
            idle: 5,
            max_connections: 30,
        };
        assert_eq!(stats.size, 10);
        assert_eq!(stats.idle, 5);
    }
}