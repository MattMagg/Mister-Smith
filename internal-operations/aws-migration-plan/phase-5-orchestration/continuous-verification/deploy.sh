#!/bin/bash

# Continuous Verification System Deployment Script
# AWS Migration Project - Phase 5 Orchestration

set -euo pipefail

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_NAME="${PROJECT_NAME:-aws-migration}"
ENVIRONMENT="${ENVIRONMENT:-production}"
AWS_REGION="${AWS_REGION:-us-east-1}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check AWS CLI
    if ! command -v aws &> /dev/null; then
        log_error "AWS CLI is not installed. Please install it first."
        exit 1
    fi
    
    # Check AWS credentials
    if ! aws sts get-caller-identity &> /dev/null; then
        log_error "AWS credentials not configured. Please run 'aws configure'."
        exit 1
    fi
    
    # Check required environment variables
    if [[ -z "${API_BASE_URL:-}" ]]; then
        log_error "API_BASE_URL environment variable is required"
        exit 1
    fi
    
    if [[ -z "${UI_BASE_URL:-}" ]]; then
        log_error "UI_BASE_URL environment variable is required"
        exit 1
    fi
    
    if [[ -z "${NOTIFICATION_EMAIL:-}" ]]; then
        log_error "NOTIFICATION_EMAIL environment variable is required"
        exit 1
    fi
    
    if [[ -z "${VPC_ID:-}" ]]; then
        log_error "VPC_ID environment variable is required"
        exit 1
    fi
    
    if [[ -z "${SUBNET_IDS:-}" ]]; then
        log_error "SUBNET_IDS environment variable is required (comma-separated)"
        exit 1
    fi
    
    if [[ -z "${DATABASE_HOST:-}" ]]; then
        log_error "DATABASE_HOST environment variable is required"
        exit 1
    fi
    
    log_success "Prerequisites check passed"
}

# Function to validate CloudFormation template
validate_template() {
    log_info "Validating CloudFormation template..."
    
    if aws cloudformation validate-template \
        --template-body file://"${SCRIPT_DIR}/monitoring-infrastructure.yaml" \
        --region "${AWS_REGION}" > /dev/null; then
        log_success "CloudFormation template is valid"
    else
        log_error "CloudFormation template validation failed"
        exit 1
    fi
}

# Function to deploy CloudFormation stack
deploy_stack() {
    local stack_name="${PROJECT_NAME}-${ENVIRONMENT}-verification"
    
    log_info "Deploying CloudFormation stack: ${stack_name}"
    
    # Convert comma-separated subnet IDs to proper parameter format
    local subnet_param
    subnet_param=$(echo "${SUBNET_IDS}" | sed 's/,/\\,/g')
    
    # Deploy the stack
    aws cloudformation deploy \
        --template-file "${SCRIPT_DIR}/monitoring-infrastructure.yaml" \
        --stack-name "${stack_name}" \
        --parameter-overrides \
            ProjectName="${PROJECT_NAME}" \
            Environment="${ENVIRONMENT}" \
            ApiBaseUrl="${API_BASE_URL}" \
            UIBaseUrl="${UI_BASE_URL}" \
            NotificationEmail="${NOTIFICATION_EMAIL}" \
            VpcId="${VPC_ID}" \
            SubnetIds="${subnet_param}" \
            DatabaseHost="${DATABASE_HOST}" \
            DatabaseName="${DATABASE_NAME:-migration_db}" \
        --capabilities CAPABILITY_NAMED_IAM \
        --region "${AWS_REGION}" \
        --tags \
            Project="${PROJECT_NAME}" \
            Environment="${ENVIRONMENT}" \
            Component="ContinuousVerification"
    
    if [[ $? -eq 0 ]]; then
        log_success "CloudFormation stack deployed successfully"
    else
        log_error "CloudFormation stack deployment failed"
        exit 1
    fi
}

# Function to get stack outputs
get_stack_outputs() {
    local stack_name="${PROJECT_NAME}-${ENVIRONMENT}-verification"
    
    log_info "Retrieving stack outputs..."
    
    aws cloudformation describe-stacks \
        --stack-name "${stack_name}" \
        --region "${AWS_REGION}" \
        --query 'Stacks[0].Outputs' \
        --output table
}

