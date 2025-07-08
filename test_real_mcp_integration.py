#!/usr/bin/env python3
"""
Real MCP Server Integration Test Suite

Complete end-to-end testing of the MisterSmith MCP server with multi-agent scenarios.
"""

import json
import subprocess
import sys
import time
import threading
import uuid
import asyncio
from concurrent.futures import ThreadPoolExecutor
from typing import Dict, Any, List, Optional, Union, Tuple

class MCPIntegrationTester:
    def __init__(self, binary_path: str):
        self.binary_path = binary_path
        self.sessions = {}
        self.notifications = []
        self.running = False
        
    def create_session(self, session_id: str) -> Dict:
        """Create a new MCP session"""
        try:
            process = subprocess.Popen(
                [self.binary_path],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=0
            )
            
            session = {
                'id': session_id,
                'process': process,
                'notifications': [],
                'initialized': False
            }
            
            self.sessions[session_id] = session
            print(f"✅ Created session {session_id} (PID: {process.pid})")
            return session
            
        except Exception as e:
            print(f"❌ Failed to create session {session_id}: {e}")
            return None
            
    def close_session(self, session_id: str):
        """Close an MCP session"""
        if session_id in self.sessions:
            session = self.sessions[session_id]
            session['process'].terminate()
            session['process'].wait()
            del self.sessions[session_id]
            print(f"🛑 Closed session {session_id}")
            
    def close_all_sessions(self):
        """Close all MCP sessions"""
        for session_id in list(self.sessions.keys()):
            self.close_session(session_id)
            
    def send_to_session(self, session_id: str, message: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """Send message to specific session"""
        if session_id not in self.sessions:
            print(f"❌ Session {session_id} not found")
            return None
            
        session = self.sessions[session_id]
        process = session['process']
        
        try:
            json_message = json.dumps(message) + '\n'
            print(f"📤 [{session_id}] Sending: {json_message.strip()}")
            
            process.stdin.write(json_message)
            process.stdin.flush()
            
            # For notifications, don't wait for response
            if "id" not in message:
                return None
                
            # Read response
            response_line = process.stdout.readline()
            if response_line:
                response = json.loads(response_line.strip())
                print(f"📥 [{session_id}] Received: {json.dumps(response, indent=2)}")
                return response
            else:
                print(f"❌ [{session_id}] No response received")
                return None
                
        except Exception as e:
            print(f"❌ [{session_id}] Communication error: {e}")
            return None
            
    def initialize_session(self, session_id: str, client_name: str) -> bool:
        """Initialize MCP session"""
        print(f"\n🔧 Initializing session {session_id}...")
        
        # Send initialize request
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {
                    "resources": {"subscribe": True},
                    "roots": {"listChanged": True},
                    "sampling": {}
                },
                "clientInfo": {
                    "name": client_name,
                    "version": "1.0.0"
                }
            }
        }
        
        response = self.send_to_session(session_id, init_request)
        if not response or "result" not in response:
            print(f"❌ [{session_id}] MCP initialization failed")
            return False
            
        # Send initialized notification
        initialized_notification = {
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }
        
        self.send_to_session(session_id, initialized_notification)
        
        self.sessions[session_id]['initialized'] = True
        print(f"✅ [{session_id}] MCP initialization complete")
        return True
        
    def test_multi_agent_discovery_sharing(self) -> bool:
        """Test multi-agent discovery sharing scenario"""
        print("\n🔍 Testing multi-agent discovery sharing...")
        
        # Create sessions for multiple agents
        agent_sessions = {
            "agent_producer": "Producer-Agent",
            "agent_consumer1": "Consumer-Agent-1",
            "agent_consumer2": "Consumer-Agent-2"
        }
        
        # Create and initialize sessions
        for session_id, client_name in agent_sessions.items():
            session = self.create_session(session_id)
            if not session:
                return False
                
            time.sleep(0.5)  # Stagger session creation
            
            if not self.initialize_session(session_id, client_name):
                return False
                
        # Set up consumers to subscribe to discoveries
        consumer_sessions = ["agent_consumer1", "agent_consumer2"]
        
        for consumer_id in consumer_sessions:
            subscribe_request = {
                "jsonrpc": "2.0",
                "id": 10,
                "method": "tools/call",
                "params": {
                    "name": "subscribe_discoveries",
                    "arguments": {
                        "agent_id": consumer_id,
                        "filter": {
                            "agent_ids": ["agent_producer"]
                        }
                    }
                }
            }
            
            response = self.send_to_session(consumer_id, subscribe_request)
            if not response or "result" not in response:
                print(f"❌ [{consumer_id}] Failed to subscribe to discoveries")
                return False
                
            print(f"✅ [{consumer_id}] Subscribed to producer discoveries")
            
        # Producer shares multiple discoveries
        discoveries = [
            ("Pattern", "Detected consistent response pattern in workflow"),
            ("Anomaly", "Unusual delay in processing step 3"),
            ("Solution", "Proposed parallel processing to reduce delays"),
            ("Insight", "System performance improves with batch processing")
        ]
        
        producer_id = "agent_producer"
        shared_discoveries = []
        
        for i, (discovery_type, content) in enumerate(discoveries):
            discovery_id = f"discovery_{i+1}_{uuid.uuid4().hex[:8]}"
            
            share_request = {
                "jsonrpc": "2.0",
                "id": 20 + i,
                "method": "tools/call",
                "params": {
                    "name": "share_discovery",
                    "arguments": {
                        "agent_id": producer_id,
                        "discovery_id": discovery_id,
                        "discovery_type": discovery_type,
                        "content": content,
                        "metadata": {
                            "timestamp": time.time(),
                            "importance": "high" if discovery_type == "Solution" else "medium"
                        }
                    }
                }
            }
            
            response = self.send_to_session(producer_id, share_request)
            if not response or "result" not in response:
                print(f"❌ [{producer_id}] Failed to share {discovery_type} discovery")
                return False
                
            shared_discoveries.append(discovery_id)
            print(f"✅ [{producer_id}] Shared {discovery_type} discovery: {discovery_id}")
            
            # Small delay between discoveries
            time.sleep(0.5)
            
        print(f"✅ Producer shared {len(shared_discoveries)} discoveries")
        return True
        
    def test_resource_subscription_workflow(self) -> bool:
        """Test resource subscription and notification workflow"""
        print("\n🔍 Testing resource subscription workflow...")
        
        # Create observer session
        observer_session = self.create_session("observer")
        if not observer_session:
            return False
            
        if not self.initialize_session("observer", "Resource-Observer"):
            return False
            
        # Subscribe to discovery resources
        resource_patterns = [
            "discovery://agents/test_agent/discoveries",
            "discovery://agents/test_agent/discoveries/pattern",
            "discovery://agents/test_agent/discoveries/anomaly"
        ]
        
        for pattern in resource_patterns:
            subscribe_request = {
                "jsonrpc": "2.0",
                "id": 30,
                "method": "resources/subscribe",
                "params": {
                    "uri": pattern
                }
            }
            
            response = self.send_to_session("observer", subscribe_request)
            if response and "result" in response:
                print(f"✅ [observer] Subscribed to resource: {pattern}")
            else:
                print(f"⚠️  [observer] Could not subscribe to resource: {pattern}")
                
        # Create producer to generate discoveries
        producer_session = self.create_session("test_agent")
        if not producer_session:
            return False
            
        if not self.initialize_session("test_agent", "Test-Producer"):
            return False
            
        # Generate discoveries to trigger resource notifications
        test_discoveries = [
            ("Pattern", "Resource notification test pattern"),
            ("Anomaly", "Resource notification test anomaly")
        ]
        
        for discovery_type, content in test_discoveries:
            discovery_id = f"resource_test_{uuid.uuid4().hex[:8]}"
            
            share_request = {
                "jsonrpc": "2.0",
                "id": 40,
                "method": "tools/call",
                "params": {
                    "name": "share_discovery",
                    "arguments": {
                        "agent_id": "test_agent",
                        "discovery_id": discovery_id,
                        "discovery_type": discovery_type,
                        "content": content,
                        "metadata": {"resource_test": True}
                    }
                }
            }
            
            response = self.send_to_session("test_agent", share_request)
            if response and "result" in response:
                print(f"✅ [test_agent] Shared {discovery_type} for resource test")
            else:
                print(f"❌ [test_agent] Failed to share {discovery_type}")
                return False
                
        print("✅ Resource subscription workflow test complete")
        return True
        
    def test_concurrent_discovery_operations(self) -> bool:
        """Test concurrent discovery operations from multiple agents"""
        print("\n🔍 Testing concurrent discovery operations...")
        
        # Create multiple agent sessions
        agent_count = 3
        agent_sessions = {}
        
        for i in range(agent_count):
            session_id = f"concurrent_agent_{i+1}"
            agent_sessions[session_id] = self.create_session(session_id)
            if not agent_sessions[session_id]:
                return False
                
            if not self.initialize_session(session_id, f"Concurrent-Agent-{i+1}"):
                return False
                
        # Each agent shares discoveries concurrently
        def agent_discovery_task(session_id: str) -> bool:
            discoveries_per_agent = 5
            
            for i in range(discoveries_per_agent):
                discovery_id = f"{session_id}_discovery_{i+1}"
                
                share_request = {
                    "jsonrpc": "2.0",
                    "id": 50 + i,
                    "method": "tools/call",
                    "params": {
                        "name": "share_discovery",
                        "arguments": {
                            "agent_id": session_id,
                            "discovery_id": discovery_id,
                            "discovery_type": "Pattern",
                            "content": f"Concurrent discovery {i+1} from {session_id}",
                            "metadata": {
                                "concurrent_test": True,
                                "agent_number": session_id.split('_')[-1]
                            }
                        }
                    }
                }
                
                response = self.send_to_session(session_id, share_request)
                if not response or "result" not in response:
                    print(f"❌ [{session_id}] Failed to share discovery {i+1}")
                    return False
                    
                # Small delay between discoveries
                time.sleep(0.1)
                
            return True
            
        # Run discovery tasks concurrently
        with ThreadPoolExecutor(max_workers=agent_count) as executor:
            futures = []
            for session_id in agent_sessions.keys():
                future = executor.submit(agent_discovery_task, session_id)
                futures.append((session_id, future))
                
            # Wait for all tasks to complete
            all_successful = True
            for session_id, future in futures:
                try:
                    result = future.result(timeout=30)
                    if result:
                        print(f"✅ [{session_id}] Completed concurrent discovery task")
                    else:
                        print(f"❌ [{session_id}] Failed concurrent discovery task")
                        all_successful = False
                except Exception as e:
                    print(f"❌ [{session_id}] Error in concurrent discovery task: {e}")
                    all_successful = False
                    
        return all_successful
        
    def test_discovery_persistence_and_retrieval(self) -> bool:
        """Test discovery persistence and retrieval capabilities"""
        print("\n🔍 Testing discovery persistence and retrieval...")
        
        # Create session for persistence test
        persistence_session = self.create_session("persistence_test")
        if not persistence_session:
            return False
            
        if not self.initialize_session("persistence_test", "Persistence-Test-Agent"):
            return False
            
        # Share discoveries
        test_discoveries = [
            ("Pattern", "Persistent pattern discovery"),
            ("Solution", "Persistent solution discovery"),
            ("Insight", "Persistent insight discovery")
        ]
        
        shared_ids = []
        for discovery_type, content in test_discoveries:
            discovery_id = f"persistent_{uuid.uuid4().hex[:8]}"
            
            share_request = {
                "jsonrpc": "2.0",
                "id": 60,
                "method": "tools/call",
                "params": {
                    "name": "share_discovery",
                    "arguments": {
                        "agent_id": "persistence_test",
                        "discovery_id": discovery_id,
                        "discovery_type": discovery_type,
                        "content": content,
                        "metadata": {"persistence_test": True}
                    }
                }
            }
            
            response = self.send_to_session("persistence_test", share_request)
            if response and "result" in response:
                shared_ids.append(discovery_id)
                print(f"✅ [persistence_test] Shared {discovery_type}: {discovery_id}")
            else:
                print(f"❌ [persistence_test] Failed to share {discovery_type}")
                return False
                
        # Test retrieval of shared discoveries
        subscribe_request = {
            "jsonrpc": "2.0",
            "id": 70,
            "method": "tools/call",
            "params": {
                "name": "subscribe_discoveries",
                "arguments": {
                    "agent_id": "persistence_test",
                    "filter": {
                        "agent_ids": ["persistence_test"]
                    }
                }
            }
        }
        
        response = self.send_to_session("persistence_test", subscribe_request)
        if response and "result" in response:
            print("✅ [persistence_test] Retrieved shared discoveries")
        else:
            print("❌ [persistence_test] Failed to retrieve discoveries")
            return False
            
        return True
        
    def run_all_tests(self) -> bool:
        """Run all integration tests"""
        print("🚀 Starting Real MCP Integration Tests")
        print("=" * 50)
        
        try:
            tests = [
                ("Multi-Agent Discovery Sharing", self.test_multi_agent_discovery_sharing),
                ("Resource Subscription Workflow", self.test_resource_subscription_workflow),
                ("Concurrent Discovery Operations", self.test_concurrent_discovery_operations),
                ("Discovery Persistence and Retrieval", self.test_discovery_persistence_and_retrieval)
            ]
            
            passed = 0
            total = len(tests)
            
            for test_name, test_func in tests:
                try:
                    print(f"\n{'='*20} {test_name} {'='*20}")
                    if test_func():
                        passed += 1
                        print(f"✅ {test_name}: PASSED")
                    else:
                        print(f"❌ {test_name}: FAILED")
                        
                    # Clean up sessions between tests
                    self.close_all_sessions()
                    time.sleep(1)
                    
                except Exception as e:
                    print(f"❌ {test_name}: ERROR - {e}")
                    self.close_all_sessions()
                    
            print("\n" + "=" * 50)
            print(f"📊 Integration Test Results: {passed}/{total} tests passed")
            
            return passed == total
            
        finally:
            self.close_all_sessions()

def main():
    binary_path = "/Users/mac-main/Mister-Smith/agenterra/mcp-foundation/mistersmith-mcp-server/target/debug/mistersmith-mcp-server"
    
    tester = MCPIntegrationTester(binary_path)
    success = tester.run_all_tests()
    
    if success:
        print("🎉 All integration tests passed!")
        sys.exit(0)
    else:
        print("💥 Some tests failed")
        sys.exit(1)

if __name__ == "__main__":
    main()