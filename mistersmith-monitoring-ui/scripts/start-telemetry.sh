#!/bin/bash

# MisterSmith Monitoring UI - Telemetry Stack Startup Script
# This script starts the OpenTelemetry + Jaeger Docker stack

set -e

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

# Check if Docker is running
if ! docker info >/dev/null 2>&1; then
    print_error "Docker is not running. Please start Docker first."
    exit 1
fi

# Check if Docker Compose is available
if ! command -v docker-compose >/dev/null 2>&1; then
    print_error "Docker Compose is not installed. Please install Docker Compose first."
    exit 1
fi

# Change to the project directory
cd "$(dirname "$0")/.."

print_status "Starting MisterSmith Telemetry Stack..."

# Clean up any existing containers
print_status "Cleaning up existing containers..."
docker-compose down --remove-orphans

# Build and start the services
print_status "Building and starting services..."
docker-compose up -d --build

# Wait for services to be healthy
print_status "Waiting for services to be healthy..."

# Function to check if a service is healthy
check_service_health() {
    local service_name=$1
    local max_attempts=30
    local attempt=0
    
    while [ $attempt -lt $max_attempts ]; do
        if docker-compose ps $service_name | grep -q "(healthy)"; then
            print_success "$service_name is healthy"
            return 0
        fi
        
        if [ $attempt -eq 0 ]; then
            print_status "Waiting for $service_name to become healthy..."
        fi
        
        sleep 2
        attempt=$((attempt + 1))
    done
    
    print_warning "$service_name did not become healthy within timeout"
    return 1
}

# Check health of key services
check_service_health "jaeger" || print_warning "Jaeger may not be fully ready"
check_service_health "otel-collector" || print_warning "OpenTelemetry Collector may not be fully ready"
check_service_health "prometheus" || print_warning "Prometheus may not be fully ready"

# Show service status
print_status "Service status:"
docker-compose ps

# Display URLs
print_success "Telemetry stack is running!"
echo
print_status "Available services:"
echo "  • Jaeger UI:           http://localhost:16686"
echo "  • Prometheus:          http://localhost:9090"
echo "  • OTel Collector:      http://localhost:4318 (HTTP) / http://localhost:4317 (gRPC)"
echo "  • Health Check:        http://localhost:13133"
echo "  • Prometheus Metrics:  http://localhost:8889"
echo
print_status "To start the UI application:"
echo "  npm install && npm run dev"
echo
print_status "To stop the stack:"
echo "  ./scripts/stop-telemetry.sh"
echo
print_status "To view logs:"
echo "  docker-compose logs -f [service_name]"
echo
print_status "To view all logs:"
echo "  docker-compose logs -f"

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    print_status "Installing Node.js dependencies..."
    npm install
fi

# Validate configuration
print_status "Validating configuration..."
if [ -f "otel-collector-config.yaml" ]; then
    print_success "OpenTelemetry Collector configuration found"
else
    print_error "OpenTelemetry Collector configuration not found"
fi

if [ -f "prometheus.yml" ]; then
    print_success "Prometheus configuration found"
else
    print_error "Prometheus configuration not found"
fi

print_success "Setup complete! 🚀"