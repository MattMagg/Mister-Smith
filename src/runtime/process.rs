//! Process Management for Claude CLI
//!
//! Manages individual Claude CLI processes with supervision and monitoring.

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock};
use anyhow::Result;
use thiserror::Error;
use tracing::{info, warn, error, debug};

use crate::agent::{AgentId, AgentConfig};
use crate::runtime::ClaudeCommand;

/// Process manager for a single Claude CLI instance
#[derive(Debug)]
pub struct ProcessManager {
    /// Agent ID this process manager belongs to
    agent_id: AgentId,
    /// Current Claude CLI process (if running)
    process: Arc<Mutex<Option<Child>>>,
    /// Process state
    state: Arc<RwLock<ProcessState>>,
    /// stdin handle for sending input
    stdin: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    /// stdout reader for receiving output
    stdout_reader: Arc<Mutex<Option<BufReader<tokio::process::ChildStdout>>>>,
    /// Paused state flag
    paused: Arc<RwLock<bool>>,
}

impl ProcessManager {
    /// Create a new process manager for the given agent
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            process: Arc::new(Mutex::new(None)),
            state: Arc::new(RwLock::new(ProcessState::NotStarted)),
            stdin: Arc::new(Mutex::new(None)),
            stdout_reader: Arc::new(Mutex::new(None)),
            paused: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Spawn a Claude CLI process with the given configuration
    pub async fn spawn(&self, config: &AgentConfig) -> Result<(), ProcessError> {
        info!(agent_id = %self.agent_id, "Spawning Claude CLI process");
        
        // Check if already running
        let current_state = *self.state.read().await;
        if !matches!(current_state, ProcessState::NotStarted | ProcessState::Terminated) {
            return Err(ProcessError::AlreadyRunning);
        }
        
        // Build Claude CLI command
        let claude_cmd = ClaudeCommand::build(config)
            .map_err(|e| ProcessError::SpawnFailed(format!("Failed to build command: {}", e)))?;
        
        // Set state to starting
        *self.state.write().await = ProcessState::Starting;
        
        // Spawn the process
        let mut cmd = Command::new("claude");
        cmd.args(&claude_cmd.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        
        debug!(agent_id = %self.agent_id, args = ?claude_cmd.args, "Spawning Claude CLI");
        
        let mut child = cmd.spawn().map_err(|e| {
            error!(agent_id = %self.agent_id, error = %e, "Failed to spawn Claude CLI");
            ProcessError::SpawnFailed(e.to_string())
        })?;
        
        // Extract handles
        let stdin = child.stdin.take().ok_or(ProcessError::NoStdin)?;
        let stdout = child.stdout.take().ok_or(ProcessError::NoStdout)?;
        
        // Store handles
        *self.process.lock().await = Some(child);
        *self.stdin.lock().await = Some(stdin);
        *self.stdout_reader.lock().await = Some(BufReader::new(stdout));
        
        // Set state to running
        *self.state.write().await = ProcessState::Running;
        
        info!(agent_id = %self.agent_id, "Claude CLI process spawned successfully");
        
        Ok(())
    }
    
    /// Stop the Claude CLI process
    pub async fn stop(&self) -> Result<(), ProcessError> {
        info!(agent_id = %self.agent_id, "Stopping Claude CLI process");
        
        let current_state = *self.state.read().await;
        if matches!(current_state, ProcessState::NotStarted | ProcessState::Terminated) {
            return Ok(()); // Already stopped
        }
        
        // Set state to stopping
        *self.state.write().await = ProcessState::Stopping;
        
        // Try graceful shutdown first
        if let Err(e) = self.send_input("exit").await {
            warn!(agent_id = %self.agent_id, error = %e, "Failed to send exit command");
        }
        
        // Wait for graceful shutdown with timeout
        let timeout = Duration::from_secs(10);
        let start = std::time::Instant::now();
        
        while start.elapsed() < timeout {
            if let Some(mut process) = self.process.lock().await.as_mut() {
                match process.try_wait() {
                    Ok(Some(_)) => {
                        // Process exited
                        *self.state.write().await = ProcessState::Terminated;
                        info!(agent_id = %self.agent_id, "Claude CLI process stopped gracefully");
                        return Ok(());
                    }
                    Ok(None) => {
                        // Still running
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                    Err(e) => {
                        error!(agent_id = %self.agent_id, error = %e, "Error checking process status");
                        break;
                    }
                }
            } else {
                // No process
                break;
            }
        }
        
        // Force kill if graceful shutdown failed
        warn!(agent_id = %self.agent_id, "Graceful shutdown timed out, force killing");
        self.kill().await
    }
    
    /// Force kill the Claude CLI process
    pub async fn kill(&self) -> Result<(), ProcessError> {
        warn!(agent_id = %self.agent_id, "Force killing Claude CLI process");
        
        if let Some(mut process) = self.process.lock().await.as_mut() {
            if let Err(e) = process.kill().await {
                error!(agent_id = %self.agent_id, error = %e, "Failed to kill process");
                return Err(ProcessError::KillFailed(e.to_string()));
            }
        }
        
        // Clean up handles
        *self.process.lock().await = None;
        *self.stdin.lock().await = None;
        *self.stdout_reader.lock().await = None;
        
        // Set state
        *self.state.write().await = ProcessState::Terminated;
        
        warn!(agent_id = %self.agent_id, "Claude CLI process killed");
        Ok(())
    }
    
    /// Pause the process (suspend input processing)
    pub async fn pause(&self) -> Result<(), ProcessError> {
        info!(agent_id = %self.agent_id, "Pausing process");
        
        let current_state = *self.state.read().await;
        if !matches!(current_state, ProcessState::Running) {
            return Err(ProcessError::InvalidState(current_state));
        }
        
        *self.paused.write().await = true;
        *self.state.write().await = ProcessState::Paused;
        
        Ok(())
    }
    
    /// Resume the process from paused state
    pub async fn resume(&self) -> Result<(), ProcessError> {
        info!(agent_id = %self.agent_id, "Resuming process");
        
        let current_state = *self.state.read().await;
        if !matches!(current_state, ProcessState::Paused) {
            return Err(ProcessError::InvalidState(current_state));
        }
        
        *self.paused.write().await = false;
        *self.state.write().await = ProcessState::Running;
        
        Ok(())
    }
    
    /// Send input to the Claude CLI process
    pub async fn send_input(&self, input: &str) -> Result<(), ProcessError> {
        // Check if paused
        if *self.paused.read().await {
            return Err(ProcessError::ProcessPaused);
        }
        
        let mut stdin_guard = self.stdin.lock().await;
        if let Some(stdin) = stdin_guard.as_mut() {
            stdin.write_all(input.as_bytes()).await
                .map_err(|e| ProcessError::InputFailed(e.to_string()))?;
            stdin.write_all(b"\n").await
                .map_err(|e| ProcessError::InputFailed(e.to_string()))?;
            stdin.flush().await
                .map_err(|e| ProcessError::InputFailed(e.to_string()))?;
            
            debug!(agent_id = %self.agent_id, input = input, "Sent input to Claude CLI");
            Ok(())
        } else {
            Err(ProcessError::NoStdin)
        }
    }
    
    /// Read a line of output from Claude CLI
    pub async fn read_output_line(&self) -> Result<Option<String>, ProcessError> {
        let mut reader_guard = self.stdout_reader.lock().await;
        if let Some(reader) = reader_guard.as_mut() {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => Ok(None), // EOF
                Ok(_) => {
                    // Remove trailing newline
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }
                    debug!(agent_id = %self.agent_id, output = %line, "Read output from Claude CLI");
                    Ok(Some(line))
                }
                Err(e) => Err(ProcessError::OutputFailed(e.to_string())),
            }
        } else {
            Err(ProcessError::NoStdout)
        }
    }
    
