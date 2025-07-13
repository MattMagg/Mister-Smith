import json
import boto3
from datetime import datetime, timedelta
from decimal import Decimal
import os

# Initialize AWS clients
cloudwatch = boto3.client('cloudwatch')
dynamodb = boto3.resource('dynamodb')

# Environment variables
TABLE_NAME = os.environ.get('MIGRATION_TABLE', 'MigrationProgress')
NAMESPACE = os.environ.get('METRIC_NAMESPACE', 'AWS/MigrationTracker')

def lambda_handler(event, context):
    """
    Calculate derived metrics from raw migration data and publish to CloudWatch
    """
    try:
        table = dynamodb.Table(TABLE_NAME)
        
        # Get current phase data
        response = table.scan()
        items = response['Items']
        
        metrics = []
        
        for item in items:
            phase = item.get('Phase')
            phase_metrics = calculate_phase_metrics(item)
            metrics.extend(phase_metrics)
            
        # Calculate aggregate metrics
        aggregate_metrics = calculate_aggregate_metrics(items)
        metrics.extend(aggregate_metrics)
        
        # Publish metrics to CloudWatch in batches
        publish_metrics(metrics)
        
        return {
            'statusCode': 200,
            'body': json.dumps({
                'message': f'Successfully published {len(metrics)} metrics',
                'timestamp': datetime.now().isoformat()
            })
        }
        
    except Exception as e:
        print(f"Error in metric calculation: {str(e)}")
        return {
            'statusCode': 500,
            'body': json.dumps({
                'error': str(e),
                'timestamp': datetime.now().isoformat()
            })
        }

def calculate_phase_metrics(phase_data):
    """Calculate metrics for a specific phase"""
    metrics = []
    phase = phase_data.get('Phase')
    
    # Phase completion percentage
    total_tasks = int(phase_data.get('TotalTasks', 0))
    completed_tasks = int(phase_data.get('CompletedTasks', 0))
    
    if total_tasks > 0:
        completion_percentage = (completed_tasks / total_tasks) * 100
        
        metrics.append({
            'MetricName': 'PhaseCompletionPercentage',
            'Value': float(completion_percentage),
            'Unit': 'Percent',
            'Dimensions': [
                {'Name': 'Phase', 'Value': str(phase)},
                {'Name': 'Environment', 'Value': phase_data.get('Environment', 'production')}
            ],
            'Timestamp': datetime.now()
        })
    
    # Sub-phase status
    for sub_phase in phase_data.get('SubPhases', []):
        status_map = {
            'pending': 0,
            'in-progress': 1,
            'completed': 2,
            'blocked': 3
        }
        
        metrics.append({
            'MetricName': 'SubPhaseStatus',
            'Value': float(status_map.get(sub_phase.get('Status', 'pending'), 0)),
            'Unit': 'None',
            'Dimensions': [
                {'Name': 'Phase', 'Value': str(phase)},
                {'Name': 'SubPhase', 'Value': sub_phase.get('Name')},
                {'Name': 'Component', 'Value': sub_phase.get('Component', 'General')}
            ],
            'Timestamp': datetime.now()
        })
    
    # Days elapsed and remaining
    start_date = phase_data.get('StartDate')
    if start_date:
        start_datetime = datetime.fromisoformat(start_date)
        days_elapsed = (datetime.now() - start_datetime).days
        
        metrics.append({
            'MetricName': 'DaysElapsed',
            'Value': float(days_elapsed),
            'Unit': 'Count',
            'Dimensions': [
                {'Name': 'Phase', 'Value': str(phase)}
            ],
            'Timestamp': datetime.now()
        })
        
        # Estimate days remaining based on velocity
        if completion_percentage > 0 and days_elapsed > 0:
            velocity = completion_percentage / days_elapsed
            if velocity > 0:
                days_remaining = (100 - completion_percentage) / velocity
                
                metrics.append({
                    'MetricName': 'DaysRemaining',
                    'Value': float(days_remaining),
                    'Unit': 'Count',
                    'Dimensions': [
                        {'Name': 'Phase', 'Value': str(phase)}
                    ],
                    'Timestamp': datetime.now()
                })
    
    # Blocker count by severity
    blockers = phase_data.get('Blockers', [])
    blocker_counts = {'Critical': 0, 'High': 0, 'Medium': 0, 'Low': 0}
    
    for blocker in blockers:
        severity = blocker.get('Severity', 'Medium')
        blocker_counts[severity] = blocker_counts.get(severity, 0) + 1
    
    for severity, count in blocker_counts.items():
        metrics.append({
            'MetricName': 'BlockerCount',
            'Value': float(count),
            'Unit': 'Count',
            'Dimensions': [
                {'Name': 'Phase', 'Value': str(phase)},
                {'Name': 'Severity', 'Value': severity}
            ],
            'Timestamp': datetime.now()
        })
    
    return metrics

