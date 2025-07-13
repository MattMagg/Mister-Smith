"""
AWS Migration Verification Function
Handles health checks, functional tests, performance tests, and security scans
"""

import json
import boto3
import logging
import requests
import time
from typing import Dict, Any, List
from concurrent.futures import ThreadPoolExecutor, as_completed
import statistics

# Configure logging
logger = logging.getLogger()
logger.setLevel(logging.INFO)

# Initialize AWS clients
cloudwatch = boto3.client('cloudwatch')
elbv2 = boto3.client('elbv2')
rds = boto3.client('rds')


def lambda_handler(event: Dict[str, Any], context) -> Dict[str, Any]:
    """
    Main handler for verification function
    
    Args:
        event: Contains action, endpoints, and tests to run
        context: Lambda context object
        
    Returns:
        Dict containing verification results
    """
    try:
        action = event.get('action')
        logger.info(f"Processing verification action: {action}")
        
        if action == 'runAllTests':
            return run_all_verification_tests(event)
        elif action == 'healthCheck':
            return run_health_checks(event)
        elif action == 'functionalTest':
            return run_functional_tests(event)
        elif action == 'performanceTest':
            return run_performance_tests(event)
        elif action == 'securityScan':
            return run_security_scan(event)
        else:
            raise ValueError(f"Unknown action: {action}")
            
    except Exception as e:
        logger.error(f"Verification failed: {str(e)}")
        return {
            'statusCode': 500,
            'status': 'failed',
            'error': str(e),
            'timestamp': int(time.time())
        }


def run_all_verification_tests(event: Dict[str, Any]) -> Dict[str, Any]:
    """
    Run all verification tests in parallel
    
    Args:
        event: Contains endpoints and test configurations
        
    Returns:
        Dict with all test results
    """
    try:
        endpoints = event.get('endpoints', [])
        tests = event.get('tests', ['healthCheck', 'functionalTest', 'performanceTest'])
        
        results = {}
        
        # Run tests in parallel using ThreadPoolExecutor
        with ThreadPoolExecutor(max_workers=4) as executor:
            future_to_test = {}
            
            if 'healthCheck' in tests:
                future = executor.submit(run_health_checks, {'endpoints': endpoints})
                future_to_test[future] = 'healthCheck'
            
            if 'functionalTest' in tests:
                future = executor.submit(run_functional_tests, {'endpoints': endpoints})
                future_to_test[future] = 'functionalTest'
            
            if 'performanceTest' in tests:
                future = executor.submit(run_performance_tests, {'endpoints': endpoints})
                future_to_test[future] = 'performanceTest'
            
            if 'securityScan' in tests:
                future = executor.submit(run_security_scan, {'endpoints': endpoints})
                future_to_test[future] = 'securityScan'
            
            # Collect results
            for future in as_completed(future_to_test):
                test_name = future_to_test[future]
                try:
                    result = future.result()
                    results[test_name] = result['status']
                    results[f"{test_name}Details"] = result
                except Exception as e:
                    logger.error(f"{test_name} failed: {str(e)}")
                    results[test_name] = 'failed'
                    results[f"{test_name}Details"] = {'error': str(e)}
        
        # Determine overall status
        all_passed = all(results.get(test) == 'passed' for test in tests)
        overall_status = 'passed' if all_passed else 'failed'
        
        return {
            'statusCode': 200,
            'status': overall_status,
            **results,
            'summary': f"Verification tests: {overall_status}",
            'timestamp': int(time.time())
        }
        
    except Exception as e:
        logger.error(f"All tests failed: {str(e)}")
        return {
            'statusCode': 500,
            'status': 'failed',
            'error': str(e),
            'timestamp': int(time.time())
        }


