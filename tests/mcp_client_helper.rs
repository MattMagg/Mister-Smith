use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::timeout;

/// Helper for testing the actual MCP server binary
pub struct McpClientHelper {
    child: Child,
    stdin: Option<tokio::process::ChildStdin>,
    stdout_receiver: mpsc::Receiver<String>,
    request_id: u64,
}

impl McpClientHelper {
    /// Start the MCP server binary and establish STDIO communication
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let binary_path = "/Users/mac-main/Mister-Smith/agenterra/mcp-foundation/mistersmith-mcp-server/target/debug/mistersmith-mcp-server";
        
        let mut child = Command::new(binary_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        
        // Set up async reading from stdout
        let (tx, rx) = mpsc::channel(100);
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        
        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                if tx.send(line).await.is_err() {
                    break;
                }
            }
        });

        Ok(McpClientHelper {
            child,
            stdin: Some(stdin),
            stdout_receiver: rx,
            request_id: 1,
        })
    }

    /// Send a JSON-RPC request to the MCP server
    pub async fn send_request(&mut self, method: &str, params: Option<Value>) -> Result<Value, Box<dyn std::error::Error>> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params
        });

        let request_line = format!("{}\n", request);
        
        if let Some(stdin) = &mut self.stdin {
            stdin.write_all(request_line.as_bytes()).await?;
            stdin.flush().await?;
        }

        self.request_id += 1;

        // Wait for response
        let response = timeout(Duration::from_secs(10), self.stdout_receiver.recv())
            .await
            .map_err(|_| "Timeout waiting for response")?
            .ok_or("No response received")?;

        let response_json: Value = serde_json::from_str(&response)?;
        Ok(response_json)
    }

    /// Initialize the MCP server connection
    pub async fn initialize(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        let params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "notifications": {
                    "resources": true
                }
            },
            "clientInfo": {
                "name": "mistersmith-test-client",
                "version": "1.0.0"
            }
        });

        self.send_request("initialize", Some(params)).await
    }

    /// List all available tools
    pub async fn list_tools(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        self.send_request("tools/list", None).await
    }

    /// Call a specific tool
    pub async fn call_tool(&mut self, name: &str, arguments: Option<Value>) -> Result<Value, Box<dyn std::error::Error>> {
        let params = json!({
            "name": name,
            "arguments": arguments
        });

        self.send_request("tools/call", Some(params)).await
    }

    /// Share a discovery using the share_discovery tool
    pub async fn share_discovery(&mut self, discovery_type: &str, agent_id: &str, content: &str, metadata: Option<Value>) -> Result<Value, Box<dyn std::error::Error>> {
        let arguments = json!({
            "discovery_type": discovery_type,
            "agent_id": agent_id,
            "content": content,
            "metadata": metadata
        });

        self.call_tool("share_discovery", Some(arguments)).await
    }

    /// Subscribe to discoveries using the subscribe_discoveries tool
    pub async fn subscribe_discoveries(&mut self, agent_id: Option<&str>, discovery_types: Option<Vec<&str>>) -> Result<Value, Box<dyn std::error::Error>> {
        let mut arguments = json!({});
        
        if let Some(id) = agent_id {
            arguments["agent_id"] = json!(id);
        }
        
        if let Some(types) = discovery_types {
            arguments["discovery_types"] = json!(types);
        }

        self.call_tool("subscribe_discoveries", Some(arguments)).await
    }

    /// Wait for a notification from the server
    pub async fn wait_for_notification(&mut self, timeout_secs: u64) -> Result<Value, Box<dyn std::error::Error>> {
        let response = timeout(Duration::from_secs(timeout_secs), self.stdout_receiver.recv())
            .await
            .map_err(|_| "Timeout waiting for notification")?
            .ok_or("No notification received")?;

        let response_json: Value = serde_json::from_str(&response)?;
        Ok(response_json)
    }

    /// Get system health
    pub async fn get_system_health(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        self.call_tool("get_system_health", None).await
    }

    /// List all agents
    pub async fn list_agents(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        self.call_tool("list_agents", None).await
    }

    /// Get system metrics
    pub async fn get_system_metrics(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        self.call_tool("get_system_metrics", None).await
    }
}

impl Drop for McpClientHelper {
    fn drop(&mut self) {
        // Clean up the child process
        if let Err(e) = self.child.kill() {
            eprintln!("Failed to kill MCP server process: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_client_helper_basic() {
        let mut client = McpClientHelper::new().await.expect("Failed to start MCP client");
        
        // Test initialization
        let init_response = client.initialize().await.expect("Failed to initialize");
        assert!(init_response.get("result").is_some());
        
        // Test tools listing
        let tools_response = client.list_tools().await.expect("Failed to list tools");
        assert!(tools_response.get("result").is_some());
    }
}