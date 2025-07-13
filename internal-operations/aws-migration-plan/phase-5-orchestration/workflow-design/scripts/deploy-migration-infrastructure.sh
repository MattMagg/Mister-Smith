#!/bin/bash

# AWS Migration Infrastructure Deployment Script
# This script deploys the complete Step Functions orchestration infrastructure

set -e

# Configuration
PROJECT_NAME="aws-migration"
ENVIRONMENT="prod"
AWS_REGION="us-east-1"
STACK_NAME="${PROJECT_NAME}-${ENVIRONMENT}-infrastructure"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check prerequisites
check_prerequisites() {
    print_status "Checking prerequisites..."
    
    # Check if AWS CLI is installed
    if ! command -v aws &> /dev/null; then
        print_error "AWS CLI is not installed. Please install it first."
        exit 1
    fi
    
    # Check if AWS credentials are configured
    if ! aws sts get-caller-identity &> /dev/null; then
        print_error "AWS credentials are not configured. Please run 'aws configure' first."
        exit 1
    fi
    
    # Check if required parameters are provided
    if [ -z "$NOTIFICATION_EMAIL" ]; then
        print_error "NOTIFICATION_EMAIL environment variable is required."
        echo "Usage: NOTIFICATION_EMAIL=admin@example.com APPROVAL_EMAIL=approver@example.com $0"
        exit 1
    fi
    
    if [ -z "$APPROVAL_EMAIL" ]; then
        print_error "APPROVAL_EMAIL environment variable is required."
        echo "Usage: NOTIFICATION_EMAIL=admin@example.com APPROVAL_EMAIL=approver@example.com $0"
        exit 1
    fi
    
    print_success "Prerequisites check passed."
}

# Function to validate CloudFormation template
validate_template() {
    print_status "Validating CloudFormation template..."
    
    local template_file="cloudformation/migration-orchestration-infrastructure.yaml"
    
    if [ ! -f "$template_file" ]; then
        print_error "CloudFormation template not found: $template_file"
        exit 1
    fi
    
    if aws cloudformation validate-template --template-body "file://$template_file" &> /dev/null; then
        print_success "CloudFormation template is valid."
    else
        print_error "CloudFormation template validation failed."
        exit 1
    fi
}

