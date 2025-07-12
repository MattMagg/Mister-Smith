#!/bin/bash
# emergency-fallback.sh
# Emergency fallback to local Docker containers when AWS is completely unavailable

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

# Emergency log
EMERGENCY_LOG="/var/log/mistersmith/emergency-$(date +%Y%m%d-%H%M%S).log"
mkdir -p /var/log/mistersmith

# Function to log
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a $EMERGENCY_LOG
}

# Banner
echo -e "${RED}╔════════════════════════════════════════╗${NC}" | tee $EMERGENCY_LOG
echo -e "${RED}║      EMERGENCY FALLBACK INITIATED      ║${NC}" | tee -a $EMERGENCY_LOG
echo -e "${RED}╚════════════════════════════════════════╝${NC}" | tee -a $EMERGENCY_LOG
echo "" | tee -a $EMERGENCY_LOG

log "⚠️  EMERGENCY FALLBACK INITIATED - AWS services unavailable"

# Step 1: Block AWS connectivity (prevent partial connections)
echo -e "${YELLOW}Step 1: Blocking AWS connectivity...${NC}" | tee -a $EMERGENCY_LOG

# Check if we have sudo access for iptables
if sudo -n true 2>/dev/null; then
    # AWS IP ranges (common ranges - extend as needed)
    AWS_RANGES=(
        "52.0.0.0/8"
        "54.0.0.0/8"
        "18.0.0.0/8"
        "35.0.0.0/8"
    )
    
    for range in "${AWS_RANGES[@]}"; do
        sudo iptables -A OUTPUT -d $range -j DROP 2>/dev/null || true
        log "Blocked outbound traffic to $range"
    done
else
    log "⚠️  Cannot modify iptables - skipping AWS blocking"
fi

# Step 2: Create emergency Docker Compose configuration
echo -e "${YELLOW}Step 2: Creating emergency Docker configuration...${NC}" | tee -a $EMERGENCY_LOG

cat > $PROJECT_ROOT/docker-compose.emergency.yml << 'EOF'
version: '3.8'

services:
  postgres:
    image: postgres:14-alpine
    container_name: mistersmith-postgres-emergency
    environment:
      POSTGRES_DB: mistersmith
      POSTGRES_USER: mistersmith
      POSTGRES_PASSWORD: emergency-password
    volumes:
      - ./emergency-data/postgres:/var/lib/postgresql/data
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U mistersmith"]
      interval: 10s
      timeout: 5s
      retries: 5

  nats:
    image: nats:2.10-alpine
    container_name: mistersmith-nats-emergency
    command: ["-js", "-m", "8222"]
    ports:
      - "4222:4222"
      - "8222:8222"
    volumes:
      - ./emergency-data/nats:/data
    healthcheck:
      test: ["CMD", "nc", "-z", "localhost", "4222"]
      interval: 10s
      timeout: 5s
      retries: 5

  orchestrator:
    image: mistersmith/orchestrator:previous
    container_name: mistersmith-orchestrator-emergency
    depends_on:
      postgres:
        condition: service_healthy
      nats:
        condition: service_healthy
    environment:
      DB_HOST: postgres
      DB_PORT: 5432
      DB_NAME: mistersmith
      DB_USER: mistersmith
      DB_PASSWORD: emergency-password
      NATS_URL: nats://nats:4222
      LOG_LEVEL: debug
      EMERGENCY_MODE: "true"
    ports:
      - "50051:50051"
    volumes:
      - ./emergency-config/orchestrator:/app/config
    healthcheck:
      test: ["CMD", "grpc_health_probe", "-addr=:50051"]
      interval: 10s
      timeout: 5s
      retries: 5

  webui:
    image: mistersmith/webui:previous
    container_name: mistersmith-webui-emergency
    depends_on:
      orchestrator:
        condition: service_healthy
    environment:
      REACT_APP_API_URL: http://localhost:50051
      REACT_APP_EMERGENCY_MODE: "true"
    ports:
      - "3000:3000"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 10s
      timeout: 5s
      retries: 5

networks:
  default:
    name: mistersmith-emergency
EOF

log "Emergency Docker Compose configuration created"

