//! Interactive Claude Session - Foundation for Real-Time Collaboration
//! 
//! This is WHERE WE START: A single persistent Claude session that stays alive

use std::process::Stdio;
use tokio::process::{Command, Child, ChildStdin, ChildStdout, ChildStderr};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{RwLock, mpsc};
use std::sync::Arc;
use anyhow::{Result, Context};
use tracing::{info, debug, error};

/// A persistent Claude session that maintains conversation context
pub struct InteractiveClaudeSession {
    /// The long-lived Claude process
    process: Child,
    /// Write to Claude
    stdin: ChildStdin,
    /// Read from Claude
    stdout: BufReader<ChildStdout>,
    /// Error stream
    stderr: BufReader<ChildStderr>,
    /// Session state
    state: Arc<RwLock<SessionState>>,
    /// Message history for context
    history: Arc<RwLock<Vec<Message>>>,
}

#[derive(Debug, Clone)]
pub enum SessionState {
    Starting,
    Ready,
    Processing,
    Error(String),
    Terminated,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: std::time::Instant,
}

impl InteractiveClaudeSession {
    /// Start a persistent Claude session - THIS IS THE KEY DIFFERENCE!
    pub async fn start() -> Result<Self> {
        info!("Starting interactive Claude session (no --print flag!)");
        
        // Start Claude in INTERACTIVE mode - it stays alive!
        let mut cmd = Command::new("claude");
        // Interactive REPL mode with permission bypass
        cmd.arg("--dangerously-skip-permissions")
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let mut child = cmd.spawn()
            .context("Failed to spawn Claude process")?;
        
        // Take ownership of pipes
        let stdin = child.stdin.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdin"))?;
        let stdout = BufReader::new(
            child.stdout.take()
                .ok_or_else(|| anyhow::anyhow!("Failed to get stdout"))?
        );
        let stderr = BufReader::new(
            child.stderr.take()
                .ok_or_else(|| anyhow::anyhow!("Failed to get stderr"))?
        );
        
        let mut session = Self {
            process: child,
            stdin,
            stdout,
            stderr,
            state: Arc::new(RwLock::new(SessionState::Starting)),
            history: Arc::new(RwLock::new(Vec::new())),
        };
        
        // Wait for Claude to be ready
        session.wait_for_ready().await?;
        
        Ok(session)
    }
    
    /// Wait for Claude to show it's ready for input
    async fn wait_for_ready(&mut self) -> Result<()> {
        info!("Waiting for Claude to be ready...");
        
        // Read initial output until we see a prompt or indication it's ready
        // This will vary based on Claude's actual interactive mode behavior
        let mut line = String::new();
        
        // Set a timeout for startup
        let timeout = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            async {
                loop {
                    line.clear();
                    match self.stdout.read_line(&mut line).await {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            debug!("Claude output: {}", line.trim());
                            // Look for signs Claude is ready
                            // This depends on actual Claude CLI behavior
                            if line.contains(">") || line.contains("ready") {
                                break;
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
                Ok(())
            }
        ).await??;
        
        *self.state.write().await = SessionState::Ready;
        info!("Claude session ready!");
        Ok(())
    }
    
    /// Send a message and get response - SESSION STAYS ALIVE!
    pub async fn send_message(&mut self, content: &str) -> Result<String> {
        info!("Sending message to persistent Claude session");
        
        // Check state
        let state = self.state.read().await.clone();
        match state {
            SessionState::Ready => {},
            _ => return Err(anyhow::anyhow!("Session not ready: {:?}", state)),
        }
        
        // Update state
        *self.state.write().await = SessionState::Processing;
        
        // Record in history
        self.history.write().await.push(Message {
            role: "user".to_string(),
            content: content.to_string(),
            timestamp: std::time::Instant::now(),
        });
        
        // Send to Claude
        self.stdin.write_all(content.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        
        // Read response (this is the tricky part - need to know when response ends)
        let response = self.read_response().await?;
        
        // Record response
        self.history.write().await.push(Message {
            role: "assistant".to_string(),
            content: response.clone(),
            timestamp: std::time::Instant::now(),
        });
        
        *self.state.write().await = SessionState::Ready;
        Ok(response)
    }
    
    /// Read Claude's response - handling streaming output
    async fn read_response(&mut self) -> Result<String> {
        let mut response = String::new();
        let mut line = String::new();
        let mut blank_lines = 0;
        
        loop {
            line.clear();
            match self.stdout.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    debug!("Claude line: {}", line.trim());
                    
                    // Detect end of response
                    // This is heuristic - may need tuning based on Claude's behavior
                    if line.trim().is_empty() {
                        blank_lines += 1;
                        if blank_lines >= 2 {
                            break; // Assume response complete after 2 blank lines
                        }
                    } else {
                        blank_lines = 0;
                        response.push_str(&line);
                    }
                    
                    // Alternative: look for prompt character
                    if line.contains(">") && line.trim().len() < 5 {
                        break;
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        
        Ok(response.trim().to_string())
    }
    
    /// Get conversation history
    pub async fn get_history(&self) -> Vec<Message> {
        self.history.read().await.clone()
    }
    
    /// Gracefully terminate session
    pub async fn shutdown(mut self) -> Result<()> {
        info!("Shutting down Claude session");
        
        // Send exit command
        let _ = self.stdin.write_all(b"exit\n").await;
        let _ = self.stdin.flush().await;
        
        // Wait a bit for graceful shutdown
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        
        // Kill if still running
        let _ = self.process.kill().await;
        
        Ok(())
    }
}

/// Quick test to verify persistent sessions work
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_persistent_session() -> Result<()> {
        // Start session
        let mut session = InteractiveClaudeSession::start().await?;
        
        // Send first message
        let response1 = session.send_message("Hello! Please remember the number 42.").await?;
        println!("Response 1: {}", response1);
        
        // Send second message - Claude should remember!
        let response2 = session.send_message("What number did I ask you to remember?").await?;
        println!("Response 2: {}", response2);
        
        // Check if Claude remembered (should mention 42)
        assert!(response2.contains("42"), "Claude should remember the number!");
        
        // Shutdown
        session.shutdown().await?;
        
        Ok(())
    }
}

// THIS is the foundation - once we have persistent sessions, we can add:
// 1. NATS integration for real-time sharing
// 2. Multiple sessions collaborating
// 3. Orchestration layer
// 4. Shared context management