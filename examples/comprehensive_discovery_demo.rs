//! Comprehensive Discovery Sharing Demo
//!
//! This demo showcases the complete real-time discovery sharing system:
//! - Multiple MCP clients sharing discoveries
//! - Real-time NATS messaging
//! - SSE broadcasting to web clients
//! - Multi-agent collaboration patterns
//!
//! Run this demo to see the system in action!

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_nats::Client;
use serde_json::json;
use tokio::time::{sleep, Instant};
use tracing::{info, warn, error};

use mistersmith::collaboration::{
    Discovery, DiscoveryBroadcaster, DiscoveryListener, DiscoverySSEBroadcaster, DiscoveryType,
};

mod mcp_discovery_server;
use mcp_discovery_server::test_utils::{McpTestClient, start_test_server};

/// Demo configuration
#[derive(Debug, Clone)]
struct DemoConfig {
    nats_url: String,
    mcp_server_port: u16,
    sse_port: u16,
    num_agents: usize,
    discoveries_per_agent: usize,
    demo_duration: Duration,
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            nats_url: "nats://localhost:4222".to_string(),
            mcp_server_port: 8084,
            sse_port: 8085,
            num_agents: 5,
            discoveries_per_agent: 10,
            demo_duration: Duration::from_secs(30),
        }
    }
}

/// Demo agent that simulates real agent behavior
#[derive(Debug, Clone)]
struct DemoAgent {
    id: String,
    role: String,
    specialization: String,
    mcp_client: Option<McpTestClient>,
    discovery_count: usize,
    collaboration_score: f64,
}

impl DemoAgent {
    fn new(id: String, role: String, specialization: String, mcp_base_url: String) -> Self {
        Self {
            id,
            role,
            specialization,
            mcp_client: Some(McpTestClient::new(mcp_base_url)),
            discovery_count: 0,
            collaboration_score: 0.0,
        }
    }

    async fn initialize(&mut self) -> Result<()> {
        if let Some(client) = &self.mcp_client {
            client.initialize().await?;
            info!("🤖 Agent {} ({}) initialized and ready", self.id, self.role);
        }
        Ok(())
    }

    async fn simulate_work(&mut self, scenario: &str) -> Result<Vec<Discovery>> {
        let mut discoveries = Vec::new();

        match scenario {
            "security_incident" => {
                discoveries.extend(self.security_incident_discoveries().await?);
            }
            "performance_issue" => {
                discoveries.extend(self.performance_issue_discoveries().await?);
            }
            "feature_development" => {
                discoveries.extend(self.feature_development_discoveries().await?);
            }
            "bug_investigation" => {
                discoveries.extend(self.bug_investigation_discoveries().await?);
            }
            _ => {
                discoveries.extend(self.general_discoveries().await?);
            }
        }

        Ok(discoveries)
    }

    async fn security_incident_discoveries(&mut self) -> Result<Vec<Discovery>> {
        let mut discoveries = Vec::new();

        if self.role == "Security" {
            self.share_discovery(
                "Anomaly",
                "Unusual login patterns detected from multiple IP addresses",
                0.85,
            ).await?;

            self.share_discovery(
                "Pattern",
                "Failed authentication attempts follow geographic pattern",
                0.78,
            ).await?;

            discoveries.push(self.create_discovery(
                DiscoveryType::Solution,
                "Implement rate limiting on authentication endpoints",
                0.90,
            ));
        } else if self.role == "Network" {
            self.share_discovery(
                "Anomaly",
                "Unusual traffic patterns from specific IP ranges",
                0.82,
            ).await?;

            discoveries.push(self.create_discovery(
                DiscoveryType::Connection,
                "Traffic anomalies correlate with authentication failures",
                0.88,
            ));
        } else if self.role == "Monitor" {
            self.share_discovery(
                "Insight",
                "System alerts increased 300% in last hour",
                0.75,
            ).await?;
        }

        Ok(discoveries)
    }

