//! Direct PostgreSQL Connection Test
//! 
//! Tests database connectivity without using MisterSmith persistence module

use sqlx::{postgres::PgPoolOptions, Row};
use std::time::Duration;

#[tokio::test]
async fn test_direct_postgresql_connection() {
    println!("🧪 Testing direct PostgreSQL connection...");
    
    // Database URL for our setup
    let database_url = "postgresql://mistersmith_user:mistersmith_dev@localhost/mistersmith";
    
    // Create connection pool
    let pool_result = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(900))
        .connect(database_url)
        .await;
    
    match pool_result {
        Ok(pool) => {
            println!("✅ Connected to PostgreSQL successfully");
            
            // Test a simple query
            let result = sqlx::query("SELECT 'Hello MisterSmith!' as message, version() as pg_version")
                .fetch_one(&pool)
                .await;
                
            match result {
                Ok(row) => {
                    let message: String = row.get("message");
                    let version: String = row.get("pg_version");
                    println!("✅ Query successful: {}", message);
                    println!("📊 PostgreSQL version: {}", version);
                }
                Err(e) => {
                    println!("❌ Query failed: {:?}", e);
                    panic!("Database query test failed");
                }
            }
            
            // Test creating a simple table (for supervision data)
            let create_result = sqlx::query("
                CREATE TABLE IF NOT EXISTS supervision_test (
                    id SERIAL PRIMARY KEY,
                    node_id VARCHAR(255) NOT NULL,
                    status VARCHAR(50) NOT NULL,
                    created_at TIMESTAMP DEFAULT NOW()
                )
            ").execute(&pool).await;
            
            match create_result {
                Ok(_) => {
                    println!("✅ Can create supervision tables");
                    
                    // Insert test supervision data
                    let insert_result = sqlx::query("
                        INSERT INTO supervision_test (node_id, status) 
                        VALUES ($1, $2) 
                        RETURNING id
                    ")
                    .bind("test-supervisor")
                    .bind("active")
                    .fetch_one(&pool)
                    .await;
                    
                    match insert_result {
                        Ok(row) => {
                            let id: i32 = row.get("id");
                            println!("✅ Inserted supervision test data with ID: {}", id);
                            
                            // Clean up
                            sqlx::query("DROP TABLE supervision_test")
                                .execute(&pool)
                                .await
                                .expect("Should drop test table");
                            println!("✅ Test table cleaned up");
                        }
                        Err(e) => {
                            println!("❌ Failed to insert test data: {:?}", e);
                            panic!("Insert test failed");
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to create table: {:?}", e);
                    panic!("Create table test failed");
                }
            }
            
            // Close pool
            pool.close().await;
            println!("✅ Database connection closed cleanly");
        }
        Err(e) => {
            println!("❌ Failed to connect to PostgreSQL: {:?}", e);
            println!("🔧 Connection URL: {}", database_url);
            println!("🔧 Make sure PostgreSQL is running and user/database exist");
            panic!("Database connection test failed");
        }
    }
}

#[tokio::test]
async fn test_database_supervision_schema_ready() {
    println!("🧪 Testing readiness for supervision schema...");
    
    let database_url = "postgresql://mistersmith_user:mistersmith_dev@localhost/mistersmith";
    
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
        .expect("Should connect to database");
    
    // Test creating tables separately (PostgreSQL doesn't allow multiple commands in prepared statements)
    let nodes_table = sqlx::query("
        CREATE TABLE IF NOT EXISTS test_supervision_nodes (
            id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
            node_id TEXT NOT NULL UNIQUE,
            node_type TEXT NOT NULL,
            parent_node_id TEXT,
            status TEXT NOT NULL DEFAULT 'created',
            supervision_strategy TEXT NOT NULL DEFAULT 'one_for_one',
            restart_count INTEGER NOT NULL DEFAULT 0,
            failure_count INTEGER NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
    ").execute(&pool).await;
    
    let failures_table = sqlx::query("
        CREATE TABLE IF NOT EXISTS test_supervision_failures (
            id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
            node_id TEXT NOT NULL,
            failure_reason TEXT NOT NULL,
            failure_message TEXT NOT NULL,
            occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
    ").execute(&pool).await;
    
    let schema_test = nodes_table.and(failures_table);
    
    match schema_test {
        Ok(_) => {
            println!("✅ Supervision-like schema can be created");
            
            // Test inserting supervision-like data
            let node_id = "test-claude-supervisor";
            let insert_test = sqlx::query("
                INSERT INTO test_supervision_nodes (node_id, node_type, status) 
                VALUES ($1, $2, $3)
            ")
            .bind(node_id)
            .bind("root_supervisor")
            .bind("active")
            .execute(&pool)
            .await;
            
            match insert_test {
                Ok(_) => {
                    println!("✅ Can insert supervision node data");
                    
                    // Test failure record
                    let failure_test = sqlx::query("
                        INSERT INTO test_supervision_failures (node_id, failure_reason, failure_message)
                        VALUES ($1, $2, $3)
                    ")
                    .bind(node_id)
                    .bind("crash")
                    .bind("Test failure for PostgreSQL validation")
                    .execute(&pool)
                    .await;
                    
                    match failure_test {
                        Ok(_) => {
                            println!("✅ Can record supervision failures");
                            
                            // Query back the data
                            let query_result = sqlx::query("
                                SELECT n.node_id, n.status, COUNT(f.id) as failure_count
                                FROM test_supervision_nodes n
                                LEFT JOIN test_supervision_failures f ON n.node_id = f.node_id
                                WHERE n.node_id = $1
                                GROUP BY n.node_id, n.status
                            ")
                            .bind(node_id)
                            .fetch_one(&pool)
                            .await;
                            
                            match query_result {
                                Ok(row) => {
                                    let node: String = row.get("node_id");
                                    let status: String = row.get("status");
                                    let failures: i64 = row.get("failure_count");
                                    println!("✅ Query successful - Node: {}, Status: {}, Failures: {}", 
                                            node, status, failures);
                                }
                                Err(e) => {
                                    println!("❌ Query failed: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            println!("❌ Failed to insert failure record: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to insert node data: {:?}", e);
                }
            }
            
            // Clean up
            sqlx::query("DROP TABLE IF EXISTS test_supervision_failures, test_supervision_nodes")
                .execute(&pool)
                .await
                .expect("Should clean up test tables");
            println!("✅ Test schema cleaned up");
        }
        Err(e) => {
            println!("❌ Failed to create supervision schema: {:?}", e);
            panic!("Schema test failed");
        }
    }
    
    pool.close().await;
}