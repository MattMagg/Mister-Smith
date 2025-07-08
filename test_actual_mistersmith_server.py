#!/usr/bin/env python3
"""
Test the actual MisterSmith discovery server implementation
"""

import asyncio
import subprocess
import sys
import time
import json
import requests
from datetime import datetime

class MisterSmithServerTester:
    def __init__(self):
        self.server_process = None
        self.server_url = "http://127.0.0.1:8080"
        
    def start_nats_server(self):
        """Start NATS server if not running"""
        try:
            # Check if NATS is already running
            result = subprocess.run(["pgrep", "-f", "nats-server"], 
                                  capture_output=True, text=True)
            if result.returncode == 0:
                print("✅ NATS server already running")
                return True
        except:
            pass
            
        # Try to start NATS server
        try:
            subprocess.Popen(["nats-server"], 
                           stdout=subprocess.DEVNULL, 
                           stderr=subprocess.DEVNULL)
            time.sleep(2)  # Give time to start
            print("✅ NATS server started")
            return True
        except Exception as e:
            print(f"❌ Failed to start NATS server: {e}")
            return False
            
    def run_discovery_server_test(self):
        """Run the MisterSmith discovery server test"""
        try:
            print("🚀 Running MisterSmith discovery server test...")
            
            # Run the MCP discovery server test
            result = subprocess.run([
                "cargo", "test", "--test", "mcp_discovery_server", 
                "test_mcp_server_initialization", "--", "--nocapture"
            ], capture_output=True, text=True, timeout=60)
            
            if result.returncode == 0:
                print("✅ MisterSmith discovery server test PASSED")
                print(f"Output: {result.stdout}")
                return True
            else:
                print("❌ MisterSmith discovery server test FAILED")
                print(f"Error: {result.stderr}")
                return False
                
        except subprocess.TimeoutExpired:
            print("⏰ Test timed out after 60 seconds")
            return False
        except Exception as e:
            print(f"❌ Test execution error: {e}")
            return False
            
    def run_discovery_tools_test(self):
        """Run the discovery tools test"""
        try:
            print("🔍 Running discovery tools test...")
            
            # Run the tools test
            result = subprocess.run([
                "cargo", "test", "--test", "mcp_discovery_server", 
                "test_mcp_tools", "--", "--nocapture"
            ], capture_output=True, text=True, timeout=60)
            
            if result.returncode == 0:
                print("✅ Discovery tools test PASSED")
                print(f"Output: {result.stdout}")
                return True
            else:
                print("❌ Discovery tools test FAILED")
                print(f"Error: {result.stderr}")
                return False
                
        except subprocess.TimeoutExpired:
            print("⏰ Test timed out after 60 seconds")
            return False
        except Exception as e:
            print(f"❌ Test execution error: {e}")
            return False
            
    def run_discovery_sharing_test(self):
        """Run the discovery sharing flow test"""
        try:
            print("🔄 Running discovery sharing flow test...")
            
            # Run the sharing test
            result = subprocess.run([
                "cargo", "test", "--test", "mcp_discovery_server", 
                "test_discovery_sharing_flow", "--", "--nocapture"
            ], capture_output=True, text=True, timeout=60)
            
            if result.returncode == 0:
                print("✅ Discovery sharing flow test PASSED")
                print(f"Output: {result.stdout}")
                return True
            else:
                print("❌ Discovery sharing flow test FAILED")
                print(f"Error: {result.stderr}")
                return False
                
        except subprocess.TimeoutExpired:
            print("⏰ Test timed out after 60 seconds")
            return False
        except Exception as e:
            print(f"❌ Test execution error: {e}")
            return False
            
    def verify_discovery_components(self):
        """Verify that discovery components exist"""
        try:
            print("🔍 Verifying discovery components...")
            
            # Check if discovery components compile
            result = subprocess.run([
                "cargo", "check", "--tests"
            ], capture_output=True, text=True, timeout=30)
            
            if result.returncode == 0:
                print("✅ Discovery components compile successfully")
                return True
            else:
                print("❌ Discovery components have compilation errors")
                print(f"Error: {result.stderr}")
                return False
                
        except subprocess.TimeoutExpired:
            print("⏰ Compilation check timed out")
            return False
        except Exception as e:
            print(f"❌ Compilation check error: {e}")
            return False
            
    def run_comprehensive_test(self):
        """Run comprehensive test suite"""
        print("🚀 Starting Comprehensive MisterSmith Discovery Server Tests")
        print("=" * 70)
        
        # Check prerequisites
        if not self.start_nats_server():
            print("❌ NATS server requirement not met")
            return False
            
        # Run tests
        tests = [
            ("Component Verification", self.verify_discovery_components),
            ("Server Initialization", self.run_discovery_server_test),
            ("Discovery Tools", self.run_discovery_tools_test),
            ("Discovery Sharing Flow", self.run_discovery_sharing_test),
        ]
        
        passed = 0
        total = len(tests)
        
        for test_name, test_func in tests:
            print(f"\n{'='*20} {test_name} {'='*20}")
            try:
                if test_func():
                    passed += 1
                    print(f"✅ {test_name}: PASSED")
                else:
                    print(f"❌ {test_name}: FAILED")
            except Exception as e:
                print(f"❌ {test_name}: ERROR - {e}")
                
        print(f"\n{'='*70}")
        print(f"📊 Final Results: {passed}/{total} tests passed")
        
        if passed == total:
            print("🎉 All tests passed! MisterSmith discovery server is functional.")
            return True
        else:
            print("💥 Some tests failed. See details above.")
            return False

def main():
    tester = MisterSmithServerTester()
    success = tester.run_comprehensive_test()
    
    if success:
        print("\n🎯 CONCLUSION: MisterSmith MCP discovery server is working correctly!")
        print("   - All discovery tools are functional")
        print("   - Real-time discovery sharing is operational")
        print("   - MCP protocol compliance is verified")
        sys.exit(0)
    else:
        print("\n💥 CONCLUSION: Issues found with MisterSmith discovery server")
        print("   - Check NATS server availability")
        print("   - Verify dependency installation")
        print("   - Review error messages above")
        sys.exit(1)

if __name__ == "__main__":
    main()