# Function to create additional Lambda layers if needed
create_lambda_layers() {
    log_info "Creating Lambda layers for database connectivity..."
    
    # Create psycopg2 layer for PostgreSQL connectivity
    local layer_dir="${SCRIPT_DIR}/lambda-layers"
    mkdir -p "${layer_dir}/python"
    
    # Install psycopg2-binary in layer directory
    if command -v pip3 &> /dev/null; then
        pip3 install psycopg2-binary -t "${layer_dir}/python/" > /dev/null 2>&1
        
        # Create layer zip
        cd "${layer_dir}"
        zip -r psycopg2-layer.zip python/ > /dev/null
        
        # Upload layer to AWS
        local layer_arn
        layer_arn=$(aws lambda publish-layer-version \
            --layer-name "${PROJECT_NAME}-${ENVIRONMENT}-psycopg2" \
            --description "PostgreSQL adapter for Python" \
            --zip-file fileb://psycopg2-layer.zip \
            --compatible-runtimes python3.9 \
            --region "${AWS_REGION}" \
            --query 'LayerVersionArn' \
            --output text)
        
        log_success "Lambda layer created: ${layer_arn}"
        
        # Update Lambda functions to use the layer
        update_lambda_layers "${layer_arn}"
        
        # Cleanup
        cd "${SCRIPT_DIR}"
        rm -rf "${layer_dir}"
    else
        log_warning "pip3 not found, skipping Lambda layer creation"
    fi
}

# Function to update Lambda functions with layers
update_lambda_layers() {
    local layer_arn="$1"
    local functions=(
        "${PROJECT_NAME}-${ENVIRONMENT}-infrastructure-health"
        "${PROJECT_NAME}-${ENVIRONMENT}-database-health"
    )
    
    log_info "Updating Lambda functions with layers..."
    
    for function_name in "${functions[@]}"; do
        aws lambda update-function-configuration \
            --function-name "${function_name}" \
            --layers "${layer_arn}" \
            --region "${AWS_REGION}" > /dev/null
        
        log_success "Updated function: ${function_name}"
    done
}

# Function to verify deployment
verify_deployment() {
    log_info "Verifying deployment..."
    
    local stack_name="${PROJECT_NAME}-${ENVIRONMENT}-verification"
    
    # Check stack status
    local stack_status
    stack_status=$(aws cloudformation describe-stacks \
        --stack-name "${stack_name}" \
        --region "${AWS_REGION}" \
        --query 'Stacks[0].StackStatus' \
        --output text)
    
    if [[ "${stack_status}" == "CREATE_COMPLETE" || "${stack_status}" == "UPDATE_COMPLETE" ]]; then
        log_success "Stack deployment verified: ${stack_status}"
    else
        log_error "Stack deployment verification failed: ${stack_status}"
        return 1
    fi
    
    # Check canary status
    local canaries=(
        "${PROJECT_NAME}-${ENVIRONMENT}-infrastructure-health"
        "${PROJECT_NAME}-${ENVIRONMENT}-api-health"
        "${PROJECT_NAME}-${ENVIRONMENT}-ui-functional"
    )
    
    for canary in "${canaries[@]}"; do
        local canary_status
        canary_status=$(aws synthetics get-canary \
            --name "${canary}" \
            --region "${AWS_REGION}" \
            --query 'Canary.Status.State' \
            --output text 2>/dev/null || echo "NOT_FOUND")
        
        if [[ "${canary_status}" == "RUNNING" ]]; then
            log_success "Canary verified: ${canary}"
        else
            log_warning "Canary status: ${canary} - ${canary_status}"
        fi
    done
    
    # Check Lambda functions
    local lambda_functions=(
        "${PROJECT_NAME}-${ENVIRONMENT}-infrastructure-health"
        "${PROJECT_NAME}-${ENVIRONMENT}-database-health"
    )
    
    for function_name in "${lambda_functions[@]}"; do
        local function_status
        function_status=$(aws lambda get-function \
            --function-name "${function_name}" \
            --region "${AWS_REGION}" \
            --query 'Configuration.State' \
            --output text 2>/dev/null || echo "NOT_FOUND")
        
        if [[ "${function_status}" == "Active" ]]; then
            log_success "Lambda function verified: ${function_name}"
        else
            log_warning "Lambda function status: ${function_name} - ${function_status}"
        fi
    done
}

# Function to test canaries
test_canaries() {
    log_info "Testing canaries..."
    
    local canaries=(
        "${PROJECT_NAME}-${ENVIRONMENT}-infrastructure-health"
        "${PROJECT_NAME}-${ENVIRONMENT}-api-health"
        "${PROJECT_NAME}-${ENVIRONMENT}-ui-functional"
    )
    
    for canary in "${canaries[@]}"; do
        log_info "Starting canary: ${canary}"
        
        aws synthetics start-canary \
            --name "${canary}" \
            --region "${AWS_REGION}" > /dev/null 2>&1 || true
        
        # Wait a moment for canary to start
        sleep 5
        
        local canary_status
        canary_status=$(aws synthetics get-canary \
            --name "${canary}" \
            --region "${AWS_REGION}" \
            --query 'Canary.Status.State' \
            --output text)
        
        log_info "Canary ${canary} status: ${canary_status}"
    done
}