def calculate_aggregate_metrics(all_phases):
    """Calculate aggregate metrics across all phases"""
    metrics = []
    
    # Overall migration progress
    total_progress = 0
    phase_count = 0
    
    for phase in all_phases:
        if phase.get('Active', True):
            completion = calculate_completion(phase)
            total_progress += completion
            phase_count += 1
    
    if phase_count > 0:
        overall_progress = total_progress / phase_count
        
        metrics.append({
            'MetricName': 'OverallMigrationProgress',
            'Value': float(overall_progress),
            'Unit': 'Percent',
            'Dimensions': [],
            'Timestamp': datetime.now()
        })
    
    # Verification metrics
    total_verifications = sum(int(p.get('TotalVerifications', 0)) for p in all_phases)
    passed_verifications = sum(int(p.get('PassedVerifications', 0)) for p in all_phases)
    
    if total_verifications > 0:
        pass_rate = (passed_verifications / total_verifications) * 100
        
        metrics.append({
            'MetricName': 'VerificationPassRate',
            'Value': float(pass_rate),
            'Unit': 'Percent',
            'Dimensions': [],
            'Timestamp': datetime.now()
        })
    
    # Automation metrics
    total_automated = sum(int(p.get('AutomatedTasks', 0)) for p in all_phases)
    successful_automated = sum(int(p.get('SuccessfulAutomatedTasks', 0)) for p in all_phases)
    
    if total_automated > 0:
        automation_rate = (successful_automated / total_automated) * 100
        
        metrics.append({
            'MetricName': 'AutomationSuccessRate',
            'Value': float(automation_rate),
            'Unit': 'Percent',
            'Dimensions': [],
            'Timestamp': datetime.now()
        })
    
    # Error frequency (last hour)
    error_count = calculate_recent_errors(all_phases)
    
    metrics.append({
        'MetricName': 'ErrorFrequency',
        'Value': float(error_count),
        'Unit': 'Count/Hour',
        'Dimensions': [],
        'Timestamp': datetime.now()
    })
    
    # Migration velocity
    velocity = calculate_migration_velocity(all_phases)
    
    metrics.append({
        'MetricName': 'MigrationVelocity',
        'Value': float(velocity),
        'Unit': 'Count/Day',
        'Dimensions': [],
        'Timestamp': datetime.now()
    })
    
    return metrics

def calculate_completion(phase_data):
    """Calculate completion percentage for a phase"""
    total = int(phase_data.get('TotalTasks', 0))
    completed = int(phase_data.get('CompletedTasks', 0))
    
    if total > 0:
        return (completed / total) * 100
    return 0

def calculate_recent_errors(all_phases):
    """Calculate error count in the last hour"""
    one_hour_ago = datetime.now() - timedelta(hours=1)
    error_count = 0
    
    for phase in all_phases:
        errors = phase.get('RecentErrors', [])
        for error in errors:
            error_time = error.get('Timestamp')
            if error_time:
                error_datetime = datetime.fromisoformat(error_time)
                if error_datetime > one_hour_ago:
                    error_count += 1
    
    return error_count

def calculate_migration_velocity(all_phases):
    """Calculate items migrated per day"""
    total_migrated = 0
    total_days = 0
    
    for phase in all_phases:
        migrated = int(phase.get('ItemsMigrated', 0))
        start_date = phase.get('StartDate')
        
        if start_date and migrated > 0:
            start_datetime = datetime.fromisoformat(start_date)
            days = (datetime.now() - start_datetime).days
            
            if days > 0:
                total_migrated += migrated
                total_days += days
    
    if total_days > 0:
        return total_migrated / total_days
    return 0

def publish_metrics(metrics):
    """Publish metrics to CloudWatch in batches of 20"""
    batch_size = 20
    
    for i in range(0, len(metrics), batch_size):
        batch = metrics[i:i + batch_size]
        
        try:
            cloudwatch.put_metric_data(
                Namespace=NAMESPACE,
                MetricData=batch
            )
            print(f"Published batch of {len(batch)} metrics")
        except Exception as e:
            print(f"Error publishing metrics batch: {str(e)}")
            raise

# Helper function for testing
def generate_test_data():
    """Generate test data for local testing"""
    return {
        'Phase': '2',
        'Environment': 'production',
        'TotalTasks': 100,
        'CompletedTasks': 65,
        'StartDate': (datetime.now() - timedelta(days=23)).isoformat(),
        'SubPhases': [
            {'Name': 'A', 'Status': 'completed', 'Component': 'EC2'},
            {'Name': 'B', 'Status': 'in-progress', 'Component': 'RDS'},
            {'Name': 'C', 'Status': 'pending', 'Component': 'S3'}
        ],
        'Blockers': [
            {'Severity': 'Critical', 'Description': 'RDS lag'},
            {'Severity': 'High', 'Description': 'IAM permissions'}
        ],
        'TotalVerifications': 100,
        'PassedVerifications': 96,
        'AutomatedTasks': 50,
        'SuccessfulAutomatedTasks': 44,
        'ItemsMigrated': 276,
        'RecentErrors': [
            {'Timestamp': datetime.now().isoformat(), 'Type': 'Connection'},
            {'Timestamp': (datetime.now() - timedelta(hours=2)).isoformat(), 'Type': 'Permission'}
        ]
    }

if __name__ == "__main__":
    # Local testing
    test_event = {}
    test_context = {}
    result = lambda_handler(test_event, test_context)
    print(json.dumps(result, indent=2))