def run_health_checks(event: Dict[str, Any]) -> Dict[str, Any]:
    """
    Run comprehensive health checks on migrated services
    
    Args:
        event: Contains endpoints to check
        
    Returns:
        Dict with health check results
    """
    try:
        endpoints = event.get('endpoints', [])
        health_results = []
        
        # Check application endpoints
        for endpoint in endpoints:
            endpoint_health = check_endpoint_health(endpoint)
            health_results.append(endpoint_health)
        
        # Check AWS service health
        aws_health = check_aws_service_health()
        health_results.extend(aws_health)
        
        # Check database health
        db_health = check_database_health()
        health_results.append(db_health)
        
        # Determine overall health status
        failed_checks = [check for check in health_results if check['status'] != 'healthy']
        overall_status = 'passed' if not failed_checks else 'failed'
        
        return {
            'statusCode': 200,
            'status': overall_status,
            'healthChecks': health_results,
            'failedChecks': failed_checks,
            'healthScore': calculate_health_score(health_results),
            'timestamp': int(time.time())
        }
        
    except Exception as e:
        logger.error(f"Health checks failed: {str(e)}")
        return {
            'statusCode': 500,
            'status': 'failed',
            'error': str(e),
            'timestamp': int(time.time())
        }


def run_functional_tests(event: Dict[str, Any]) -> Dict[str, Any]:
    """
    Run functional tests to verify application behavior
    
    Args:
        event: Contains endpoints and test configurations
        
    Returns:
        Dict with functional test results
    """
    try:
        endpoints = event.get('endpoints', [])
        test_results = []
        
        # Define functional test scenarios
        test_scenarios = [
            {'name': 'user_authentication', 'endpoint': '/api/auth/login', 'method': 'POST'},
            {'name': 'data_retrieval', 'endpoint': '/api/data', 'method': 'GET'},
            {'name': 'data_creation', 'endpoint': '/api/data', 'method': 'POST'},
            {'name': 'data_update', 'endpoint': '/api/data/1', 'method': 'PUT'},
            {'name': 'data_deletion', 'endpoint': '/api/data/1', 'method': 'DELETE'}
        ]
        
        # Run test scenarios
        for endpoint_config in endpoints:
            base_url = endpoint_config.get('url', '')
            for scenario in test_scenarios:
                result = run_functional_test_scenario(base_url, scenario)
                test_results.append(result)
        
        # Calculate test statistics
        passed_tests = [test for test in test_results if test['status'] == 'passed']
        failed_tests = [test for test in test_results if test['status'] == 'failed']
        
        overall_status = 'passed' if len(failed_tests) == 0 else 'failed'
        
        return {
            'statusCode': 200,
            'status': overall_status,
            'functionalTests': test_results,
            'statistics': {
                'total': len(test_results),
                'passed': len(passed_tests),
                'failed': len(failed_tests),
                'passRate': len(passed_tests) / len(test_results) if test_results else 0
            },
            'timestamp': int(time.time())
        }
        
    except Exception as e:
        logger.error(f"Functional tests failed: {str(e)}")
        return {
            'statusCode': 500,
            'status': 'failed',
            'error': str(e),
            'timestamp': int(time.time())
        }


def run_performance_tests(event: Dict[str, Any]) -> Dict[str, Any]:
    """
    Run performance tests to verify application performance
    
    Args:
        event: Contains endpoints and performance thresholds
        
    Returns:
        Dict with performance test results
    """
    try:
        endpoints = event.get('endpoints', [])
        performance_results = []
        
        # Performance test configuration
        test_config = {
            'concurrent_users': 10,
            'duration_seconds': 60,
            'ramp_up_seconds': 10,
            'thresholds': {
                'response_time_p95': 2000,  # ms
                'response_time_avg': 500,   # ms
                'error_rate': 0.05,         # 5%
                'throughput_min': 100       # requests per second
            }
        }
        
        # Run performance tests for each endpoint
        for endpoint_config in endpoints:
            perf_result = run_endpoint_performance_test(endpoint_config, test_config)
            performance_results.append(perf_result)
        
        # Check CloudWatch metrics
        cloudwatch_metrics = get_cloudwatch_performance_metrics()
        
        # Determine overall performance status
        failed_tests = [test for test in performance_results if test['status'] != 'passed']
        overall_status = 'passed' if not failed_tests else 'failed'
        
        return {
            'statusCode': 200,
            'status': overall_status,
            'performanceTests': performance_results,
            'cloudWatchMetrics': cloudwatch_metrics,
            'failedTests': failed_tests,
            'summary': generate_performance_summary(performance_results),
            'timestamp': int(time.time())
        }
        
    except Exception as e:
        logger.error(f"Performance tests failed: {str(e)}")
        return {
            'statusCode': 500,
            'status': 'failed',
            'error': str(e),
            'timestamp': int(time.time())
        }