# Function to show dashboard information
show_dashboard_info() {
    log_info "Deployment completed successfully!"
    echo
    echo "=== Continuous Verification System Deployed ==="
    echo
    
    # Get dashboard URL
    local stack_name="${PROJECT_NAME}-${ENVIRONMENT}-verification"
    local dashboard_url
    dashboard_url=$(aws cloudformation describe-stacks \
        --stack-name "${stack_name}" \
        --region "${AWS_REGION}" \
        --query 'Stacks[0].Outputs[?OutputKey==`DashboardURL`].OutputValue' \
        --output text)
    
    echo "📊 CloudWatch Dashboard: ${dashboard_url}"
    echo
    
    # Show canary URLs
    echo "🕷️ Synthetics Canaries:"
    local canaries=(
        "${PROJECT_NAME}-${ENVIRONMENT}-infrastructure-health"
        "${PROJECT_NAME}-${ENVIRONMENT}-api-health"
        "${PROJECT_NAME}-${ENVIRONMENT}-ui-functional"
    )
    
    for canary in "${canaries[@]}"; do
        echo "  - ${canary}"
        echo "    https://${AWS_REGION}.console.aws.amazon.com/synthetics/canaries/detail/${canary}"
    done
    echo
    
    # Show Lambda function URLs
    echo "⚡ Lambda Health Check Functions:"
    local lambda_functions=(
        "${PROJECT_NAME}-${ENVIRONMENT}-infrastructure-health"
        "${PROJECT_NAME}-${ENVIRONMENT}-database-health"
    )
    
    for function_name in "${lambda_functions[@]}"; do
        echo "  - ${function_name}"
        echo "    https://${AWS_REGION}.console.aws.amazon.com/lambda/home?region=${AWS_REGION}#/functions/${function_name}"
    done
    echo
    
    echo "📧 Alert notifications will be sent to: ${NOTIFICATION_EMAIL}"
    echo
    echo "🔍 Monitor the system using the CloudWatch Dashboard above."
    echo "📋 Check the tracker.md file for detailed operational procedures."
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo
    echo "Deploy the Continuous Verification System for AWS Migration"
    echo
    echo "Required Environment Variables:"
    echo "  API_BASE_URL      - Base URL for API health checks"
    echo "  UI_BASE_URL       - Base URL for UI functional testing"
    echo "  NOTIFICATION_EMAIL - Email address for alerts"
    echo "  VPC_ID           - VPC ID for canary execution"
    echo "  SUBNET_IDS       - Comma-separated subnet IDs"
    echo "  DATABASE_HOST    - Aurora PostgreSQL cluster endpoint"
    echo
    echo "Optional Environment Variables:"
    echo "  PROJECT_NAME     - Project name (default: aws-migration)"
    echo "  ENVIRONMENT      - Environment name (default: production)"
    echo "  AWS_REGION       - AWS region (default: us-east-1)"
    echo "  DATABASE_NAME    - Database name (default: migration_db)"
    echo
    echo "Options:"
    echo "  -h, --help       Show this help message"
    echo "  --verify-only    Only verify existing deployment"
    echo "  --test-only      Only test canaries"
    echo
    echo "Examples:"
    echo "  # Deploy with environment variables"
    echo "  export API_BASE_URL=https://api.example.com"
    echo "  export UI_BASE_URL=https://app.example.com"
    echo "  export NOTIFICATION_EMAIL=alerts@example.com"
    echo "  export VPC_ID=vpc-12345678"
    echo "  export SUBNET_IDS=subnet-12345678,subnet-87654321"
    echo "  export DATABASE_HOST=aurora-cluster.cluster-xyz.us-east-1.rds.amazonaws.com"
    echo "  $0"
    echo
    echo "  # Verify existing deployment"
    echo "  $0 --verify-only"
}

# Main execution function
main() {
    local verify_only=false
    local test_only=false
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            --verify-only)
                verify_only=true
                shift
                ;;
            --test-only)
                test_only=true
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
    
    # Show banner
    echo "========================================"
    echo "  AWS Migration Continuous Verification"
    echo "  Deployment Script"
    echo "========================================"
    echo
    
    if [[ "${verify_only}" == true ]]; then
        log_info "Running verification only..."
        verify_deployment
        return $?
    fi
    
    if [[ "${test_only}" == true ]]; then
        log_info "Running tests only..."
        test_canaries
        return $?
    fi
    
    # Full deployment
    check_prerequisites
    validate_template
    deploy_stack
    create_lambda_layers
    verify_deployment
    test_canaries
    show_dashboard_info
    
    log_success "Continuous Verification System deployment completed!"
}

# Error handling
trap 'log_error "Script failed on line $LINENO"' ERR

# Run main function with all arguments
main "$@"