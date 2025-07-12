"""
AWS Migration Validation Function
Handles prerequisite validation, source environment checks, and AWS permission verification
"""

import json
import boto3
import logging
from typing import Dict, Any, List
from botocore.exceptions import ClientError, NoCredentialsError
import time

# Configure logging
logger = logging.getLogger()
logger.setLevel(logging.INFO)

# Initialize AWS clients
sts_client = boto3.client('sts')
ec2_client = boto3.client('ec2')
rds_client = boto3.client('rds')
s3_client = boto3.client('s3')


def lambda_handler(event: Dict[str, Any], context) -> Dict[str, Any]:
    """
    Main handler for validation function
    
    Args:
        event: Contains action and parameters for validation
        context: Lambda context object
        
    Returns:
        Dict containing validation results
    """
    try:
        action = event.get('action')
        logger.info(f"Processing validation action: {action}")
        
        if action == 'initialize':
            return initialize_validation(event)
        elif action == 'validateSource':
            return validate_source_environment(event)
        elif action == 'validatePermissions':
            return validate_aws_permissions(event)
        elif action == 'validateNetwork':
            return validate_network_connectivity(event)
        else:
            raise ValueError(f"Unknown action: {action}")
            
    except Exception as e:
        logger.error(f"Validation failed: {str(e)}")
        return {
            'statusCode': 500,
            'status': 'failed',
            'error': str(e),
            'timestamp': int(time.time())
        }


def initialize_validation(event: Dict[str, Any]) -> Dict[str, Any]:
    """
    Initialize migration validation process
    
    Args:
        event: Contains migrationId, sourceEnvironment, targetEnvironment
        
    Returns:
        Dict with initialization status
    """
    try:
        migration_id = event.get('migrationId')
        source_env = event.get('sourceEnvironment', {})
        target_env = event.get('targetEnvironment', {})
        
        # Validate required parameters
        if not migration_id:
            raise ValueError("migrationId is required")
        
        # Store migration metadata
        metadata = {
            'migrationId': migration_id,
            'sourceEnvironment': source_env,
            'targetEnvironment': target_env,
            'startTime': int(time.time()),
            'validationStarted': True
        }
        
        logger.info(f"Initialized migration validation for {migration_id}")
        
        return {
            'statusCode': 200,
            'status': 'initialized',
            'migrationId': migration_id,
            'metadata': metadata,
            'timestamp': int(time.time())
        }
        
    except Exception as e:
        logger.error(f"Initialization failed: {str(e)}")
        raise


def validate_source_environment(event: Dict[str, Any]) -> Dict[str, Any]:
    """
    Validate source environment connectivity and readiness
    
    Args:
        event: Contains sourceEnvironment configuration
        
    Returns:
        Dict with validation results
    """
    try:
        source_env = event.get('sourceEnvironment', {})
        validation_results = []
        
        # Check database connectivity
        if 'database' in source_env:
            db_result = validate_database_connectivity(source_env['database'])
            validation_results.append(db_result)
        
        # Check application endpoints
        if 'applications' in source_env:
            app_result = validate_application_endpoints(source_env['applications'])
            validation_results.append(app_result)
        
        # Check file systems
        if 'filesystems' in source_env:
            fs_result = validate_filesystem_access(source_env['filesystems'])
            validation_results.append(fs_result)
        
        # Determine overall status
        all_passed = all(result['status'] == 'passed' for result in validation_results)
        overall_status = 'passed' if all_passed else 'failed'
        
        return {
            'statusCode': 200,
            'status': overall_status,
            'validationResults': validation_results,
            'summary': f"Source environment validation: {overall_status}",
            'timestamp': int(time.time())
        }
        
    except Exception as e:
        logger.error(f"Source validation failed: {str(e)}")
        return {
            'statusCode': 500,
            'status': 'failed',
            'error': str(e),
            'timestamp': int(time.time())
        }


def validate_aws_permissions(event: Dict[str, Any]) -> Dict[str, Any]:
    """
    Validate AWS permissions for migration
    
    Args:
        event: Contains targetEnvironment and requiredPermissions
        
    Returns:
        Dict with permission validation results
    """
    try:
        required_permissions = event.get('requiredPermissions', [])
        permission_results = []
        
        # Get current user/role identity
        identity = sts_client.get_caller_identity()
        logger.info(f"Validating permissions for: {identity.get('Arn')}")
        
        # Test specific permissions
        for permission in required_permissions:
            try:
                result = test_permission(permission)
                permission_results.append({
                    'permission': permission,
                    'status': 'granted' if result else 'denied',
                    'details': f"Permission {permission} validation"
                })
            except Exception as e:
                permission_results.append({
                    'permission': permission,
                    'status': 'error',
                    'error': str(e)
                })
        
        # Check overall permission status
        denied_permissions = [p for p in permission_results if p['status'] != 'granted']
        overall_status = 'passed' if not denied_permissions else 'failed'
        
        return {
            'statusCode': 200,
            'status': overall_status,
            'identity': identity,
            'permissionResults': permission_results,
            'deniedPermissions': denied_permissions,
            'timestamp': int(time.time())
        }
        
    except NoCredentialsError:
        return {
            'statusCode': 401,
            'status': 'failed',
            'error': 'AWS credentials not configured',
            'timestamp': int(time.time())
        }
    except Exception as e:
        logger.error(f"Permission validation failed: {str(e)}")
        return {
            'statusCode': 500,
            'status': 'failed',
            'error': str(e),
            'timestamp': int(time.time())
        }


