//! Context Sharing Protocol - How agents share working memory

use serde::{Serialize, Deserialize};
use async_nats::Client;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use futures::StreamExt;

/// Context that agents can share with each other
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedContext {
    /// Which agent is sharing
    pub from_agent: String,
    /// Target agent(s) - None means broadcast
    pub to_agent: Option<String>,
    /// Type of context being shared
    pub context_type: ContextType,
    /// The actual context
    pub content: ContextContent,
    /// How important this context is
    pub relevance: f32,
    /// Time-to-live in seconds
    pub ttl: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextType {
    /// Background information
    Background,
    /// Current working state
    WorkingMemory,
    /// Discovered patterns
    Patterns,
    /// Open questions
    Questions,
    /// Partial solutions
    PartialSolutions,
    /// Warnings or concerns
    Warnings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextContent {
    /// Simple text context
    Text(String),
    /// Key-value pairs
    Structured(std::collections::HashMap<String, String>),
    /// Conversation snippet
    Conversation {
        messages: Vec<(String, String)>, // (role, content)
    },
    /// Code or data
    Code {
        language: String,
        content: String,
    },
}

/// Manages context sharing between agents
pub struct ContextManager {
    agent_id: String,
    nats: Client,
    /// Local context cache
    context_cache: Arc<RwLock<Vec<SharedContext>>>,
}

impl ContextManager {
    pub fn new(agent_id: String, nats: Client) -> Self {
        Self {
            agent_id,
            nats,
            context_cache: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Share context with specific agent or broadcast
    pub async fn share_context(
        &self,
        to_agent: Option<String>,
        context_type: ContextType,
        content: ContextContent,
        relevance: f32,
    ) -> Result<()> {
        let context = SharedContext {
            from_agent: self.agent_id.clone(),
            to_agent: to_agent.clone(),
            context_type,
            content,
            relevance,
            ttl: 300, // 5 minutes default
        };
        
        // Cache locally
        self.context_cache.write().await.push(context.clone());
        
        // Determine subject
        let subject = match &to_agent {
            Some(agent) => format!("context.direct.{}", agent),
            None => "context.broadcast".to_string(),
        };
        
        self.nats.publish(
            subject,
            serde_json::to_vec(&context)?.into()
        ).await?;
        
        Ok(())
    }
    
    /// Share working memory with team
    pub async fn share_working_memory(&self, summary: &str) -> Result<()> {
        self.share_context(
            None,
            ContextType::WorkingMemory,
            ContextContent::Text(summary.to_string()),
            0.7,
        ).await
    }
    
    /// Ask a question with context
    pub async fn ask_with_context(
        &self,
        question: &str,
        background: &str,
    ) -> Result<()> {
        let mut context_map = std::collections::HashMap::new();
        context_map.insert("question".to_string(), question.to_string());
        context_map.insert("background".to_string(), background.to_string());
        
        self.share_context(
            None,
            ContextType::Questions,
            ContextContent::Structured(context_map),
            0.9,
        ).await
    }
    
    /// Listen for relevant context
    pub async fn listen_for_context<F>(&self, mut callback: F) -> Result<()>
    where
        F: FnMut(SharedContext) -> Result<()> + Send + 'static,
    {
        let agent_id = self.agent_id.clone();
        let mut sub = self.nats.subscribe("context.>").await?;
        
        while let Some(msg) = sub.next().await {
            if let Ok(context) = serde_json::from_slice::<SharedContext>(&msg.payload) {
                // Don't process our own context
                if context.from_agent != agent_id {
                    // Check if it's for us or broadcast
                    let is_for_us = match &context.to_agent {
                        Some(target) => target == &agent_id,
                        None => true, // Broadcast
                    };
                    
                    if is_for_us {
                        callback(context)?;
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// Example: How context sharing enables collaboration
pub mod examples {
    use super::*;
    
    /// Debugging scenario with shared context
    pub async fn collaborative_debugging() -> Result<()> {
        let nats = async_nats::connect("nats://localhost:4222").await?;
        
        // Frontend agent shares observation
        let frontend = ContextManager::new("frontend-agent".to_string(), nats.clone());
        
        frontend.share_context(
            None,
            ContextType::Patterns,
            ContextContent::Text(
                "UI freezes after exactly 50 API calls".to_string()
            ),
            0.9,
        ).await?;
        
        // Backend agent shares their context
        let backend = ContextManager::new("backend-agent".to_string(), nats.clone());
        
        let mut api_context = std::collections::HashMap::new();
        api_context.insert("rate_limit".to_string(), "50 per minute".to_string());
        api_context.insert("response".to_string(), "Should return 429".to_string());
        
        backend.share_context(
            None,
            ContextType::WorkingMemory,
            ContextContent::Structured(api_context),
            0.8,
        ).await?;
        
        // Network agent connects the dots with both contexts
        let network = ContextManager::new("network-agent".to_string(), nats.clone());
        
        network.share_context(
            None,
            ContextType::PartialSolutions,
            ContextContent::Text(
                "Proxy is swallowing 429 responses! Frontend never sees rate limit errors.".to_string()
            ),
            0.95,
        ).await?;
        
        // The shared context enabled finding the root cause!
        
        Ok(())
    }
}

// Context sharing turns isolated agents into a hive mind!