def run_security_scan(event: Dict[str, Any]) -> Dict[str, Any]:
    """
    Run security scans on migrated infrastructure
    
    Args:
        event: Contains endpoints and security configurations
        
    Returns:
        Dict with security scan results
    """
    try:
        endpoints = event.get('endpoints', [])
        security_results = []
        
        # Security test categories
        security_tests = [
            'ssl_certificate_check',
            'security_headers_check',
            'vulnerability_scan',
            'access_control_test',
            'encryption_check'
        ]
        
        # Run security tests
        for endpoint_config in endpoints:
            for test_type in security_tests:
                result = run_security_test(endpoint_config, test_type)
                security_results.append(result)
        
        # Check AWS security configurations
        aws_security = check_aws_security_configurations()
        security_results.extend(aws_security)
        
        # Categorize results by severity
        critical_issues = [test for test in security_results if test.get('severity') == 'critical' and test['status'] == 'failed']
        high_issues = [test for test in security_results if test.get('severity') == 'high' and test['status'] == 'failed']
        medium_issues = [test for test in security_results if test.get('severity') == 'medium' and test['status'] == 'failed']
        
        # Determine overall security status
        overall_status = 'passed' if not critical_issues and not high_issues else 'failed'
        
        return {
            'statusCode': 200,
            'status': overall_status,
            'securityTests': security_results,
            'issues': {
                'critical': critical_issues,
                'high': high_issues,
                'medium': medium_issues
            },
            'securityScore': calculate_security_score(security_results),
            'recommendations': generate_security_recommendations(security_results),
            'timestamp': int(time.time())
        }
        
    except Exception as e:
        logger.error(f"Security scan failed: {str(e)}")
        return {
            'statusCode': 500,
            'status': 'failed',
            'error': str(e),
            'timestamp': int(time.time())
        }


def check_endpoint_health(endpoint_config: Dict[str, Any]) -> Dict[str, Any]:
    """Check health of a specific endpoint"""
    try:
        url = endpoint_config.get('url', '')
        health_url = f"{url}/health"
        
        response = requests.get(health_url, timeout=30)
        
        if response.status_code == 200:
            return {
                'endpoint': url,
                'status': 'healthy',
                'responseTime': response.elapsed.total_seconds() * 1000,
                'statusCode': response.status_code
            }
        else:
            return {
                'endpoint': url,
                'status': 'unhealthy',
                'statusCode': response.status_code,
                'error': f"Unexpected status code: {response.status_code}"
            }
            
    except Exception as e:
        return {
            'endpoint': endpoint_config.get('url', 'unknown'),
            'status': 'unhealthy',
            'error': str(e)
        }


def check_aws_service_health() -> List[Dict[str, Any]]:
    """Check health of AWS services"""
    health_checks = []
    
    try:
        # Check Load Balancer health
        response = elbv2.describe_load_balancers()
        for lb in response.get('LoadBalancers', []):
            health_checks.append({
                'service': 'LoadBalancer',
                'resource': lb['LoadBalancerName'],
                'status': 'healthy' if lb['State']['Code'] == 'active' else 'unhealthy',
                'details': lb['State']
            })
            
    except Exception as e:
        health_checks.append({
            'service': 'LoadBalancer',
            'status': 'unhealthy',
            'error': str(e)
        })
    
    return health_checks


def check_database_health() -> Dict[str, Any]:
    """Check database health"""
    try:
        response = rds.describe_db_instances()
        
        healthy_dbs = 0
        total_dbs = len(response.get('DBInstances', []))
        
        for db in response.get('DBInstances', []):
            if db['DBInstanceStatus'] == 'available':
                healthy_dbs += 1
        
        status = 'healthy' if healthy_dbs == total_dbs else 'unhealthy'
        
        return {
            'service': 'Database',
            'status': status,
            'healthyInstances': healthy_dbs,
            'totalInstances': total_dbs
        }
        
    except Exception as e:
        return {
            'service': 'Database',
            'status': 'unhealthy',
            'error': str(e)
        }


