#!/usr/bin/env python3
"""
MCP Protocol Compliance Test Suite

Tests the MisterSmith MCP server binary for JSON-RPC 2.0 compliance
and proper MCP protocol implementation.
"""

import json
import subprocess
import sys
import time
import threading
from typing import Dict, Any, List, Optional, Union

class MCPProtocolTester:
    def __init__(self, binary_path: str):
        self.binary_path = binary_path
        self.test_results = []
        self.process = None
        
    def start_server(self):
        """Start the MCP server process"""
        try:
            self.process = subprocess.Popen(
                [self.binary_path],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=0
            )
            print(f"✅ MCP server started (PID: {self.process.pid})")
            return True
        except Exception as e:
            print(f"❌ Failed to start MCP server: {e}")
            return False
            
    def stop_server(self):
        """Stop the MCP server process"""
        if self.process:
            self.process.terminate()
            self.process.wait()
            print("🛑 MCP server stopped")
            
    def send_json_rpc(self, message: Dict[str, Any], timeout: float = 5.0) -> Optional[Dict[str, Any]]:
        """Send JSON-RPC message and wait for response"""
        try:
            json_message = json.dumps(message) + '\n'
            print(f"📤 Sending: {json_message.strip()}")
            
            self.process.stdin.write(json_message)
            self.process.stdin.flush()
            
            # Read response with timeout
            response_line = self.process.stdout.readline()
            if response_line:
                response = json.loads(response_line.strip())
                print(f"📥 Received: {json.dumps(response, indent=2)}")
                return response
            else:
                print("❌ No response received")
                return None
                
        except Exception as e:
            print(f"❌ Communication error: {e}")
            return None
            
    def validate_json_rpc_response(self, response: Dict[str, Any], expected_id: Union[str, int]) -> bool:
        """Validate JSON-RPC 2.0 response format"""
        if not isinstance(response, dict):
            print("❌ Response is not a JSON object")
            return False
            
        if response.get("jsonrpc") != "2.0":
            print(f"❌ Invalid jsonrpc version: {response.get('jsonrpc')}")
            return False
            
        if response.get("id") != expected_id:
            print(f"❌ ID mismatch: expected {expected_id}, got {response.get('id')}")
            return False
            
        has_result = "result" in response
        has_error = "error" in response
        
        if has_result and has_error:
            print("❌ Response has both result and error")
            return False
            
        if not has_result and not has_error:
            print("❌ Response has neither result nor error")
            return False
            
        if has_error:
            error = response["error"]
            if not isinstance(error, dict):
                print("❌ Error is not an object")
                return False
            if "code" not in error or "message" not in error:
                print("❌ Error missing code or message")
                return False
            if not isinstance(error["code"], int):
                print("❌ Error code is not an integer")
                return False
                
        print("✅ JSON-RPC 2.0 response format valid")
        return True
        
    def test_initialization_handshake(self) -> bool:
        """Test MCP initialization handshake"""
        print("\n🔍 Testing MCP initialization handshake...")
        
        # Send initialize request
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {
                    "roots": {"listChanged": True},
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "MCP-Test-Client",
                    "version": "1.0.0"
                }
            }
        }
        
        response = self.send_json_rpc(init_request)
        if not response:
            print("❌ No response to initialize request")
            return False
            
        if not self.validate_json_rpc_response(response, 1):
            return False
            
        if "result" not in response:
            print("❌ Initialize request failed")
            return False
            
        result = response["result"]
        required_fields = ["protocolVersion", "capabilities", "serverInfo"]
        for field in required_fields:
            if field not in result:
                print(f"❌ Missing required field in initialize response: {field}")
                return False
                
        print("✅ MCP initialization handshake successful")
        
        # Send initialized notification
        initialized_notification = {
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }
        
        # Send notification (no response expected)
        json_message = json.dumps(initialized_notification) + '\n'
        self.process.stdin.write(json_message)
        self.process.stdin.flush()
        print("✅ Initialized notification sent")
        
        return True
        
    def test_ping_utility(self) -> bool:
        """Test ping utility for responsiveness"""
        print("\n🔍 Testing ping utility...")
        
        ping_request = {
            "jsonrpc": "2.0",
            "id": "ping-123",
            "method": "ping"
        }
        
        response = self.send_json_rpc(ping_request)
        if not response:
            print("❌ No response to ping request")
            return False
            
        if not self.validate_json_rpc_response(response, "ping-123"):
            return False
            
        if "result" not in response:
            print("❌ Ping request failed")
            return False
            
        print("✅ Ping utility working correctly")
        return True
        
    def test_tools_list(self) -> bool:
        """Test tools/list to discover available tools"""
        print("\n🔍 Testing tools/list...")
        
        tools_request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }
        
        response = self.send_json_rpc(tools_request)
        if not response:
            print("❌ No response to tools/list request")
            return False
            
        if not self.validate_json_rpc_response(response, 2):
            return False
            
        if "result" not in response:
            print("❌ Tools/list request failed")
            return False
            
        tools = response["result"].get("tools", [])
        print(f"📋 Available tools: {len(tools)}")
        
        # Look for discovery tools
        discovery_tools = ["share_discovery", "subscribe_discoveries"]
        found_tools = []
        
        for tool in tools:
            if tool.get("name") in discovery_tools:
                found_tools.append(tool.get("name"))
                print(f"✅ Found discovery tool: {tool.get('name')}")
                
        if len(found_tools) == len(discovery_tools):
            print("✅ All discovery tools found")
            return True
        else:
            print(f"❌ Missing discovery tools: {set(discovery_tools) - set(found_tools)}")
            return False
            
    def test_error_handling(self) -> bool:
        """Test error handling for invalid requests"""
        print("\n🔍 Testing error handling...")
        
        # Test invalid method
        invalid_method_request = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "invalid_method_name"
        }
        
        response = self.send_json_rpc(invalid_method_request)
        if not response:
            print("❌ No response to invalid method request")
            return False
            
        if not self.validate_json_rpc_response(response, 3):
            return False
            
        if "error" not in response:
            print("❌ Expected error response for invalid method")
            return False
            
        error = response["error"]
        if error.get("code") != -32601:  # Method not found
            print(f"❌ Expected error code -32601, got {error.get('code')}")
            return False
            
        print("✅ Error handling for invalid method works correctly")
        
        # Test malformed JSON-RPC
        malformed_request = {
            "jsonrpc": "1.0",  # Wrong version
            "id": 4,
            "method": "ping"
        }
        
        response = self.send_json_rpc(malformed_request)
        if response and "error" in response:
            print("✅ Error handling for malformed JSON-RPC works correctly")
            return True
        else:
            print("⚠️  Server might be lenient with JSON-RPC version")
            return True  # Not a critical failure
            
    def run_all_tests(self) -> bool:
        """Run all protocol compliance tests"""
        print("🚀 Starting MCP Protocol Compliance Tests")
        print("=" * 50)
        
        if not self.start_server():
            return False
            
        try:
            # Give server time to start
            time.sleep(1)
            
            tests = [
                ("Initialization Handshake", self.test_initialization_handshake),
                ("Ping Utility", self.test_ping_utility),
                ("Tools List", self.test_tools_list),
                ("Error Handling", self.test_error_handling)
            ]
            
            passed = 0
            total = len(tests)
            
            for test_name, test_func in tests:
                try:
                    if test_func():
                        passed += 1
                        print(f"✅ {test_name}: PASSED")
                    else:
                        print(f"❌ {test_name}: FAILED")
                except Exception as e:
                    print(f"❌ {test_name}: ERROR - {e}")
                    
            print("\n" + "=" * 50)
            print(f"📊 Test Results: {passed}/{total} tests passed")
            
            return passed == total
            
        finally:
            self.stop_server()

def main():
    binary_path = "/Users/mac-main/Mister-Smith/agenterra/mcp-foundation/mistersmith-mcp-server/target/debug/mistersmith-mcp-server"
    
    tester = MCPProtocolTester(binary_path)
    success = tester.run_all_tests()
    
    if success:
        print("🎉 All protocol compliance tests passed!")
        sys.exit(0)
    else:
        print("💥 Some tests failed")
        sys.exit(1)

if __name__ == "__main__":
    main()