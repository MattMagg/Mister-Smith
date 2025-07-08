//! Live Orchestrator - Coordinates real-time agent collaboration

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use async_nats::Client;
use anyhow::Result;
use tracing::{info, warn};
use serde::{Serialize, Deserialize};
use futures::StreamExt;

use crate::runtime::InteractiveClaudeSession;
use super::discovery_sharing::{Discovery, DiscoveryType};

/// Orchestrates multiple Claude sessions with real-time collaboration
pub struct LiveOrchestrator {
    /// Active Claude REPL sessions
    sessions: Arc<RwLock<HashMap<String, Arc<RwLock<InteractiveClaudeSession>>>>>,
    /// NATS client for coordination
    nats: Client,
    /// Orchestrator's own session for synthesis
    orchestrator_session: Arc<RwLock<InteractiveClaudeSession>>,
}

/// Commands the orchestrator can send to agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrchestratorCommand {
    /// Focus on a specific area
    Focus { area: String, priority: Priority },
    /// Collaborate with another agent
    Collaborate { with_agent: String, on_topic: String },
    /// Pause current work
    Pause { reason: String },
    /// Share your current findings
    ShareStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    High,
    Medium,
    Low,
}

impl LiveOrchestrator {
    /// Create a new orchestrator
    pub async fn new(nats: Client) -> Result<Self> {
        let orchestrator_session = InteractiveClaudeSession::start().await?;
        
        Ok(Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            nats,
            orchestrator_session: Arc::new(RwLock::new(orchestrator_session)),
        })
    }
    
    /// Spawn a specialized agent with a role
    pub async fn spawn_agent(&self, role: &str, initial_context: &str) -> Result<()> {
        info!("🚀 Spawning {} agent", role);
        
        // Start new Claude session
        let mut session = InteractiveClaudeSession::start().await?;
        
        // Give it initial context and role
        let setup = format!(
            "You are a {} specialist in a collaborative AI team. \
             Your role: {}. You'll receive real-time updates from other agents. \
             Always think about how your findings connect to others' work.",
            role, initial_context
        );
        
        session.send_message(&setup).await?;
        
        // Store session
        self.sessions.write().await.insert(
            role.to_string(),
            Arc::new(RwLock::new(session))
        );
        
        // Set up discovery listener for this agent
        self.setup_agent_listener(role).await?;
        
        Ok(())
    }
    
    /// Set up an agent to listen for discoveries
    async fn setup_agent_listener(&self, role: &str) -> Result<()> {
        let agent_role = role.to_string();
        let sessions = self.sessions.clone();
        let nats = self.nats.clone();
        
        tokio::spawn(async move {
            let mut sub = nats.subscribe("discoveries.>").await
                .expect("Failed to subscribe");
            
            while let Some(msg) = sub.next().await {
                if let Ok(discovery) = serde_json::from_slice::<Discovery>(&msg.payload) {
                    // Get the agent's session
                    if let Some(session) = sessions.read().await.get(&agent_role) {
                        let mut session = session.write().await;
                        
                        // Inform agent of the discovery
                        let prompt = format!(
                            "Another agent ({}) just discovered: '{}' \
                             (confidence: {}). How does this relate to your work?",
                            discovery.agent_role, discovery.content, discovery.confidence
                        );
                        
                        if let Ok(response) = session.send_message(&prompt).await {
                            // Agent might share their own discovery based on this
                            if response.contains("relate") || response.contains("connect") {
                                // Broadcast connection
                                let connection = Discovery {
                                    agent_id: format!("{}-agent", agent_role),
                                    agent_role: agent_role.clone(),
                                    discovery_type: DiscoveryType::Connection,
                                    content: response,
                                    confidence: 0.8,
                                    timestamp: chrono::Utc::now(),
                                    related_to: Some(discovery.content),
                                };
                                
                                let _ = nats.publish(
                                    "discoveries.connections",
                                    serde_json::to_vec(&connection).unwrap().into()
                                ).await;
                            }
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Orchestrate a complex task across multiple agents
    pub async fn orchestrate_task(&self, task: &str) -> Result<String> {
        info!("🎯 Orchestrating task: {}", task);
        
        // First, analyze the task
        let mut orchestrator = self.orchestrator_session.write().await;
        let analysis = orchestrator.send_message(&format!(
            "Analyze this task and determine what specialist agents we need: '{}'",
            task
        )).await?;
        
        // Spawn needed agents based on analysis
        if analysis.contains("security") {
            self.spawn_agent("Security", "Focus on vulnerabilities and threats").await?;
        }
        if analysis.contains("performance") {
            self.spawn_agent("Performance", "Analyze efficiency and bottlenecks").await?;
        }
        if analysis.contains("architecture") {
            self.spawn_agent("Architecture", "Review design and patterns").await?;
        }
        
        // Broadcast task to all agents
        self.broadcast_to_agents(task).await?;
        
        // Monitor discoveries for 30 seconds
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        
        // Synthesize findings
        let synthesis = orchestrator.send_message(
            "Based on all the discoveries shared by the team, \
             provide a comprehensive solution."
        ).await?;
        
        Ok(synthesis)
    }
    
    /// Broadcast a message to all agents
    async fn broadcast_to_agents(&self, message: &str) -> Result<()> {
        let sessions = self.sessions.read().await;
        
        for (role, session) in sessions.iter() {
            let mut session = session.write().await;
            session.send_message(message).await?;
            info!("📢 Sent to {}: {}", role, message);
        }
        
        Ok(())
    }
    
    /// Guide agents when they need help
    pub async fn provide_guidance(&self, agent_role: &str, question: &str) -> Result<String> {
        let mut orchestrator = self.orchestrator_session.write().await;
        
        let guidance = orchestrator.send_message(&format!(
            "The {} agent asks: '{}'. Provide brief guidance.",
            agent_role, question
        )).await?;
        
        // Send guidance to the specific agent
        if let Some(session) = self.sessions.read().await.get(agent_role) {
            let mut session = session.write().await;
            session.send_message(&format!("Orchestrator guidance: {}", guidance)).await?;
        }
        
        Ok(guidance)
    }
}

/// Example of orchestrated collaboration
pub async fn example_orchestrated_debugging() -> Result<()> {
    let nats = async_nats::connect("nats://localhost:4222").await?;
    let orchestrator = LiveOrchestrator::new(nats).await?;
    
    // Orchestrate a debugging session
    let result = orchestrator.orchestrate_task(
        "Debug why our API returns 502 errors intermittently during high load"
    ).await?;
    
    println!("🎯 Orchestrated solution: {}", result);
    
    // The magic: Multiple agents working together in real-time:
    // - Performance agent finds memory spikes
    // - Network agent discovers connection pool exhaustion  
    // - Architecture agent suggests caching layer
    // - All happening simultaneously with cross-pollination!
    
    Ok(())
}

use tracing::error;

// The orchestrator enables true collaborative intelligence!