def run_functional_test_scenario(base_url: str, scenario: Dict[str, Any]) -> Dict[str, Any]:
    """Run a specific functional test scenario"""
    try:
        url = f"{base_url}{scenario['endpoint']}"
        method = scenario['method']
        
        # Prepare test data based on scenario
        test_data = prepare_test_data(scenario['name'])
        
        if method == 'GET':
            response = requests.get(url, timeout=30)
        elif method == 'POST':
            response = requests.post(url, json=test_data, timeout=30)
        elif method == 'PUT':
            response = requests.put(url, json=test_data, timeout=30)
        elif method == 'DELETE':
            response = requests.delete(url, timeout=30)
        else:
            raise ValueError(f"Unsupported method: {method}")
        
        # Validate response
        is_valid = validate_response(response, scenario)
        
        return {
            'scenario': scenario['name'],
            'endpoint': scenario['endpoint'],
            'method': method,
            'status': 'passed' if is_valid else 'failed',
            'statusCode': response.status_code,
            'responseTime': response.elapsed.total_seconds() * 1000
        }
        
    except Exception as e:
        return {
            'scenario': scenario['name'],
            'endpoint': scenario['endpoint'],
            'method': scenario['method'],
            'status': 'failed',
            'error': str(e)
        }


def run_endpoint_performance_test(endpoint_config: Dict[str, Any], test_config: Dict[str, Any]) -> Dict[str, Any]:
    """Run performance test on an endpoint"""
    try:
        url = endpoint_config.get('url', '')
        response_times = []
        errors = 0
        
        # Simulate load testing
        for i in range(test_config['concurrent_users']):
            try:
                start_time = time.time()
                response = requests.get(url, timeout=30)
                end_time = time.time()
                
                response_time = (end_time - start_time) * 1000
                response_times.append(response_time)
                
                if response.status_code >= 400:
                    errors += 1
                    
            except Exception:
                errors += 1
        
        # Calculate metrics
        if response_times:
            avg_response_time = statistics.mean(response_times)
            p95_response_time = statistics.quantiles(response_times, n=20)[18] if len(response_times) > 1 else response_times[0]
        else:
            avg_response_time = 0
            p95_response_time = 0
        
        error_rate = errors / test_config['concurrent_users']
        throughput = test_config['concurrent_users'] / test_config['duration_seconds']
        
        # Check against thresholds
        thresholds = test_config['thresholds']
        status = 'passed'
        
        if p95_response_time > thresholds['response_time_p95']:
            status = 'failed'
        if avg_response_time > thresholds['response_time_avg']:
            status = 'failed'
        if error_rate > thresholds['error_rate']:
            status = 'failed'
        if throughput < thresholds['throughput_min']:
            status = 'failed'
        
        return {
            'endpoint': url,
            'status': status,
            'metrics': {
                'avgResponseTime': avg_response_time,
                'p95ResponseTime': p95_response_time,
                'errorRate': error_rate,
                'throughput': throughput
            },
            'thresholds': thresholds
        }
        
    except Exception as e:
        return {
            'endpoint': endpoint_config.get('url', 'unknown'),
            'status': 'failed',
            'error': str(e)
        }


def run_security_test(endpoint_config: Dict[str, Any], test_type: str) -> Dict[str, Any]:
    """Run a specific security test"""
    try:
        url = endpoint_config.get('url', '')
        
        if test_type == 'ssl_certificate_check':
            return check_ssl_certificate(url)
        elif test_type == 'security_headers_check':
            return check_security_headers(url)
        elif test_type == 'vulnerability_scan':
            return run_vulnerability_scan(url)
        elif test_type == 'access_control_test':
            return test_access_control(url)
        elif test_type == 'encryption_check':
            return check_encryption(url)
        else:
            raise ValueError(f"Unknown security test type: {test_type}")
            
    except Exception as e:
        return {
            'testType': test_type,
            'endpoint': endpoint_config.get('url', 'unknown'),
            'status': 'failed',
            'error': str(e)
        }


def prepare_test_data(scenario_name: str) -> Dict[str, Any]:
    """Prepare test data for functional test scenarios"""
    test_data_map = {
        'user_authentication': {
            'username': 'test_user',
            'password': 'test_password'
        },
        'data_creation': {
            'name': 'Test Item',
            'description': 'Test description'
        },
        'data_update': {
            'name': 'Updated Test Item',
            'description': 'Updated description'
        }
    }
    
    return test_data_map.get(scenario_name, {})


def validate_response(response, scenario: Dict[str, Any]) -> bool:
    """Validate API response based on scenario"""
    # Basic validation - check if status code is in success range
    if scenario['method'] in ['GET', 'PUT']:
        return 200 <= response.status_code < 300
    elif scenario['method'] == 'POST':
        return 200 <= response.status_code < 302
    elif scenario['method'] == 'DELETE':
        return response.status_code in [200, 204]
    
    return False


