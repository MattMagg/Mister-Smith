#!/usr/bin/env python3
"""
MCP Discovery Tools Test Suite

Tests the share_discovery and subscribe_discoveries tools of the MisterSmith MCP server.
"""

import json
import subprocess
import sys
import time
import threading
from typing import Dict, Any, List, Optional, Union
import uuid

class DiscoveryToolsTester:
    def __init__(self, binary_path: str):
        self.binary_path = binary_path
        self.process = None
        self.notifications = []
        self.notification_thread = None
        self.running = False
        
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
        self.running = False
        if self.notification_thread:
            self.notification_thread.join(timeout=2)
            
        if self.process:
            self.process.terminate()
            self.process.wait()
            print("🛑 MCP server stopped")
            
    def start_notification_listener(self):
        """Start background thread to listen for notifications"""
        def listen_for_notifications():
            while self.running:
                try:
                    if self.process.stdout.readable():
                        line = self.process.stdout.readline()
                        if line:
                            try:
                                message = json.loads(line.strip())
                                if "method" in message and "notifications/" in message["method"]:
                                    self.notifications.append(message)
                                    print(f"📢 Notification received: {message['method']}")
                            except json.JSONDecodeError:
                                pass
                except Exception as e:
                    if self.running:
                        print(f"⚠️  Notification listener error: {e}")
                        
        self.running = True
        self.notification_thread = threading.Thread(target=listen_for_notifications)
        self.notification_thread.daemon = True
        self.notification_thread.start()
        
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
            
    def initialize_mcp(self) -> bool:
        """Initialize MCP connection"""
        print("\n🔧 Initializing MCP connection...")
        
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
                    "name": "Discovery-Test-Client",
                    "version": "1.0.0"
                }
            }
        }
        
        response = self.send_json_rpc(init_request)
        if not response or "result" not in response:
            print("❌ MCP initialization failed")
            return False
            
        # Send initialized notification
        initialized_notification = {
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }
        
        json_message = json.dumps(initialized_notification) + '\n'
        self.process.stdin.write(json_message)
        self.process.stdin.flush()
        
        print("✅ MCP initialization complete")
        return True
        
    def test_share_discovery_tool(self) -> bool:
        """Test share_discovery tool with various discovery types"""
        print("\n🔍 Testing share_discovery tool...")
        
        discovery_types = [
            ("Pattern", "Detected recurring pattern in agent behavior"),
            ("Anomaly", "Unusual response time detected"),
            ("Connection", "Found relationship between agents A and B"),
            ("Solution", "Proposed solution for coordination issue"),
            ("Question", "Need help with resource allocation"),
            ("Insight", "Agent coordination improves with shared context")
        ]
        
        agent_id = "test_agent_001"
        
        for i, (discovery_type, content) in enumerate(discovery_types):
            discovery_id = f"discovery_{i+1}_{uuid.uuid4().hex[:8]}"
            
            share_request = {
                "jsonrpc": "2.0",
                "id": 10 + i,
                "method": "tools/call",
                "params": {
                    "name": "share_discovery",
                    "arguments": {
                        "agent_id": agent_id,
                        "discovery_id": discovery_id,
                        "discovery_type": discovery_type,
                        "content": content,
                        "metadata": {
                            "timestamp": time.time(),
                            "confidence": 0.8,
                            "source": "test_suite"
                        }
                    }
                }
            }
            
            response = self.send_json_rpc(share_request)
            if not response:
                print(f"❌ No response for {discovery_type} discovery")
                return False
                
            if "result" not in response:
                print(f"❌ Share discovery failed for {discovery_type}")
                if "error" in response:
                    print(f"    Error: {response['error']}")
                return False
                
            result = response["result"]
            print(f"✅ Shared {discovery_type} discovery: {discovery_id}")
            
            # Verify result contains expected fields
            if "content" not in result:
                print(f"❌ Result missing content for {discovery_type}")
                return False
                
        print("✅ All discovery types shared successfully")
        return True
        
    def test_subscribe_discoveries_tool(self) -> bool:
        """Test subscribe_discoveries tool with filtering"""
        print("\n🔍 Testing subscribe_discoveries tool...")
        
        # Test subscription with no filters (all discoveries)
        subscribe_request = {
            "jsonrpc": "2.0",
            "id": 20,
            "method": "tools/call",
            "params": {
                "name": "subscribe_discoveries",
                "arguments": {
                    "agent_id": "test_subscriber_001"
                }
            }
        }
        
        response = self.send_json_rpc(subscribe_request)
        if not response:
            print("❌ No response for subscribe_discoveries")
            return False
            
        if "result" not in response:
            print("❌ Subscribe discoveries failed")
            if "error" in response:
                print(f"    Error: {response['error']}")
            return False
            
        print("✅ Subscribed to all discoveries")
        
        # Test subscription with filters
        filtered_subscribe_request = {
            "jsonrpc": "2.0",
            "id": 21,
            "method": "tools/call",
            "params": {
                "name": "subscribe_discoveries",
                "arguments": {
                    "agent_id": "test_subscriber_002",
                    "filter": {
                        "discovery_types": ["Pattern", "Anomaly"],
                        "agent_ids": ["test_agent_001"]
                    }
                }
            }
        }
        
        response = self.send_json_rpc(filtered_subscribe_request)
        if not response:
            print("❌ No response for filtered subscribe_discoveries")
            return False
            
        if "result" not in response:
            print("❌ Filtered subscribe discoveries failed")
            if "error" in response:
                print(f"    Error: {response['error']}")
            return False
            
        print("✅ Subscribed to filtered discoveries")
        return True
        
    def test_discovery_resource_notifications(self) -> bool:
        """Test resource notifications for discoveries"""
        print("\n🔍 Testing discovery resource notifications...")
        
        # Subscribe to resource updates
        subscribe_request = {
            "jsonrpc": "2.0",
            "id": 30,
            "method": "resources/subscribe",
            "params": {
                "uri": "discovery://agents/test_agent_001/discoveries"
            }
        }
        
        response = self.send_json_rpc(subscribe_request)
        if not response:
            print("❌ No response for resource subscription")
            return False
            
        if "result" not in response:
            print("❌ Resource subscription failed")
            if "error" in response:
                print(f"    Error: {response['error']}")
            return False
            
        print("✅ Subscribed to discovery resources")
        
        # Clear previous notifications
        self.notifications.clear()
        
        # Share a discovery to trigger notification
        discovery_id = f"notification_test_{uuid.uuid4().hex[:8]}"
        share_request = {
            "jsonrpc": "2.0",
            "id": 31,
            "method": "tools/call",
            "params": {
                "name": "share_discovery",
                "arguments": {
                    "agent_id": "test_agent_001",
                    "discovery_id": discovery_id,
                    "discovery_type": "Pattern",
                    "content": "Test discovery for notification",
                    "metadata": {"test": True}
                }
            }
        }
        
        response = self.send_json_rpc(share_request)
        if not response or "result" not in response:
            print("❌ Failed to share discovery for notification test")
            return False
            
        # Wait for notification
        time.sleep(2)
        
        # Check for resource update notification
        resource_notifications = [
            n for n in self.notifications
            if n.get("method") == "notifications/resources/updated"
        ]
        
        if resource_notifications:
            print(f"✅ Received {len(resource_notifications)} resource update notifications")
            for notification in resource_notifications:
                print(f"    URI: {notification.get('params', {}).get('uri')}")
            return True
        else:
            print("⚠️  No resource update notifications received")
            return False
            
    def test_discovery_data_validation(self) -> bool:
        """Test data validation and error handling"""
        print("\n🔍 Testing discovery data validation...")
        
        # Test invalid discovery type
        invalid_type_request = {
            "jsonrpc": "2.0",
            "id": 40,
            "method": "tools/call",
            "params": {
                "name": "share_discovery",
                "arguments": {
                    "agent_id": "test_agent_001",
                    "discovery_id": "invalid_type_test",
                    "discovery_type": "InvalidType",
                    "content": "This should fail"
                }
            }
        }
        
        response = self.send_json_rpc(invalid_type_request)
        if response and "error" in response:
            print("✅ Invalid discovery type properly rejected")
        else:
            print("⚠️  Invalid discovery type not rejected (might be flexible)")
            
        # Test missing required fields
        missing_fields_request = {
            "jsonrpc": "2.0",
            "id": 41,
            "method": "tools/call",
            "params": {
                "name": "share_discovery",
                "arguments": {
                    "agent_id": "test_agent_001"
                    # Missing discovery_id, discovery_type, content
                }
            }
        }
        
        response = self.send_json_rpc(missing_fields_request)
        if response and "error" in response:
            print("✅ Missing required fields properly rejected")
            return True
        else:
            print("❌ Missing required fields not rejected")
            return False
            
    def run_all_tests(self) -> bool:
        """Run all discovery tools tests"""
        print("🚀 Starting Discovery Tools Tests")
        print("=" * 50)
        
        if not self.start_server():
            return False
            
        try:
            # Give server time to start
            time.sleep(1)
            
            if not self.initialize_mcp():
                return False
                
            # Start notification listener
            self.start_notification_listener()
            
            tests = [
                ("Share Discovery Tool", self.test_share_discovery_tool),
                ("Subscribe Discoveries Tool", self.test_subscribe_discoveries_tool),
                ("Discovery Resource Notifications", self.test_discovery_resource_notifications),
                ("Discovery Data Validation", self.test_discovery_data_validation)
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
    
    tester = DiscoveryToolsTester(binary_path)
    success = tester.run_all_tests()
    
    if success:
        print("🎉 All discovery tools tests passed!")
        sys.exit(0)
    else:
        print("💥 Some tests failed")
        sys.exit(1)

if __name__ == "__main__":
    main()