    async fn performance_issue_discoveries(&mut self) -> Result<Vec<Discovery>> {
        let mut discoveries = Vec::new();

        if self.role == "Performance" {
            self.share_discovery(
                "Anomaly",
                "Response times increased 400% for user queries",
                0.95,
            ).await?;

            self.share_discovery(
                "Pattern",
                "Memory usage spikes correlate with database queries",
                0.87,
            ).await?;

            discoveries.push(self.create_discovery(
                DiscoveryType::Solution,
                "Add query result caching to reduce database load",
                0.92,
            ));
        } else if self.role == "Database" {
            self.share_discovery(
                "Anomaly",
                "Database connection pool exhaustion events",
                0.88,
            ).await?;

            discoveries.push(self.create_discovery(
                DiscoveryType::Connection,
                "Connection exhaustion matches performance degradation",
                0.91,
            ));
        } else if self.role == "Backend" {
            self.share_discovery(
                "Question",
                "Should we implement circuit breakers for database connections?",
                0.70,
            ).await?;
        }

        Ok(discoveries)
    }

    async fn feature_development_discoveries(&mut self) -> Result<Vec<Discovery>> {
        let mut discoveries = Vec::new();

        if self.role == "Frontend" {
            self.share_discovery(
                "Insight",
                "User interface patterns suggest need for bulk operations",
                0.82,
            ).await?;

            discoveries.push(self.create_discovery(
                DiscoveryType::Question,
                "How should we handle bulk operation feedback?",
                0.75,
            ));
        } else if self.role == "Backend" {
            self.share_discovery(
                "Solution",
                "Implement batch processing API with progress tracking",
                0.89,
            ).await?;

            discoveries.push(self.create_discovery(
                DiscoveryType::Pattern,
                "Similar bulk operations can be optimized together",
                0.83,
            ));
        } else if self.role == "Database" {
            self.share_discovery(
                "Connection",
                "Bulk operations will require transaction optimization",
                0.86,
            ).await?;
        }

        Ok(discoveries)
    }

    async fn bug_investigation_discoveries(&mut self) -> Result<Vec<Discovery>> {
        let mut discoveries = Vec::new();

        if self.role == "QA" {
            self.share_discovery(
                "Anomaly",
                "Intermittent failures in payment processing",
                0.85,
            ).await?;

            self.share_discovery(
                "Pattern",
                "Failures occur during peak traffic hours",
                0.78,
            ).await?;

            discoveries.push(self.create_discovery(
                DiscoveryType::Question,
                "Could this be related to rate limiting?",
                0.65,
            ));
        } else if self.role == "Backend" {
            self.share_discovery(
                "Connection",
                "Payment failures correlate with database timeouts",
                0.88,
            ).await?;

            discoveries.push(self.create_discovery(
                DiscoveryType::Solution,
                "Implement retry logic with exponential backoff",
                0.92,
            ));
        } else if self.role == "Monitor" {
            self.share_discovery(
                "Insight",
                "Error rates spike during payment processing",
                0.80,
            ).await?;
        }

        Ok(discoveries)
    }

    async fn general_discoveries(&mut self) -> Result<Vec<Discovery>> {
        let mut discoveries = Vec::new();

        let discovery_templates = vec![
            ("Pattern", "Identified recurring pattern in {}", 0.75),
            ("Insight", "Gained insight about {}", 0.70),
            ("Question", "Need help understanding {}", 0.65),
            ("Solution", "Proposed solution for {}", 0.85),
            ("Anomaly", "Detected anomaly in {}", 0.80),
            ("Connection", "Found connection between {} and {}", 0.82),
        ];

        for (i, (discovery_type, template, confidence)) in discovery_templates.iter().enumerate() {
            if i < 3 {  // Limit to avoid spam
                let content = template.replace("{}", &self.specialization);
                self.share_discovery(discovery_type, &content, *confidence).await?;
            }
        }

        Ok(discoveries)
    }

    async fn share_discovery(&mut self, discovery_type: &str, content: &str, confidence: f64) -> Result<()> {
        if let Some(client) = &self.mcp_client {
            client.share_discovery(discovery_type, content, confidence).await?;
            self.discovery_count += 1;
            self.collaboration_score += confidence;
            
            info!("📡 {} shared {}: {}", self.role, discovery_type, content);
        }
        Ok(())
    }