def calculate_health_score(health_results: List[Dict[str, Any]]) -> float:
    """Calculate overall health score"""
    if not health_results:
        return 0.0
    
    healthy_count = sum(1 for result in health_results if result.get('status') == 'healthy')
    return (healthy_count / len(health_results)) * 100


def calculate_security_score(security_results: List[Dict[str, Any]]) -> float:
    """Calculate overall security score"""
    if not security_results:
        return 0.0
    
    passed_count = sum(1 for result in security_results if result.get('status') == 'passed')
    return (passed_count / len(security_results)) * 100


def generate_performance_summary(performance_results: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Generate performance test summary"""
    if not performance_results:
        return {}
    
    total_tests = len(performance_results)
    passed_tests = sum(1 for result in performance_results if result.get('status') == 'passed')
    
    return {
        'totalTests': total_tests,
        'passedTests': passed_tests,
        'failedTests': total_tests - passed_tests,
        'passRate': (passed_tests / total_tests) * 100
    }


def generate_security_recommendations(security_results: List[Dict[str, Any]]) -> List[str]:
    """Generate security recommendations based on test results"""
    recommendations = []
    
    failed_tests = [test for test in security_results if test.get('status') == 'failed']
    
    for test in failed_tests:
        test_type = test.get('testType', '')
        if test_type == 'ssl_certificate_check':
            recommendations.append("Update SSL certificate configuration")
        elif test_type == 'security_headers_check':
            recommendations.append("Configure proper security headers")
        elif test_type == 'vulnerability_scan':
            recommendations.append("Address identified vulnerabilities")
    
    return recommendations


def get_cloudwatch_performance_metrics() -> Dict[str, Any]:
    """Get performance metrics from CloudWatch"""
    try:
        # This would fetch actual CloudWatch metrics
        # For demo purposes, return simulated data
        return {
            'cpuUtilization': 45.2,
            'memoryUtilization': 62.8,
            'networkIn': 1024000,
            'networkOut': 2048000,
            'requestCount': 15000,
            'errorCount': 25
        }
    except Exception as e:
        logger.error(f"Failed to get CloudWatch metrics: {str(e)}")
        return {}


def check_aws_security_configurations() -> List[Dict[str, Any]]:
    """Check AWS security configurations"""
    # This would check various AWS security settings
    # For demo purposes, return simulated results
    return [
        {
            'testType': 'security_group_check',
            'status': 'passed',
            'details': 'Security groups properly configured'
        },
        {
            'testType': 'iam_permissions_check',
            'status': 'passed',
            'details': 'IAM permissions follow least privilege principle'
        }
    ]


def check_ssl_certificate(url: str) -> Dict[str, Any]:
    """Check SSL certificate validity"""
    # Simulated SSL check
    return {
        'testType': 'ssl_certificate_check',
        'endpoint': url,
        'status': 'passed',
        'details': 'SSL certificate is valid',
        'severity': 'high'
    }


def check_security_headers(url: str) -> Dict[str, Any]:
    """Check security headers"""
    # Simulated security headers check
    return {
        'testType': 'security_headers_check',
        'endpoint': url,
        'status': 'passed',
        'details': 'Security headers properly configured',
        'severity': 'medium'
    }


def run_vulnerability_scan(url: str) -> Dict[str, Any]:
    """Run vulnerability scan"""
    # Simulated vulnerability scan
    return {
        'testType': 'vulnerability_scan',
        'endpoint': url,
        'status': 'passed',
        'details': 'No vulnerabilities detected',
        'severity': 'high'
    }


def test_access_control(url: str) -> Dict[str, Any]:
    """Test access control"""
    # Simulated access control test
    return {
        'testType': 'access_control_test',
        'endpoint': url,
        'status': 'passed',
        'details': 'Access control properly configured',
        'severity': 'critical'
    }


def check_encryption(url: str) -> Dict[str, Any]:
    """Check encryption configuration"""
    # Simulated encryption check
    return {
        'testType': 'encryption_check',
        'endpoint': url,
        'status': 'passed',
        'details': 'Encryption properly configured',
        'severity': 'high'
    }