# Function to package Lambda functions
package_lambda_functions() {
    print_status "Packaging Lambda functions..."
    
    local lambda_dir="lambda-functions"
    local temp_dir="/tmp/lambda-packages"
    
    # Create temporary directory
    mkdir -p "$temp_dir"
    
    # Package each Lambda function
    for function_file in "$lambda_dir"/*.py; do
        if [ -f "$function_file" ]; then
            local function_name=$(basename "$function_file" .py)
            local zip_file="$temp_dir/${function_name}.zip"
            
            print_status "Packaging $function_name..."
            
            # Create zip file with dependencies
            cd "$lambda_dir"
            zip -q "$zip_file" "$function_name.py"
            
            # Add boto3 and other dependencies if requirements.txt exists
            if [ -f "requirements.txt" ]; then
                pip install -r requirements.txt -t ./packages
                cd packages
                zip -qr "$zip_file" .
                cd ..
                rm -rf packages
            fi
            
            cd - > /dev/null
            
            print_success "Packaged $function_name to $zip_file"
        fi
    done
}

# Function to create S3 bucket for deployment artifacts
create_deployment_bucket() {
    print_status "Creating deployment artifacts bucket..."
    
    local bucket_name="${PROJECT_NAME}-${ENVIRONMENT}-deployment-artifacts-$(aws sts get-caller-identity --query Account --output text)"
    
    # Check if bucket exists
    if aws s3 ls "s3://$bucket_name" &> /dev/null; then
        print_warning "Deployment bucket already exists: $bucket_name"
    else
        # Create bucket
        if [ "$AWS_REGION" == "us-east-1" ]; then
            aws s3 mb "s3://$bucket_name"
        else
            aws s3 mb "s3://$bucket_name" --region "$AWS_REGION"
        fi
        
        # Enable versioning
        aws s3api put-bucket-versioning \
            --bucket "$bucket_name" \
            --versioning-configuration Status=Enabled
        
        print_success "Created deployment bucket: $bucket_name"
    fi
    
    echo "$bucket_name"
}

# Function to upload Lambda packages to S3
upload_lambda_packages() {
    local bucket_name="$1"
    local temp_dir="/tmp/lambda-packages"
    
    print_status "Uploading Lambda packages to S3..."
    
    for zip_file in "$temp_dir"/*.zip; do
        if [ -f "$zip_file" ]; then
            local file_name=$(basename "$zip_file")
            aws s3 cp "$zip_file" "s3://$bucket_name/lambda-functions/$file_name"
            print_success "Uploaded $file_name to S3"
        fi
    done
}

# Function to deploy CloudFormation stack
deploy_stack() {
    local bucket_name="$1"
    
    print_status "Deploying CloudFormation stack: $STACK_NAME"
    
    # Check if stack exists
    if aws cloudformation describe-stacks --stack-name "$STACK_NAME" &> /dev/null; then
        print_status "Stack exists, updating..."
        local operation="update-stack"
    else
        print_status "Stack does not exist, creating..."
        local operation="create-stack"
    fi
    
    # Deploy stack
    local stack_id=$(aws cloudformation "$operation" \
        --stack-name "$STACK_NAME" \
        --template-body "file://cloudformation/migration-orchestration-infrastructure.yaml" \
        --parameters \
            ParameterKey=ProjectName,ParameterValue="$PROJECT_NAME" \
            ParameterKey=Environment,ParameterValue="$ENVIRONMENT" \
            ParameterKey=NotificationEmail,ParameterValue="$NOTIFICATION_EMAIL" \
            ParameterKey=ApprovalEmail,ParameterValue="$APPROVAL_EMAIL" \
        --capabilities CAPABILITY_NAMED_IAM \
        --query 'StackId' \
        --output text)
    
    print_success "Stack operation initiated: $stack_id"
    
    # Wait for stack operation to complete
    print_status "Waiting for stack operation to complete..."
    
    if [ "$operation" == "create-stack" ]; then
        aws cloudformation wait stack-create-complete --stack-name "$STACK_NAME"
    else
        aws cloudformation wait stack-update-complete --stack-name "$STACK_NAME"
    fi
    
    print_success "Stack operation completed successfully!"
}

# Function to update Lambda function code
update_lambda_functions() {
    local bucket_name="$1"
    local temp_dir="/tmp/lambda-packages"
    
    print_status "Updating Lambda function code..."
    
    # Get function names from stack outputs
    local functions=(
        "${PROJECT_NAME}-${ENVIRONMENT}-validation-function"
        "${PROJECT_NAME}-${ENVIRONMENT}-verification-function"
        "${PROJECT_NAME}-${ENVIRONMENT}-rollback-function"
        "${PROJECT_NAME}-${ENVIRONMENT}-provisioning-function"
        "${PROJECT_NAME}-${ENVIRONMENT}-migration-function"
        "${PROJECT_NAME}-${ENVIRONMENT}-approval-function"
    )
    
    for function_name in "${functions[@]}"; do
        local zip_file="$temp_dir/$(echo $function_name | sed "s/${PROJECT_NAME}-${ENVIRONMENT}-//").zip"
        
        if [ -f "$zip_file" ]; then
            print_status "Updating function code: $function_name"
            
            aws lambda update-function-code \
                --function-name "$function_name" \
                --s3-bucket "$bucket_name" \
                --s3-key "lambda-functions/$(basename "$zip_file")" \
                > /dev/null
            
            print_success "Updated function: $function_name"
        fi
    done
}

# Function to create SNS topic subscriptions
setup_notifications() {
    print_status "Setting up SNS topic subscriptions..."
    
    # Get topic ARNs from stack outputs
    local status_topic=$(aws cloudformation describe-stacks \
        --stack-name "$STACK_NAME" \
        --query "Stacks[0].Outputs[?OutputKey=='StatusNotificationTopicArn'].OutputValue" \
        --output text)
    
    local error_topic=$(aws cloudformation describe-stacks \
        --stack-name "$STACK_NAME" \
        --query "Stacks[0].Outputs[?OutputKey=='ErrorNotificationTopicArn'].OutputValue" \
        --output text)
    
    print_success "Notification topics configured:"
    print_success "  Status: $status_topic"
    print_success "  Error: $error_topic"
}

# Function to display deployment summary
display_summary() {
    print_status "Deployment Summary:"
    
    # Get stack outputs
    local outputs=$(aws cloudformation describe-stacks \
        --stack-name "$STACK_NAME" \
        --query 'Stacks[0].Outputs' \
        --output table)
    
    echo "$outputs"
    
    print_success "Migration orchestration infrastructure deployed successfully!"
    
    # Get API endpoint
    local api_endpoint=$(aws cloudformation describe-stacks \
        --stack-name "$STACK_NAME" \
        --query "Stacks[0].Outputs[?OutputKey=='MigrationAPIEndpoint'].OutputValue" \
        --output text)
    
    print_status "You can trigger migrations using the API endpoint:"
    echo "  $api_endpoint/migration/start"
    
    # Get dashboard URL
    local dashboard_url=$(aws cloudformation describe-stacks \
        --stack-name "$STACK_NAME" \
        --query "Stacks[0].Outputs[?OutputKey=='DashboardURL'].OutputValue" \
        --output text)
    
    print_status "Monitor migrations using the CloudWatch dashboard:"
    echo "  $dashboard_url"
}

# Function to run smoke tests
run_smoke_tests() {
    print_status "Running smoke tests..."
    
    # Test Lambda functions
    local functions=(
        "${PROJECT_NAME}-${ENVIRONMENT}-validation-function"
        "${PROJECT_NAME}-${ENVIRONMENT}-verification-function"
    )
    
    for function_name in "${functions[@]}"; do
        print_status "Testing function: $function_name"
        
        local test_payload='{"action": "test", "test": true}'
        local result=$(aws lambda invoke \
            --function-name "$function_name" \
            --payload "$test_payload" \
            --output text \
            --query 'StatusCode' \
            /tmp/lambda-response.json)
        
        if [ "$result" == "200" ]; then
            print_success "Function test passed: $function_name"
        else
            print_warning "Function test failed: $function_name"
        fi
    done
    
    print_success "Smoke tests completed."
}

# Function to cleanup temporary files
cleanup() {
    print_status "Cleaning up temporary files..."
    rm -rf /tmp/lambda-packages
    rm -f /tmp/lambda-response.json
    print_success "Cleanup completed."
}

# Main deployment function
main() {
    print_status "Starting AWS Migration Infrastructure Deployment"
    print_status "Project: $PROJECT_NAME"
    print_status "Environment: $ENVIRONMENT"
    print_status "Region: $AWS_REGION"
    print_status "Stack: $STACK_NAME"
    echo
    
    # Run deployment steps
    check_prerequisites
    validate_template
    package_lambda_functions
    
    local bucket_name=$(create_deployment_bucket)
    upload_lambda_packages "$bucket_name"
    deploy_stack "$bucket_name"
    update_lambda_functions "$bucket_name"
    setup_notifications
    
    # Run post-deployment tasks
    run_smoke_tests
    display_summary
    cleanup
    
    print_success "Deployment completed successfully!"
}

# Trap for cleanup on exit
trap cleanup EXIT

# Run main function
main "$@"