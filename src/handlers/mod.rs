//! MCP Handler Module for Discovery Sharing
//! 
//! Implements the Model Context Protocol handlers for real-time discovery sharing

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

pub mod share_discovery;
pub mod subscribe_discoveries;
pub mod example_usage;
pub mod bridged_handlers;

/// MCP Parameters wrapper for typed handler inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameters<T> {
    pub params: T,
}

/// MCP Error type for handler errors
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
}

impl From<anyhow::Error> for McpError {
    fn from(err: anyhow::Error) -> Self {
        McpError::InternalError(err.to_string())
    }
}

impl From<serde_json::Error> for McpError {
    fn from(err: serde_json::Error) -> Self {
        McpError::InvalidParameters(err.to_string())
    }
}

/// MCP CallToolResult for handler responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<ContentItem>,
}

impl CallToolResult {
    pub fn text(content: String) -> Self {
        Self {
            content: vec![ContentItem::Text { text: content }],
        }
    }
    
    pub fn json(value: serde_json::Value) -> Self {
        Self {
            content: vec![ContentItem::Text { 
                text: serde_json::to_string_pretty(&value).unwrap_or_default() 
            }],
        }
    }
    
    pub fn with_metadata(content: String, metadata: HashMap<String, serde_json::Value>) -> Self {
        Self {
            content: vec![ContentItem::TextWithMetadata { 
                text: content,
                metadata,
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentItem {
    #[serde(rename = "text")]
    Text { text: String },
    
    #[serde(rename = "text_with_metadata")]
    TextWithMetadata { 
        text: String,
        metadata: HashMap<String, serde_json::Value>,
    },
}

/// Re-export DiscoveryStore and DiscoveryFilter from mcp_server
pub use crate::mcp_server::{DiscoveryStore, DiscoveryFilter};

// Global discovery store instance
lazy_static::lazy_static! {
    pub static ref DISCOVERY_STORE: DiscoveryStore = DiscoveryStore::new();
}

// Re-export bridged handlers for convenience
pub use bridged_handlers::*;