    /// Check if the process is healthy (running and responsive)
    pub async fn is_healthy(&self) -> Result<bool, ProcessError> {
        let current_state = *self.state.read().await;
        
        match current_state {
            ProcessState::Running | ProcessState::Paused => {
                // Check if process is still alive
                if let Some(process) = self.process.lock().await.as_mut() {
                    match process.try_wait() {
                        Ok(Some(_)) => {
                            // Process exited unexpectedly
                            *self.state.write().await = ProcessState::Terminated;
                            Ok(false)
                        }
                        Ok(None) => Ok(true), // Still running
                        Err(_) => Ok(false),   // Error checking status
                    }
                } else {
                    Ok(false) // No process
                }
            }
            _ => Ok(false), // Not in a healthy state
        }
    }
    
    /// Get current process state
    pub async fn get_state(&self) -> ProcessState {
        *self.state.read().await
    }
    
    /// Check if process is paused
    pub async fn is_paused(&self) -> bool {
        *self.paused.read().await
    }
}

/// State of a Claude CLI process
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// Process not started yet
    NotStarted,
    /// Process is starting
    Starting,
    /// Process is running normally
    Running,
    /// Process is paused (alive but not processing input)
    Paused,
    /// Process is stopping
    Stopping,
    /// Process has terminated
    Terminated,
    /// Process is in error state
    Error,
}

impl std::fmt::Display for ProcessState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessState::NotStarted => write!(f, "not_started"),
            ProcessState::Starting => write!(f, "starting"),
            ProcessState::Running => write!(f, "running"),
            ProcessState::Paused => write!(f, "paused"),
            ProcessState::Stopping => write!(f, "stopping"),
            ProcessState::Terminated => write!(f, "terminated"),
            ProcessState::Error => write!(f, "error"),
        }
    }
}

/// Errors that can occur during process management
#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("Process already running")]
    AlreadyRunning,
    
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(String),
    
    #[error("Process has no stdin")]
    NoStdin,
    
    #[error("Process has no stdout")]
    NoStdout,
    
    #[error("Failed to send input: {0}")]
    InputFailed(String),
    
    #[error("Failed to read output: {0}")]
    OutputFailed(String),
    
    #[error("Failed to kill process: {0}")]
    KillFailed(String),
    
    #[error("Invalid state for operation: {0}")]
    InvalidState(ProcessState),
    
    #[error("Process is paused")]
    ProcessPaused,
    
    #[error("Command build failed: {0}")]
    CommandBuildFailed(String),
}
