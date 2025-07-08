#!/usr/bin/env python3
"""
Direct MCP Server Test for MisterSmith Real-Time Discovery Flow
Tests the actual MCP server using JSON-RPC 2.0 calls
"""

import json
import subprocess
import sys
import time
import threading
from datetime import datetime

class MCPServerTester:
    def __init__(self):
        self.server_process = None
        self.test_results = []
        
    def start_mcp_server(self):
        """Start the MCP server process"""
        try:
            cmd = ["/Users/mac-main/Mister-Smith/agenterra/mcp-foundation/mistersmith-mcp-server/target/debug/mistersmith-mcp-server"]
            self.server_process = subprocess.Popen(
                cmd,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1
            )
            print("✅ MCP server started successfully")
            return True
        except Exception as e:
            print(f"❌ Failed to start MCP server: {e}")
            return False
    
    def send_json_rpc_request(self, method, params=None):
        """Send a JSON-RPC 2.0 request to the MCP server"""
        request = {
            "jsonrpc": "2.0",
            "id": int(time.time() * 1000),
            "method": method,
            "params": params or {}
        }
        
        try:
            request_json = json.dumps(request) + "\n"
            print(f"📤 Sending: {request_json.strip()}")
            
            self.server_process.stdin.write(request_json)
            self.server_process.stdin.flush()
            
            # Read response
            response_line = self.server_process.stdout.readline()
            if response_line:
                response = json.loads(response_line.strip())
                print(f"📥 Received: {json.dumps(response, indent=2)}")
                return response
            else:
                print("❌ No response received")
                return None
        except Exception as e:
            print(f"❌ Error sending request: {e}")
            return None
    
    def test_initialize(self):
        """Test MCP server initialization"""
        print("\n🔧 Test 1: MCP Server Initialization")
        response = self.send_json_rpc_request("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "roots": {
                    "listChanged": True
                },
                "sampling": {}
            },
            "clientInfo": {
                "name": "mistersmith-validation-test",
                "version": "1.0.0"
            }
        })
        
        if response and "result" in response:
            print("✅ MCP server initialized successfully")
            return True
        else:
            print("❌ MCP server initialization failed")
            return False
    
    def test_list_tools(self):
        """Test listing available tools"""
        print("\n🛠️  Test 2: List Available Tools")
        response = self.send_json_rpc_request("tools/list")
        
        if response and "result" in response:
            tools = response["result"]["tools"]
            print(f"✅ Found {len(tools)} tools:")
            for tool in tools:
                print(f"   • {tool['name']}: {tool.get('description', 'No description')}")
            
            # Check for our discovery tools
            tool_names = [tool['name'] for tool in tools]
            if "share_discovery" in tool_names and "subscribe_discoveries" in tool_names:
                print("✅ Discovery tools found")
                return True
            else:
                print("❌ Discovery tools not found")
                return False
        else:
            print("❌ Failed to list tools")
            return False
    
    def test_share_discovery(self):
        """Test sharing a discovery through MCP"""
        print("\n🔍 Test 3: Share Discovery")
        
        discovery_params = {
            "agent_id": "test-agent-1",
            "discovery_type": "pattern",
            "content": "End-to-end validation discovery from MCP test",
            "confidence": 0.95,
            "tags": ["validation", "test", "mcp"],
            "metadata": {
                "test_type": "end_to_end_validation",
                "timestamp": datetime.now().isoformat()
            }
        }
        
        response = self.send_json_rpc_request("tools/call", {
            "name": "share_discovery",
            "arguments": discovery_params
        })
        
        if response and "result" in response:
            print("✅ Discovery shared successfully")
            return True
        else:
            print("❌ Discovery sharing failed")
            return False
    
    def test_subscribe_discoveries(self):
        """Test subscribing to discoveries"""
        print("\n📡 Test 4: Subscribe to Discoveries")
        
        subscribe_params = {
            "agent_id": "test-subscriber",
            "tags": ["validation", "test"],
            "discovery_types": ["pattern", "insight"]
        }
        
        response = self.send_json_rpc_request("tools/call", {
            "name": "subscribe_discoveries",
            "arguments": subscribe_params
        })
        
        if response and "result" in response:
            print("✅ Discovery subscription successful")
            return True
        else:
            print("❌ Discovery subscription failed")
            return False
    
    def test_multi_discovery_flow(self):
        """Test multiple discoveries flowing through the system"""
        print("\n🔄 Test 5: Multi-Discovery Flow")
        
        discoveries = [
            {
                "agent_id": "security-agent",
                "discovery_type": "anomaly",
                "content": "Security anomaly detected in authentication patterns",
                "confidence": 0.88,
                "tags": ["security", "authentication"],
                "metadata": {"severity": "high"}
            },
            {
                "agent_id": "performance-agent",
                "discovery_type": "connection",
                "content": "Performance degradation correlates with security events",
                "confidence": 0.82,
                "tags": ["performance", "correlation"],
                "metadata": {"correlation_strength": 0.82}
            },
            {
                "agent_id": "data-agent",
                "discovery_type": "solution",
                "content": "Implement adaptive rate limiting based on user behavior",
                "confidence": 0.91,
                "tags": ["solution", "rate_limiting"],
                "metadata": {"priority": "high"}
            }
        ]
        
        success_count = 0
        for i, discovery in enumerate(discoveries):
            print(f"📤 Sharing discovery {i+1}/3...")
            response = self.send_json_rpc_request("tools/call", {
                "name": "share_discovery",
                "arguments": discovery
            })
            
            if response and "result" in response:
                success_count += 1
                print(f"✅ Discovery {i+1} shared successfully")
            else:
                print(f"❌ Discovery {i+1} sharing failed")
            
            time.sleep(0.1)  # Small delay between discoveries
        
        print(f"📊 Multi-discovery flow result: {success_count}/{len(discoveries)} successful")
        return success_count == len(discoveries)
    
    def cleanup(self):
        """Clean up the MCP server process"""
        if self.server_process:
            try:
                self.server_process.terminate()
                self.server_process.wait(timeout=5)
                print("✅ MCP server terminated cleanly")
            except:
                self.server_process.kill()
                print("⚠️  MCP server force killed")
    
    def run_all_tests(self):
        """Run all MCP server tests"""
        print("🧪 DIRECT MCP SERVER VALIDATION TEST")
        print("====================================")
        
        if not self.start_mcp_server():
            return False
        
        # Give server time to start
        time.sleep(2)
        
        try:
            test_results = []
            
            # Run all tests
            test_results.append(self.test_initialize())
            test_results.append(self.test_list_tools())
            test_results.append(self.test_share_discovery())
            test_results.append(self.test_subscribe_discoveries())
            test_results.append(self.test_multi_discovery_flow())
            
            # Summary
            passed = sum(test_results)
            total = len(test_results)
            
            print(f"\n🎯 TEST SUMMARY")
            print(f"===============")
            print(f"Tests passed: {passed}/{total}")
            print(f"Success rate: {passed/total*100:.1f}%")
            
            if passed == total:
                print("🎉 ALL MCP SERVER TESTS PASSED!")
                return True
            else:
                print("❌ Some MCP server tests failed")
                return False
                
        finally:
            self.cleanup()

def main():
    tester = MCPServerTester()
    success = tester.run_all_tests()
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()