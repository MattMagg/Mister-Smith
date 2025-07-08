//! Phase 3 Database Tests
//! 
//! Tests PostgreSQL connectivity and basic persistence operations

use std::time::Duration;

#[tokio::test]
async fn test_01_database_connection() {
    use mistersmith::persistence::{DbConfig, create_pool_with_retry};
    
    println!("🧪 Testing PostgreSQL connection for MisterSmith...");
    
    // Create database configuration with our setup
    let config = DbConfig {
        url: "postgresql://mistersmith_user:mistersmith_dev@localhost/mistersmith".to_string(),
        max_connections: 10,
        min_connections: 1,
        connect_timeout_secs: 5,
        idle_timeout_secs: 600,
        max_lifetime_secs: 1800,
    };
    
    // Test connection pool creation
    match create_pool_with_retry(&config, 3, Duration::from_secs(1)).await {
        Ok(pool) => {
            println!("✅ Database connection pool created successfully");
            
            // Test a simple query
            let result = sqlx::query("SELECT 1 as test_value")
                .fetch_one(&pool)
                .await;
                
            match result {
                Ok(row) => {
                    let value: i32 = row.get("test_value");
                    assert_eq!(value, 1);
                    println!("✅ Database query executed successfully: {}", value);
                }
                Err(e) => {
                    println!("❌ Database query failed: {:?}", e);
                    panic!("Database query test failed");
                }
            }
        }
        Err(e) => {
            println!("❌ Database connection failed: {:?}", e);
            println!("🔧 Make sure PostgreSQL is running and configured correctly");
            panic!("Database connection test failed");
        }
    }
}

#[tokio::test]
async fn test_02_database_health_check() {
    use mistersmith::persistence::{DbConfig, create_pool_with_retry};
    
    println!("🧪 Testing database health check...");
    
    let config = DbConfig {
        url: "postgresql://mistersmith_user:mistersmith_dev@localhost/mistersmith".to_string(),
        max_connections: 5,
        min_connections: 1,
        connect_timeout_secs: 5,
        idle_timeout_secs: 300,
        max_lifetime_secs: 900,
    };
    
    let pool = create_pool_with_retry(&config, 3, Duration::from_secs(1)).await
        .expect("Should create pool");
    
    // Test multiple concurrent connections
    let mut handles = vec![];
    
    for i in 0..3 {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let result = sqlx::query("SELECT $1 as connection_id")
                .bind(i)
                .fetch_one(&pool_clone)
                .await;
                
            match result {
                Ok(row) => {
                    let id: i32 = row.get("connection_id");
                    println!("✅ Connection {} successful", id);
                    id
                }
                Err(e) => {
                    println!("❌ Connection {} failed: {:?}", i, e);
                    -1
                }
            }
        });
        handles.push(handle);
    }
    
    // Wait for all connections to complete
    let mut successful = 0;
    for handle in handles {
        if let Ok(result) = handle.await {
            if result >= 0 {
                successful += 1;
            }
        }
    }
    
    assert_eq!(successful, 3, "All 3 concurrent connections should succeed");
    println!("✅ All {} concurrent connections successful", successful);
}

#[tokio::test]
async fn test_03_database_schema_ready() {
    use mistersmith::persistence::{DbConfig, create_pool_with_retry};
    
    println!("🧪 Testing database schema preparation...");
    
    let config = DbConfig {
        url: "postgresql://mistersmith_user:mistersmith_dev@localhost/mistersmith".to_string(),
        max_connections: 5,
        min_connections: 1,
        connect_timeout_secs: 5,
        idle_timeout_secs: 300,
        max_lifetime_secs: 900,
    };
    
    let pool = create_pool_with_retry(&config, 3, Duration::from_secs(1)).await
        .expect("Should create pool");
    
    // Check if we can create tables (testing basic permissions)
    let create_test_table = sqlx::query("
        CREATE TABLE IF NOT EXISTS test_table (
            id SERIAL PRIMARY KEY,
            name VARCHAR(100),
            created_at TIMESTAMP DEFAULT NOW()
        )
    ").execute(&pool).await;
    
    match create_test_table {
        Ok(_) => {
            println!("✅ Can create tables - permissions are correct");
            
            // Insert test data
            let insert_result = sqlx::query("
                INSERT INTO test_table (name) VALUES ($1) RETURNING id
            ")
            .bind("test_entry")
            .fetch_one(&pool)
            .await;
            
            match insert_result {
                Ok(row) => {
                    let id: i32 = row.get("id");
                    println!("✅ Can insert data - got ID: {}", id);
                    
                    // Clean up
                    sqlx::query("DROP TABLE test_table")
                        .execute(&pool)
                        .await
                        .expect("Should be able to drop test table");
                    println!("✅ Test table cleaned up");
                }
                Err(e) => {
                    println!("❌ Cannot insert data: {:?}", e);
                    panic!("Insert permission test failed");
                }
            }
        }
        Err(e) => {
            println!("❌ Cannot create tables: {:?}", e);
            panic!("Create table permission test failed");
        }
    }
}