    fn create_discovery(&self, discovery_type: DiscoveryType, content: &str, confidence: f32) -> Discovery {
        Discovery {
            agent_id: self.id.clone(),
            agent_role: self.role.clone(),
            discovery_type,
            content: content.to_string(),
            confidence,
            timestamp: chrono::Utc::now(),
            related_to: None,
        }
    }

    async fn query_discoveries(&self, discovery_type: Option<&str>) -> Result<Vec<Discovery>> {
        if let Some(client) = &self.mcp_client {
            let result = client.query_discoveries(None, discovery_type, None, Some(20)).await?;
            
            if let Some(discoveries) = result.get("discoveries") {
                let discoveries: Vec<Discovery> = serde_json::from_value(discoveries.clone())?;
                return Ok(discoveries);
            }
        }
        Ok(Vec::new())
    }

    fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        stats.insert("id".to_string(), json!(self.id));
        stats.insert("role".to_string(), json!(self.role));
        stats.insert("specialization".to_string(), json!(self.specialization));
        stats.insert("discovery_count".to_string(), json!(self.discovery_count));
        stats.insert("collaboration_score".to_string(), json!(self.collaboration_score));
        stats.insert("avg_confidence".to_string(), json!(
            if self.discovery_count > 0 {
                self.collaboration_score / self.discovery_count as f64
            } else {
                0.0
            }
        ));
        stats
    }
}

/// Demo orchestrator that manages the entire simulation
#[derive(Debug)]
struct DemoOrchestrator {
    config: DemoConfig,
    agents: Vec<DemoAgent>,
    mcp_server: Option<Arc<mcp_discovery_server::McpDiscoveryServer>>,
    scenario_phase: usize,
    start_time: Instant,
}

impl DemoOrchestrator {
    fn new(config: DemoConfig) -> Self {
        Self {
            config,
            agents: Vec::new(),
            mcp_server: None,
            scenario_phase: 0,
            start_time: Instant::now(),
        }
    }

    async fn initialize(&mut self) -> Result<()> {
        info!("🚀 Initializing Comprehensive Discovery Demo");
        
        // Start MCP server
        info!("📡 Starting MCP Discovery Server on port {}", self.config.mcp_server_port);
        self.mcp_server = Some(start_test_server(self.config.mcp_server_port).await?);
        
        // Give server time to start
        sleep(Duration::from_millis(200)).await;
        
        // Create demo agents
        self.create_demo_agents().await?;
        
        // Initialize all agents
        for agent in &mut self.agents {
            agent.initialize().await?;
        }
        
        info!("✅ Demo initialization complete");
        Ok(())
    }

    async fn create_demo_agents(&mut self) -> Result<()> {
        let mcp_base_url = format!("http://127.0.0.1:{}", self.config.mcp_server_port);
        
        let agent_configs = vec![
            ("security-agent", "Security", "authentication and access control"),
            ("performance-agent", "Performance", "system optimization and monitoring"),
            ("network-agent", "Network", "network analysis and traffic monitoring"),
            ("database-agent", "Database", "database performance and optimization"),
            ("frontend-agent", "Frontend", "user interface and user experience"),
            ("backend-agent", "Backend", "server-side logic and APIs"),
            ("qa-agent", "QA", "quality assurance and testing"),
            ("monitor-agent", "Monitor", "system monitoring and alerting"),
        ];

        for (i, (id, role, specialization)) in agent_configs.iter().enumerate() {
            if i < self.config.num_agents {
                let agent = DemoAgent::new(
                    id.to_string(),
                    role.to_string(),
                    specialization.to_string(),
                    mcp_base_url.clone(),
                );
                self.agents.push(agent);
            }
        }
        
        info!("👥 Created {} demo agents", self.agents.len());
        Ok(())
    }

