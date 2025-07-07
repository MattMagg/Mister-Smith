//! Claude CLI Command Builder
//!
//! Builds Claude CLI commands with appropriate arguments and configuration.

use std::path::PathBuf;
use anyhow::Result;
use thiserror::Error;

use crate::agent::AgentConfig;

/// Builder for Claude CLI commands
#[derive(Debug, Clone)]
pub struct ClaudeCommand {
    pub args: Vec<String>,
}

impl ClaudeCommand {
    /// Build a Claude CLI command from agent configuration
    pub fn build(config: &AgentConfig) -> Result<Self, CommandError> {
        let mut args = Vec::new();
        
        // Set model
        args.push("--model".to_string());
        args.push(config.model.clone());
        
        // Use print mode for subprocess integration (non-interactive)
        args.push("--print".to_string());
        
        // Set output format to JSON for parsing
        args.push("--output-format".to_string());
        args.push("json".to_string());
        
        // Set allowed tools if specified
        if let Some(allowed_tools) = &config.allowed_tools {
            if !allowed_tools.is_empty() {
                args.push("--allowedTools".to_string());
                args.push(allowed_tools.join(","));
            }
        }
        
        // Note: Claude Code doesn't support max_turns in --print mode
        // Note: Claude Code doesn't have a direct --timeout flag
        // Note: MCP is configured separately via config files
        
        Ok(ClaudeCommand { args })
    }
    
    /// Create a new command builder
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
        }
    }
    
    /// Add an argument to the command
    pub fn arg<S: Into<String>>(mut self, arg: S) -> Self {
        self.args.push(arg.into());
        self
    }
    
    /// Add multiple arguments to the command
    pub fn args<I>(mut self, args: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }
    
    /// Set the model
    pub fn model<S: Into<String>>(self, model: S) -> Self {
        self.arg("--model").arg(model)
    }
    
    /// Set output format
    pub fn output_format(self, format: OutputFormat) -> Self {
        self.arg("--output-format").arg(format.as_str())
    }
    
    /// Set max turns
    pub fn max_turns(self, turns: u32) -> Self {
        self.arg("--max-turns").arg(turns.to_string())
    }
    
    /// Set allowed tools
    pub fn allowed_tools(self, tools: &[String]) -> Self {
        if !tools.is_empty() {
            self.arg("--allowedTools").arg(tools.join(","))
        } else {
            self
        }
    }
    
    /// Enable MCP integration
    pub fn enable_mcp(self) -> Self {
        // MCP is configured via mcp-config in Claude Code
        self
    }
    
    /// Set MCP config path
    pub fn mcp_config<P: Into<PathBuf>>(self, path: P) -> Self {
        self.arg("--mcp-config").arg(path.into().to_string_lossy().to_string())
    }
    
    /// Enable print mode (non-interactive)
    pub fn print_mode(self) -> Self {
        self.arg("--print")
    }
    
    /// Set timeout (note: Claude Code doesn't have direct timeout support)
    pub fn timeout(self, _seconds: u64) -> Self {
        // Claude Code doesn't support timeout in print mode
        self
    }
    
    /// Validate the command arguments
    pub fn validate(&self) -> Result<(), CommandError> {
        if self.args.is_empty() {
            return Err(CommandError::EmptyCommand);
        }
        
        // Check for required model argument
        let has_model = self.args.windows(2).any(|window| {
            window[0] == "--model" && !window[1].is_empty()
        });
        
        if !has_model {
            return Err(CommandError::MissingRequiredArg("model".to_string()));
        }
        
        Ok(())
    }
}

impl Default for ClaudeCommand {
    fn default() -> Self {
        Self::new()
    }
}

/// Output format options for Claude CLI
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Text,
    Json,
    StreamJson,
}

impl OutputFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Text => "text",
            OutputFormat::Json => "json",
            OutputFormat::StreamJson => "stream-json",
        }
    }
}

/// Errors that can occur when building Claude CLI commands
#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Empty command")]
    EmptyCommand,
    
    #[error("Missing required argument: {0}")]
    MissingRequiredArg(String),
    
    #[error("Invalid argument value: {0}")]
    InvalidArgValue(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