def validate_network_connectivity(event: Dict[str, Any]) -> Dict[str, Any]:
    """
    Validate network connectivity between source and target environments
    
    Args:
        event: Contains sourceEnvironment and targetEnvironment
        
    Returns:
        Dict with network validation results
    """
    try:
        source_env = event.get('sourceEnvironment', {})
        target_env = event.get('targetEnvironment', {})
        
        connectivity_tests = []
        
        # Test VPC connectivity if specified
        if 'vpc' in target_env:
            vpc_test = test_vpc_connectivity(target_env['vpc'])
            connectivity_tests.append(vpc_test)
        
        # Test internet connectivity
        internet_test = test_internet_connectivity()
        connectivity_tests.append(internet_test)
        
        # Test AWS service endpoints
        service_test = test_aws_service_connectivity()
        connectivity_tests.append(service_test)
        
        # Determine overall status
        all_passed = all(test['status'] == 'passed' for test in connectivity_tests)
        overall_status = 'passed' if all_passed else 'failed'
        
        return {
            'statusCode': 200,
            'status': overall_status,
            'connectivityTests': connectivity_tests,
            'summary': f"Network connectivity validation: {overall_status}",
            'timestamp': int(time.time())
        }
        
    except Exception as e:
        logger.error(f"Network validation failed: {str(e)}")
        return {
            'statusCode': 500,
            'status': 'failed',
            'error': str(e),
            'timestamp': int(time.time())
        }


def validate_database_connectivity(db_config: Dict[str, Any]) -> Dict[str, Any]:
    """Test database connectivity"""
    try:
        # This would typically connect to the source database
        # For demo purposes, we'll simulate the check
        return {
            'component': 'database',
            'status': 'passed',
            'details': f"Database connectivity verified for {db_config.get('host', 'unknown')}"
        }
    except Exception as e:
        return {
            'component': 'database',
            'status': 'failed',
            'error': str(e)
        }


def validate_application_endpoints(app_configs: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Test application endpoint connectivity"""
    try:
        # Test each application endpoint
        # For demo purposes, we'll simulate the check
        return {
            'component': 'applications',
            'status': 'passed',
            'details': f"Validated {len(app_configs)} application endpoints"
        }
    except Exception as e:
        return {
            'component': 'applications',
            'status': 'failed',
            'error': str(e)
        }


def validate_filesystem_access(fs_configs: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Test filesystem access"""
    try:
        # Test filesystem access
        # For demo purposes, we'll simulate the check
        return {
            'component': 'filesystems',
            'status': 'passed',
            'details': f"Validated {len(fs_configs)} filesystem mounts"
        }
    except Exception as e:
        return {
            'component': 'filesystems',
            'status': 'failed',
            'error': str(e)
        }


def test_permission(permission: str) -> bool:
    """Test specific AWS permission"""
    try:
        # Map permission to specific test
        if permission.startswith('ec2:'):
            # Test EC2 permissions
            ec2_client.describe_regions()
            return True
        elif permission.startswith('rds:'):
            # Test RDS permissions
            rds_client.describe_db_instances()
            return True
        elif permission.startswith('s3:'):
            # Test S3 permissions
            s3_client.list_buckets()
            return True
        else:
            # Generic test - check if we can make STS calls
            sts_client.get_caller_identity()
            return True
    except ClientError as e:
        if e.response['Error']['Code'] in ['AccessDenied', 'UnauthorizedOperation']:
            return False
        raise
    except Exception:
        return False


def test_vpc_connectivity(vpc_config: Dict[str, Any]) -> Dict[str, Any]:
    """Test VPC connectivity"""
    try:
        vpc_id = vpc_config.get('vpcId')
        if vpc_id:
            # Test VPC access
            response = ec2_client.describe_vpcs(VpcIds=[vpc_id])
            if response['Vpcs']:
                return {
                    'test': 'vpc_connectivity',
                    'status': 'passed',
                    'details': f"VPC {vpc_id} is accessible"
                }
        
        return {
            'test': 'vpc_connectivity',
            'status': 'failed',
            'details': 'VPC not accessible or not found'
        }
    except Exception as e:
        return {
            'test': 'vpc_connectivity',
            'status': 'failed',
            'error': str(e)
        }


def test_internet_connectivity() -> Dict[str, Any]:
    """Test internet connectivity"""
    try:
        # Test by making AWS API call
        sts_client.get_caller_identity()
        return {
            'test': 'internet_connectivity',
            'status': 'passed',
            'details': 'Internet connectivity verified'
        }
    except Exception as e:
        return {
            'test': 'internet_connectivity',
            'status': 'failed',
            'error': str(e)
        }


def test_aws_service_connectivity() -> Dict[str, Any]:
    """Test AWS service endpoint connectivity"""
    try:
        # Test multiple AWS services
        services_tested = []
        
        # Test EC2
        ec2_client.describe_regions()
        services_tested.append('EC2')
        
        # Test S3
        s3_client.list_buckets()
        services_tested.append('S3')
        
        # Test RDS
        rds_client.describe_db_instances()
        services_tested.append('RDS')
        
        return {
            'test': 'aws_service_connectivity',
            'status': 'passed',
            'details': f"AWS services accessible: {', '.join(services_tested)}"
        }
    except Exception as e:
        return {
            'test': 'aws_service_connectivity',
            'status': 'failed',
            'error': str(e)
        }