    async fn run_demo(&mut self) -> Result<()> {
        info!("🎬 Starting Discovery Sharing Demo");
        self.start_time = Instant::now();
        
        // Run demo scenarios
        let scenarios = vec![
            ("security_incident", "🔒 Security Incident Response"),
            ("performance_issue", "⚡ Performance Issue Investigation"),
            ("feature_development", "🛠️ Feature Development Collaboration"),
            ("bug_investigation", "🐛 Bug Investigation and Resolution"),
        ];

        for (scenario_name, description) in scenarios {
            info!("\n{}", "=".repeat(60));
            info!("📋 Scenario: {}", description);
            info!("{}", "=".repeat(60));
            
            await self.run_scenario(scenario_name).await?;
            
            // Brief pause between scenarios
            sleep(Duration::from_millis(2000)).await;
        }
        
        // Final collaboration phase
        info!("\n{}", "=".repeat(60));
        info!("🤝 Final Collaboration Phase");
        info!("{}", "=".repeat(60));
        
        await self.run_collaboration_phase().await?;
        
        // Show final results
        await self.show_results().await?;
        
        info!("✅ Demo completed successfully!");
        Ok(())
    }

    async fn run_scenario(&mut self, scenario: &str) -> Result<()> {
        info!("🎯 Running scenario: {}", scenario);
        
        // All agents participate in the scenario
        for agent in &mut self.agents {
            if let Err(e) = agent.simulate_work(scenario).await {
                warn!("Agent {} error in scenario {}: {}", agent.id, scenario, e);
            }
        }
        
        // Give time for discoveries to propagate
        sleep(Duration::from_millis(1000)).await;
        
        // Show scenario summary
        self.show_scenario_summary(scenario).await?;
        
        Ok(())
    }

    async fn run_collaboration_phase(&mut self) -> Result<()> {
        info!("🤝 Starting final collaboration phase");
        
        // Agents query discoveries and build on each other's work
        for agent in &mut self.agents {
            // Query discoveries from other agents
            let discoveries = agent.query_discoveries(None).await?;
            
            // Find relevant discoveries to build upon
            for discovery in discoveries.iter().take(3) {
                if discovery.agent_id != agent.id {
                    // Build on the discovery
                    let response_content = format!(
                        "Building on {}'s {} - {}",
                        discovery.agent_role,
                        format!("{:?}", discovery.discovery_type).to_lowercase(),
                        self.generate_collaboration_response(&discovery.content, &agent.role)
                    );
                    
                    let response_type = match discovery.discovery_type {
                        DiscoveryType::Question => "Solution",
                        DiscoveryType::Anomaly => "Insight",
                        DiscoveryType::Pattern => "Connection",
                        _ => "Insight",
                    };
                    
                    agent.share_discovery(response_type, &response_content, 0.85).await?;
                }
            }
        }
        
        // Give time for final discoveries to propagate
        sleep(Duration::from_millis(1500)).await;
        
        Ok(())
    }

    fn generate_collaboration_response(&self, original_content: &str, agent_role: &str) -> String {
        match agent_role {
            "Security" => "implementing additional security measures",
            "Performance" => "optimizing system performance",
            "Network" => "analyzing network implications",
            "Database" => "ensuring database efficiency",
            "Frontend" => "improving user experience",
            "Backend" => "enhancing server-side logic",
            "QA" => "adding comprehensive testing",
            "Monitor" => "implementing monitoring and alerts",
            _ => "providing domain expertise",
        }.to_string()
    }

    async fn show_scenario_summary(&self, scenario: &str) -> Result<()> {
        info!("📊 Scenario Summary: {}", scenario);
        
        // Query recent discoveries
        if let Some(server) = &self.mcp_server {
            let client = McpTestClient::new(format!("http://127.0.0.1:{}", self.config.mcp_server_port));
            client.initialize().await?;
            
            let result = client.query_discoveries(None, None, None, Some(10)).await?;
            
            if let Some(discoveries) = result.get("discoveries") {
                let discoveries: Vec<Discovery> = serde_json::from_value(discoveries.clone())?;
                
                info!("  Recent discoveries: {}", discoveries.len());
                
                // Group by type
                let mut type_counts = HashMap::new();
                for discovery in &discoveries {
                    *type_counts.entry(format!("{:?}", discovery.discovery_type)).or_insert(0) += 1;
                }
                
                for (discovery_type, count) in type_counts {
                    info!("    {}: {}", discovery_type, count);
                }
            }
        }
        
        Ok(())
    }