# Step 3: Create emergency configuration files
echo -e "${YELLOW}Step 3: Setting up emergency configurations...${NC}" | tee -a $EMERGENCY_LOG

# Create config directories
mkdir -p $PROJECT_ROOT/emergency-config/{orchestrator,webui}
mkdir -p $PROJECT_ROOT/emergency-data/{postgres,nats}

# Orchestrator emergency config
cat > $PROJECT_ROOT/emergency-config/orchestrator/config.json << EOF
{
  "mode": "emergency",
  "database": {
    "host": "postgres",
    "port": 5432,
    "name": "mistersmith",
    "user": "mistersmith",
    "password": "emergency-password"
  },
  "nats": {
    "url": "nats://nats:4222",
    "streamName": "MisterSmith-Emergency"
  },
  "server": {
    "port": 50051,
    "maxConnections": 100
  },
  "logging": {
    "level": "debug",
    "output": "stdout"
  },
  "emergency": {
    "enabled": true,
    "autoRecover": false,
    "alertWebhook": ""
  }
}
EOF

# Step 4: Stop any existing containers
echo -e "${YELLOW}Step 4: Stopping existing containers...${NC}" | tee -a $EMERGENCY_LOG

docker-compose -f $PROJECT_ROOT/docker-compose.yml down 2>/dev/null || true
docker stop $(docker ps -q --filter "name=mistersmith") 2>/dev/null || true

# Step 5: Load emergency container images from backup
echo -e "${YELLOW}Step 5: Loading emergency container images...${NC}" | tee -a $EMERGENCY_LOG

BACKUP_DIR="/var/backups/mistersmith/container-images"
if [ -d "$BACKUP_DIR" ]; then
    for image in orchestrator webui nats; do
        if [ -f "$BACKUP_DIR/${image}-backup.tar.gz" ]; then
            log "Loading $image from backup..."
            docker load < "$BACKUP_DIR/${image}-backup.tar.gz" || true
        fi
    done
else
    log "⚠️  No local image backups found - will attempt to use cached images"
fi

# Step 6: Start emergency containers
echo -e "${YELLOW}Step 6: Starting emergency containers...${NC}" | tee -a $EMERGENCY_LOG

cd $PROJECT_ROOT
docker-compose -f docker-compose.emergency.yml up -d

# Step 7: Wait for services to be healthy
echo -e "${YELLOW}Step 7: Waiting for services to become healthy...${NC}" | tee -a $EMERGENCY_LOG

MAX_WAIT=120
WAIT_INTERVAL=5
elapsed=0

while [ $elapsed -lt $MAX_WAIT ]; do
    healthy_count=$(docker-compose -f docker-compose.emergency.yml ps | grep -c "healthy" || true)
    
    if [ $healthy_count -eq 4 ]; then
        log "✅ All services are healthy"
        break
    fi
    
    echo -n "." | tee -a $EMERGENCY_LOG
    sleep $WAIT_INTERVAL
    elapsed=$((elapsed + WAIT_INTERVAL))
done

echo "" | tee -a $EMERGENCY_LOG

# Step 8: Update local DNS
echo -e "${YELLOW}Step 8: Updating local DNS entries...${NC}" | tee -a $EMERGENCY_LOG

# Backup current hosts file
sudo cp /etc/hosts /etc/hosts.backup-$(date +%Y%m%d-%H%M%S) 2>/dev/null || true

# Add emergency entries
{
    echo ""
    echo "# MisterSmith Emergency Fallback Entries"
    echo "127.0.0.1 mistersmith.local"
    echo "127.0.0.1 api.mistersmith.local"
    echo "127.0.0.1 orchestrator.mistersmith.local"
} | sudo tee -a /etc/hosts > /dev/null

log "Local DNS entries added"

# Step 9: Initialize emergency database
echo -e "${YELLOW}Step 9: Initializing emergency database...${NC}" | tee -a $EMERGENCY_LOG

# Wait for PostgreSQL to be fully ready
sleep 5

