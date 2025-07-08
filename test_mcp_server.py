#!/usr/bin/env python3
"""Test MCP server with discovery tools using JSON-RPC 2.0 protocol"""

import json
import subprocess
import sys

def send_request(proc, request):
    """Send a JSON-RPC request to the MCP server"""
    request_str = json.dumps(request) + '\n'
    proc.stdin.write(request_str.encode())
    proc.stdin.flush()
    
    # Read response
    response_line = proc.stdout.readline().decode()
    return json.loads(response_line)

def test_mcp_server():
    """Test the MCP server with discovery tools"""
    print("Starting MCP server test...")
    
    # Start the MCP server
    proc = subprocess.Popen(
        ['/Users/mac-main/.cargo/mistersmith-target/debug/mistersmith-mcp-server'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=False
    )
    
    try:
        # Test 1: Initialize
        print("\n1. Testing initialize...")
        init_request = {
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                },
                "capabilities": {}
            },
            "id": 1
        }
        response = send_request(proc, init_request)
        print(f"Initialize response: {json.dumps(response, indent=2)}")
        
        # Test 2: List tools
        print("\n2. Testing tools/list...")
        tools_request = {
            "jsonrpc": "2.0",
            "method": "tools/list",
            "params": {},
            "id": 2
        }
        response = send_request(proc, tools_request)
        print(f"Available tools: {json.dumps(response, indent=2)}")
        
        # Check if discovery tools are present
        if 'result' in response and 'tools' in response['result']:
            tools = response['result']['tools']
            discovery_tools = [t for t in tools if 'discovery' in t.get('name', '')]
            print(f"\nDiscovery tools found: {len(discovery_tools)}")
            for tool in discovery_tools:
                print(f"  - {tool.get('name')}: {tool.get('description', '')}")
        
        # Test 3: Call ping tool
        print("\n3. Testing ping tool...")
        ping_request = {
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "ping",
                "arguments": {}
            },
            "id": 3
        }
        response = send_request(proc, ping_request)
        print(f"Ping response: {json.dumps(response, indent=2)}")
        
        # Test 4: Call share_discovery tool
        print("\n4. Testing share_discovery tool...")
        share_request = {
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "share_discovery",
                "arguments": {
                    "discovery_type": "insight",
                    "content": "Test discovery from Python test script",
                    "confidence": 0.85,
                    "agent_id": "test-python-agent",
                    "agent_role": "Tester"
                }
            },
            "id": 4
        }
        response = send_request(proc, share_request)
        print(f"Share discovery response: {json.dumps(response, indent=2)}")
        
    except Exception as e:
        print(f"Error: {e}")
        # Print stderr if available
        stderr = proc.stderr.read().decode()
        if stderr:
            print(f"Server stderr: {stderr}")
    finally:
        # Terminate the server
        proc.terminate()
        proc.wait()

if __name__ == "__main__":
    test_mcp_server()