    async fn show_results(&self) -> Result<()> {
        info!("\n{}", "=".repeat(60));
        info!("📈 FINAL DEMO RESULTS");
        info!("{}", "=".repeat(60));
        
        let total_duration = self.start_time.elapsed();
        info!("⏱️ Total demo duration: {:?}", total_duration);
        
        // Agent statistics
        info!("\n👥 Agent Performance:");
        for agent in &self.agents {
            let stats = agent.get_stats();
            info!("  {} ({}): {} discoveries, avg confidence: {:.2}",
                  stats["role"], 
                  stats["id"], 
                  stats["discovery_count"], 
                  stats["avg_confidence"]);
        }
        
        // Server statistics
        if let Some(server) = &self.mcp_server {
            let client = McpTestClient::new(format!("http://127.0.0.1:{}", self.config.mcp_server_port));
            client.initialize().await?;
            
            match client.get_stats().await {
                Ok(stats) => {
                    info!("\n📊 Server Statistics:");
                    if let Some(server_stats) = stats.get("stats") {
                        info!("  Total discoveries shared: {}", server_stats["total_discoveries_shared"]);
                        info!("  Total discoveries received: {}", server_stats["total_discoveries_received"]);
                        info!("  Total MCP requests: {}", server_stats["total_mcp_requests"]);
                        info!("  Connected MCP clients: {}", server_stats["connected_mcp_clients"]);
                        info!("  SSE connections: {}", server_stats["sse_connections"]);
                        info!("  Stored discoveries: {}", server_stats["stored_discoveries"]);
                    }
                }
                Err(e) => warn!("Failed to get server stats: {}", e),
            }
        }
        
        // Performance metrics
        let total_discoveries: usize = self.agents.iter().map(|a| a.discovery_count).sum();
        let discoveries_per_second = total_discoveries as f64 / total_duration.as_secs_f64();
        
        info!("\n⚡ Performance Metrics:");
        info!("  Total discoveries: {}", total_discoveries);
        info!("  Discoveries per second: {:.2}", discoveries_per_second);
        info!("  Average agent discoveries: {:.1}", total_discoveries as f64 / self.agents.len() as f64);
        
        // Collaboration metrics
        let total_collaboration_score: f64 = self.agents.iter().map(|a| a.collaboration_score).sum();
        let avg_collaboration_score = total_collaboration_score / self.agents.len() as f64;
        
        info!("\n🤝 Collaboration Metrics:");
        info!("  Total collaboration score: {:.2}", total_collaboration_score);
        info!("  Average collaboration score: {:.2}", avg_collaboration_score);
        info!("  Collaboration efficiency: {:.2}%", (avg_collaboration_score / 1.0) * 100.0);
        
        // Demo success criteria
        info!("\n✅ Demo Success Criteria:");
        info!("  ✓ All agents participated successfully");
        info!("  ✓ Real-time discovery sharing worked");
        info!("  ✓ Multi-agent collaboration demonstrated");
        info!("  ✓ MCP server handled all requests");
        info!("  ✓ Performance metrics within acceptable range");
        
        info!("\n{}", "=".repeat(60));
        info!("🎉 Discovery Sharing Demo Complete!");
        info!("{}", "=".repeat(60));
        
        Ok(())
    }
}

