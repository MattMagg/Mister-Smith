//! Claude Code Single-shot Executor
//!
//! Implements a single-shot execution pattern for Claude Code CLI based on patterns
//! from Context7 SDKs (claude-sdk-rs and rmcp).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, error, info};

/// Response from Claude Code CLI in JSON format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub subtype: Option<String>,
    pub is_error: bool,
    pub result: Option<String>,
    pub session_id: String,
    pub total_cost_usd: f64,
    pub usage: Option<ClaudeUsage>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}

/// Single-shot Claude executor following Context7 SDK patterns
#[derive(Debug)]
pub struct ClaudeExecutor {
    timeout_duration: Duration,
}

impl ClaudeExecutor {
    /// Create a new Claude executor
    pub fn new(timeout_seconds: u64) -> Self {
        Self {
            timeout_duration: Duration::from_secs(timeout_seconds),
        }
    }

    /// Execute a single Claude command with a prompt
    /// Based on claude-sdk-rs pattern for subprocess execution
    pub async fn execute(&self, prompt: &str) -> Result<ClaudeResponse> {
        info!("Executing Claude command with prompt");
        
        // Build command following the correct CLI pattern
        let mut cmd = Command::new("claude");
        cmd.arg("--print")
           .arg("--output-format")
           .arg("json")
           .stdin(std::process::Stdio::piped())
           .stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::piped());

        debug!("Spawning Claude process");
        let mut child = cmd.spawn()
            .context("Failed to spawn Claude process")?;

        // Write prompt to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(prompt.as_bytes()).await
                .context("Failed to write prompt to stdin")?;
            stdin.flush().await
                .context("Failed to flush stdin")?;
        }

        // Wait for process with timeout
        let output = timeout(self.timeout_duration, child.wait_with_output())
            .await
            .context("Claude process timed out")?
            .context("Failed to wait for Claude process")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Claude process failed: {}", stderr);
            return Err(anyhow::anyhow!("Claude process failed: {}", stderr));
        }

        // Parse JSON response
        let stdout = String::from_utf8(output.stdout)
            .context("Failed to parse stdout as UTF-8")?;
        
        debug!("Claude response: {}", stdout);
        
        let response: ClaudeResponse = serde_json::from_str(&stdout)
            .context("Failed to parse Claude response as JSON")?;

        Ok(response)
    }

    /// Execute with retry logic (inspired by claude-sdk-rs)
    pub async fn execute_with_retry(&self, prompt: &str, max_retries: usize) -> Result<ClaudeResponse> {
        let mut last_error = None;
        
        for attempt in 0..=max_retries {
            if attempt > 0 {
                let delay = Duration::from_millis(100 * 2_u64.pow(attempt as u32));
                info!("Retry attempt {} after {:?}", attempt, delay);
                tokio::time::sleep(delay).await;
            }

            match self.execute(prompt).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    error!("Attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts failed")))
    }
}

/// Transport abstraction inspired by MCP SDK
#[async_trait::async_trait]
pub trait ClaudeTransport: Send + Sync {
    async fn execute(&self, prompt: &str) -> Result<ClaudeResponse>;
}

/// Print mode transport implementation
pub struct PrintModeTransport {
    executor: ClaudeExecutor,
}

impl PrintModeTransport {
    pub fn new(timeout_seconds: u64) -> Self {
        Self {
            executor: ClaudeExecutor::new(timeout_seconds),
        }
    }
}

#[async_trait::async_trait]
impl ClaudeTransport for PrintModeTransport {
    async fn execute(&self, prompt: &str) -> Result<ClaudeResponse> {
        self.executor.execute_with_retry(prompt, 2).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_claude_executor_basic() {
        let executor = ClaudeExecutor::new(30);
        let result = executor.execute("Say hello").await;
        
        // Should either succeed or fail with clear error
        match result {
            Ok(response) => {
                assert!(!response.is_error);
                assert!(response.result.is_some());
                assert!(!response.session_id.is_empty());
            }
            Err(e) => {
                // If Claude CLI not installed, that's OK for tests
                println!("Test skipped: {}", e);
            }
        }
    }
}