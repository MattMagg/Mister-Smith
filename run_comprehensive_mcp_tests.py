#!/usr/bin/env python3
"""
Comprehensive MCP Server Test Runner

Executes all MCP server tests and provides detailed reporting.
"""

import subprocess
import sys
import time
import json
from datetime import datetime
from pathlib import Path

class MCPTestRunner:
    def __init__(self):
        self.binary_path = "/Users/mac-main/Mister-Smith/agenterra/mcp-foundation/mistersmith-mcp-server/target/debug/mistersmith-mcp-server"
        self.test_scripts = [
            ("Protocol Compliance", "test_mcp_protocol_compliance.py"),
            ("Discovery Tools", "test_discovery_tools.py"),
            ("Integration Tests", "test_real_mcp_integration.py")
        ]
        self.results = []
        
    def check_prerequisites(self) -> bool:
        """Check if MCP server binary exists and is executable"""
        print("🔍 Checking prerequisites...")
        
        binary_path = Path(self.binary_path)
        if not binary_path.exists():
            print(f"❌ MCP server binary not found at: {self.binary_path}")
            return False
            
        if not binary_path.is_file():
            print(f"❌ MCP server binary is not a file: {self.binary_path}")
            return False
            
        # Check if executable
        try:
            result = subprocess.run([self.binary_path, "--help"], 
                                  capture_output=True, text=True, timeout=10)
            if result.returncode != 0:
                print(f"❌ MCP server binary is not executable or has issues")
                return False
        except subprocess.TimeoutExpired:
            print("⚠️  MCP server binary doesn't support --help (this is okay)")
        except Exception as e:
            print(f"❌ Error testing MCP server binary: {e}")
            return False
            
        print(f"✅ MCP server binary found and accessible: {self.binary_path}")
        return True
        
    def run_test_script(self, test_name: str, script_name: str) -> dict:
        """Run a single test script and capture results"""
        print(f"\n{'='*20} {test_name} {'='*20}")
        print(f"🚀 Running {script_name}...")
        
        start_time = time.time()
        
        try:
            result = subprocess.run([sys.executable, script_name], 
                                  capture_output=True, text=True, timeout=300)
            
            end_time = time.time()
            duration = end_time - start_time
            
            test_result = {
                "test_name": test_name,
                "script_name": script_name,
                "duration": duration,
                "return_code": result.returncode,
                "stdout": result.stdout,
                "stderr": result.stderr,
                "success": result.returncode == 0,
                "timestamp": datetime.now().isoformat()
            }
            
            if result.returncode == 0:
                print(f"✅ {test_name}: PASSED ({duration:.2f}s)")
            else:
                print(f"❌ {test_name}: FAILED ({duration:.2f}s)")
                if result.stderr:
                    print(f"Error output: {result.stderr}")
                    
            return test_result
            
        except subprocess.TimeoutExpired:
            print(f"⏰ {test_name}: TIMEOUT (5 minutes)")
            return {
                "test_name": test_name,
                "script_name": script_name,
                "duration": 300,
                "return_code": -1,
                "stdout": "",
                "stderr": "Test timed out after 5 minutes",
                "success": False,
                "timestamp": datetime.now().isoformat()
            }
            
        except Exception as e:
            print(f"💥 {test_name}: ERROR - {e}")
            return {
                "test_name": test_name,
                "script_name": script_name,
                "duration": 0,
                "return_code": -1,
                "stdout": "",
                "stderr": str(e),
                "success": False,
                "timestamp": datetime.now().isoformat()
            }
            
    def generate_report(self) -> str:
        """Generate comprehensive test report"""
        total_tests = len(self.results)
        passed_tests = sum(1 for r in self.results if r["success"])
        failed_tests = total_tests - passed_tests
        
        total_duration = sum(r["duration"] for r in self.results)
        
        report = f"""
# MisterSmith MCP Server Test Report

**Generated**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}
**MCP Server Binary**: {self.binary_path}
**Total Test Duration**: {total_duration:.2f} seconds

## Summary

- **Total Tests**: {total_tests}
- **Passed**: {passed_tests} ✅
- **Failed**: {failed_tests} ❌
- **Success Rate**: {(passed_tests/total_tests*100):.1f}%

## Test Results

"""
        
        for result in self.results:
            status = "✅ PASSED" if result["success"] else "❌ FAILED"
            report += f"""
### {result["test_name"]} {status}

- **Script**: {result["script_name"]}
- **Duration**: {result["duration"]:.2f}s
- **Return Code**: {result["return_code"]}

"""
            
            if result["stdout"]:
                report += f"""
**Output**:
```
{result["stdout"]}
```
"""
                
            if result["stderr"]:
                report += f"""
**Error Output**:
```
{result["stderr"]}
```
"""
                
        # Add technical analysis
        report += f"""
## Technical Analysis

### Protocol Compliance
{self.analyze_protocol_compliance()}

### Discovery Tools Functionality
{self.analyze_discovery_tools()}

### Integration Capabilities
{self.analyze_integration()}

### Recommendations
{self.generate_recommendations()}

## Conclusion

{self.generate_conclusion()}
"""
        
        return report
        
    def analyze_protocol_compliance(self) -> str:
        """Analyze protocol compliance results"""
        protocol_result = next((r for r in self.results if "Protocol" in r["test_name"]), None)
        if not protocol_result:
            return "❌ Protocol compliance tests were not run"
            
        if protocol_result["success"]:
            return "✅ MCP server demonstrates full JSON-RPC 2.0 compliance with proper initialization handshake, ping utilities, and error handling"
        else:
            return "❌ MCP server has protocol compliance issues that need attention"
            
    def analyze_discovery_tools(self) -> str:
        """Analyze discovery tools results"""
        discovery_result = next((r for r in self.results if "Discovery" in r["test_name"]), None)
        if not discovery_result:
            return "❌ Discovery tools tests were not run"
            
        if discovery_result["success"]:
            return "✅ Discovery tools (share_discovery, subscribe_discoveries) are fully functional with proper validation and resource notifications"
        else:
            return "❌ Discovery tools have functionality issues that need resolution"
            
    def analyze_integration(self) -> str:
        """Analyze integration test results"""
        integration_result = next((r for r in self.results if "Integration" in r["test_name"]), None)
        if not integration_result:
            return "❌ Integration tests were not run"
            
        if integration_result["success"]:
            return "✅ Real-world integration scenarios work correctly including multi-agent collaboration, resource subscriptions, and concurrent operations"
        else:
            return "❌ Integration capabilities need improvement for production use"
            
    def generate_recommendations(self) -> str:
        """Generate recommendations based on test results"""
        recommendations = []
        
        failed_tests = [r for r in self.results if not r["success"]]
        if not failed_tests:
            recommendations.append("✅ All tests passed - MCP server is ready for production use")
        else:
            recommendations.append("❌ Address failing tests before production deployment")
            
        if any("timeout" in r["stderr"].lower() for r in failed_tests):
            recommendations.append("⚠️  Performance optimization may be needed for timeout issues")
            
        if any("error" in r["stderr"].lower() for r in failed_tests):
            recommendations.append("🔧 Review error handling and validation logic")
            
        return "\n".join(f"- {rec}" for rec in recommendations)
        
    def generate_conclusion(self) -> str:
        """Generate overall conclusion"""
        passed_count = sum(1 for r in self.results if r["success"])
        total_count = len(self.results)
        
        if passed_count == total_count:
            return "🎉 **SUCCESS**: MisterSmith MCP server demonstrates full functionality and protocol compliance. Ready for real-world deployment."
        elif passed_count > total_count / 2:
            return "⚠️  **PARTIAL SUCCESS**: Most tests passed but some issues need attention before production use."
        else:
            return "❌ **FAILURE**: Significant issues detected. Server needs substantial fixes before deployment."
            
    def run_all_tests(self) -> bool:
        """Run all test scripts and generate report"""
        print("🚀 Starting Comprehensive MCP Server Testing")
        print("=" * 60)
        
        if not self.check_prerequisites():
            return False
            
        # Run each test script
        for test_name, script_name in self.test_scripts:
            result = self.run_test_script(test_name, script_name)
            self.results.append(result)
            
        # Generate and save report
        report = self.generate_report()
        
        # Save report to file
        report_file = f"mcp_test_report_{datetime.now().strftime('%Y%m%d_%H%M%S')}.md"
        with open(report_file, 'w') as f:
            f.write(report)
            
        print(f"\n📄 Detailed report saved to: {report_file}")
        
        # Print summary
        passed_count = sum(1 for r in self.results if r["success"])
        total_count = len(self.results)
        
        print("\n" + "=" * 60)
        print(f"📊 FINAL RESULTS: {passed_count}/{total_count} tests passed")
        
        if passed_count == total_count:
            print("🎉 ALL TESTS PASSED! MCP server is fully functional.")
            return True
        else:
            print("💥 SOME TESTS FAILED. See report for details.")
            return False

def main():
    runner = MCPTestRunner()
    success = runner.run_all_tests()
    
    if success:
        sys.exit(0)
    else:
        sys.exit(1)

if __name__ == "__main__":
    main()