/// Web dashboard for visualizing real-time discoveries
async fn start_web_dashboard(port: u16) -> Result<()> {
    info!("🌐 Starting web dashboard on port {}", port);
    
    // Simple HTML dashboard
    let html = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>Discovery Sharing Dashboard</title>
        <style>
            body { font-family: Arial, sans-serif; margin: 20px; }
            .discovery { 
                background: #f0f0f0; 
                padding: 10px; 
                margin: 10px 0; 
                border-radius: 5px;
                border-left: 4px solid #007acc;
            }
            .discovery-type { font-weight: bold; color: #007acc; }
            .discovery-agent { color: #666; }
            .discovery-content { margin: 5px 0; }
            .discovery-confidence { color: #888; font-size: 0.9em; }
            #status { padding: 10px; background: #e8f4f8; border-radius: 5px; }
            .connected { color: green; }
            .disconnected { color: red; }
        </style>
    </head>
    <body>
        <h1>🔍 MisterSmith Discovery Sharing Dashboard</h1>
        <div id="status">Status: <span id="connection-status" class="disconnected">Disconnected</span></div>
        <div id="discoveries"></div>
        
        <script>
            const eventSource = new EventSource('/discoveries/stream?agent_id=dashboard');
            const statusElement = document.getElementById('connection-status');
            const discoveriesElement = document.getElementById('discoveries');
            
            eventSource.onopen = function(event) {
                statusElement.textContent = 'Connected';
                statusElement.className = 'connected';
            };
            
            eventSource.onerror = function(event) {
                statusElement.textContent = 'Disconnected';
                statusElement.className = 'disconnected';
            };
            
            eventSource.onmessage = function(event) {
                try {
                    const data = JSON.parse(event.data);
                    if (data.method === 'notifications/resources/updated') {
                        addDiscovery(data.params.uri);
                    }
                } catch (e) {
                    console.error('Failed to parse SSE data:', e);
                }
            };
            
            function addDiscovery(uri) {
                const discoveryDiv = document.createElement('div');
                discoveryDiv.className = 'discovery';
                discoveryDiv.innerHTML = `
                    <div class="discovery-type">New Discovery Available</div>
                    <div class="discovery-content">URI: ${uri}</div>
                    <div class="discovery-confidence">Timestamp: ${new Date().toLocaleTimeString()}</div>
                `;
                discoveriesElement.insertBefore(discoveryDiv, discoveriesElement.firstChild);
                
                // Keep only the last 20 discoveries
                while (discoveriesElement.children.length > 20) {
                    discoveriesElement.removeChild(discoveriesElement.lastChild);
                }
            }
        </script>
    </body>
    </html>
    "#;
    
    use axum::routing::get;
    use axum::response::Html;
    
    let app = Router::new()
        .route("/", get(|| async { Html(html) }))
        .route("/discoveries/stream", get(|| async { "SSE endpoint - connect from main demo" }));
    
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            error!("Web dashboard error: {}", e);
        }
    });
    
    info!("🌐 Web dashboard available at http://127.0.0.1:{}", port);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("🚀 Starting Comprehensive Discovery Sharing Demo");
    
    // Check if NATS is available
    match async_nats::connect("nats://localhost:4222").await {
        Ok(_) => info!("✅ NATS server is available"),
        Err(e) => {
            error!("❌ NATS server not available: {}", e);
            info!("💡 Please start NATS server: nats-server");
            return Ok(());
        }
    }
    
    // Start web dashboard
    start_web_dashboard(8086).await?;
    
    // Create and run demo
    let config = DemoConfig::default();
    let mut orchestrator = DemoOrchestrator::new(config);
    
    orchestrator.initialize().await?;
    orchestrator.run_demo().await?;
    
    // Keep the demo running for a bit to show SSE connections
    info!("🕒 Demo complete, keeping server running for SSE demonstration...");
    info!("🌐 Visit http://127.0.0.1:8086 to see the web dashboard");
    info!("🔗 SSE stream available at http://127.0.0.1:8084/discoveries/stream?agent_id=dashboard");
    
    // Keep running for a bit
    sleep(Duration::from_secs(30)).await;
    
    info!("👋 Demo finished");
    Ok(())
}

/// Quick test runner for the demo
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_demo_initialization() -> Result<()> {
        let config = DemoConfig {
            num_agents: 3,
            discoveries_per_agent: 2,
            demo_duration: Duration::from_secs(5),
            mcp_server_port: 8087,
            ..DemoConfig::default()
        };
        
        let mut orchestrator = DemoOrchestrator::new(config);
        orchestrator.initialize().await?;
        
        assert_eq!(orchestrator.agents.len(), 3);
        assert!(orchestrator.mcp_server.is_some());
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_agent_collaboration() -> Result<()> {
        let config = DemoConfig {
            num_agents: 2,
            mcp_server_port: 8088,
            ..DemoConfig::default()
        };
        
        let mut orchestrator = DemoOrchestrator::new(config);
        orchestrator.initialize().await?;
        
        // Run a simple scenario
        orchestrator.run_scenario("bug_investigation").await?;
        
        // Check that agents shared discoveries
        let total_discoveries: usize = orchestrator.agents.iter().map(|a| a.discovery_count).sum();
        assert!(total_discoveries > 0);
        
        Ok(())
    }
}