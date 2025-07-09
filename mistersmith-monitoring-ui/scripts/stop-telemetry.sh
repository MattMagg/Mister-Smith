#!/bin/bash

# MisterSmith Monitoring UI - Telemetry Stack Shutdown Script
# This script stops the OpenTelemetry + Jaeger Docker stack

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

# Change to the project directory
cd "$(dirname "$0")/.."

print_status "Stopping MisterSmith Telemetry Stack..."

# Stop and remove containers
print_status "Stopping services..."
docker-compose down

# Option to remove volumes (commented out by default)
# Uncomment the next line to remove persistent data
# docker-compose down -v

# Option to remove images (commented out by default)
# Uncomment the next line to also remove images
# docker-compose down --rmi all

# Clean up any orphaned containers
print_status "Cleaning up orphaned containers..."
docker-compose down --remove-orphans

# Show remaining containers (if any)
if docker-compose ps | grep -q "Up"; then
    print_warning "Some containers are still running:"
    docker-compose ps
else
    print_success "All services stopped successfully"
fi

# Clean up dangling images (optional)
if [ "$1" = "--clean" ]; then
    print_status "Cleaning up dangling Docker images..."
    docker image prune -f
    print_success "Cleanup complete"
fi

# Show disk usage
print_status "Current Docker disk usage:"
docker system df

print_success "Telemetry stack shutdown complete! 🛑"
echo
print_status "To restart the stack:"
echo "  ./scripts/start-telemetry.sh"
echo
print_status "To clean up all Docker resources:"
echo "  ./scripts/stop-telemetry.sh --clean"
echo
print_status "To remove all volumes (persistent data):"
echo "  docker-compose down -v"