# Check if we need to restore from backup
if [ -f "$PROJECT_ROOT/emergency-data/postgres/mistersmith-emergency-backup.sql" ]; then
    log "Restoring database from emergency backup..."
    docker exec -i mistersmith-postgres-emergency psql -U mistersmith mistersmith < \
        $PROJECT_ROOT/emergency-data/postgres/mistersmith-emergency-backup.sql 2>/dev/null || true
else
    log "⚠️  No emergency database backup found - starting with empty database"
fi

# Step 10: Verify emergency services
echo -e "${YELLOW}Step 10: Verifying emergency services...${NC}" | tee -a $EMERGENCY_LOG

SERVICES_OK=true

# Check PostgreSQL
if docker exec mistersmith-postgres-emergency pg_isready -U mistersmith > /dev/null 2>&1; then
    log "✅ PostgreSQL is responding"
else
    log "❌ PostgreSQL is not responding"
    SERVICES_OK=false
fi

# Check NATS
if nc -z localhost 4222 2>/dev/null; then
    log "✅ NATS is responding"
else
    log "❌ NATS is not responding"
    SERVICES_OK=false
fi

# Check Orchestrator
if grpc_health_probe -addr=localhost:50051 2>/dev/null; then
    log "✅ Orchestrator is responding"
else
    log "❌ Orchestrator is not responding"
    SERVICES_OK=false
fi

# Check Web UI
if curl -sf http://localhost:3000/health > /dev/null; then
    log "✅ Web UI is responding"
else
    log "❌ Web UI is not responding"
    SERVICES_OK=false
fi

# Final status
echo "" | tee -a $EMERGENCY_LOG
echo -e "${MAGENTA}═══════════════════════════════════════════${NC}" | tee -a $EMERGENCY_LOG
echo -e "${MAGENTA}        Emergency Fallback Status${NC}" | tee -a $EMERGENCY_LOG
echo -e "${MAGENTA}═══════════════════════════════════════════${NC}" | tee -a $EMERGENCY_LOG
echo "" | tee -a $EMERGENCY_LOG

if [ "$SERVICES_OK" = true ]; then
    echo -e "${GREEN}✅ EMERGENCY FALLBACK ACTIVE${NC}" | tee -a $EMERGENCY_LOG
    echo "" | tee -a $EMERGENCY_LOG
    echo "Services are running locally:" | tee -a $EMERGENCY_LOG
    echo "  • Web UI: http://localhost:3000" | tee -a $EMERGENCY_LOG
    echo "  • Orchestrator: localhost:50051" | tee -a $EMERGENCY_LOG
    echo "  • NATS: localhost:4222" | tee -a $EMERGENCY_LOG
    echo "  • PostgreSQL: localhost:5432" | tee -a $EMERGENCY_LOG
    echo "" | tee -a $EMERGENCY_LOG
    echo -e "${YELLOW}⚠️  Operating in EMERGENCY MODE${NC}" | tee -a $EMERGENCY_LOG
    echo "  - No AWS connectivity" | tee -a $EMERGENCY_LOG
    echo "  - Limited functionality" | tee -a $EMERGENCY_LOG
    echo "  - Local data only" | tee -a $EMERGENCY_LOG
    echo "" | tee -a $EMERGENCY_LOG
    echo "To restore normal operations:" | tee -a $EMERGENCY_LOG
    echo "  1. Resolve AWS connectivity issues" | tee -a $EMERGENCY_LOG
    echo "  2. Run: ./restore-normal-operations.sh" | tee -a $EMERGENCY_LOG
    exit 0
else
    echo -e "${RED}❌ EMERGENCY FALLBACK FAILED${NC}" | tee -a $EMERGENCY_LOG
    echo "" | tee -a $EMERGENCY_LOG
    echo "Some services failed to start" | tee -a $EMERGENCY_LOG
    echo "Check logs: $EMERGENCY_LOG" | tee -a $EMERGENCY_LOG
    echo "" | tee -a $EMERGENCY_LOG
    echo "Manual intervention required:" | tee -a $EMERGENCY_LOG
    echo "  1. Check docker-compose logs" | tee -a $EMERGENCY_LOG
    echo "  2. Verify container images exist" | tee -a $EMERGENCY_LOG
    echo "  3. Check port conflicts" | tee -a $EMERGENCY_LOG
    exit 1
fi