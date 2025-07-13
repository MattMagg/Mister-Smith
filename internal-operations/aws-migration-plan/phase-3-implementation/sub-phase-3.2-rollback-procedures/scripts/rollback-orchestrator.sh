#!/bin/bash
# rollback-orchestrator.sh
# Rollback procedure for MisterSmith Orchestrator Service

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
BACKUP_BUCKET="s3://mistersmith-backups"
ORCHESTRATOR_PORT=50051
HEALTH_CHECK_RETRIES=10
HEALTH_CHECK_INTERVAL=5

echo -e "${YELLOW}Starting Orchestrator rollback...${NC}"

# Function to check service health
check_health() {
    local retries=$1
    local interval=$2
    
    for i in $(seq 1 $retries); do
        if grpc_health_probe -addr=:${ORCHESTRATOR_PORT} 2>/dev/null; then
            return 0
        fi
        echo "Health check attempt $i/$retries failed, waiting ${interval}s..."
        sleep $interval
    done
    return 1
}

# Step 1: Stop current service
echo -e "${YELLOW}Stopping current orchestrator service...${NC}"
docker stop mistersmith-orchestrator 2>/dev/null || true
docker rm mistersmith-orchestrator 2>/dev/null || true

# Step 2: Backup current state before rollback
echo -e "${YELLOW}Backing up current state...${NC}"
if [ -f ~/.mistersmith/orchestrator/config.json ]; then
    cp ~/.mistersmith/orchestrator/config.json ~/.mistersmith/orchestrator/config.json.rollback-$(date +%Y%m%d-%H%M%S)
fi

# Step 3: Restore previous container image
echo -e "${YELLOW}Restoring previous orchestrator image...${NC}"
if [ -f orchestrator-backup.tar.gz ]; then
    docker load < orchestrator-backup.tar.gz
else
    echo -e "${RED}Local backup not found, pulling from registry...${NC}"
    docker pull mistersmith/orchestrator:previous
fi

# Step 4: Restore configuration from S3
echo -e "${YELLOW}Restoring configuration from backup...${NC}"
aws s3 cp ${BACKUP_BUCKET}/pre-migration/orchestrator-config-latest.json ~/.mistersmith/orchestrator/config.json

# Step 5: Verify configuration
if [ ! -f ~/.mistersmith/orchestrator/config.json ]; then
    echo -e "${RED}Configuration file not found after restore!${NC}"
    exit 1
fi

# Step 6: Start previous version with enhanced health checks
echo -e "${YELLOW}Starting previous orchestrator version...${NC}"
docker run -d \
  --name mistersmith-orchestrator \
  --restart unless-stopped \
  -v ~/.mistersmith/orchestrator:/app/config:ro \
  -v /var/log/mistersmith:/app/logs \
  -p ${ORCHESTRATOR_PORT}:${ORCHESTRATOR_PORT} \
  -e LOG_LEVEL=info \
  -e CONFIG_PATH=/app/config/config.json \
  --health-cmd="grpc_health_probe -addr=:${ORCHESTRATOR_PORT}" \
  --health-interval=30s \
  --health-retries=3 \
  --health-timeout=10s \
  --health-start-period=60s \
  mistersmith/orchestrator:previous

# Step 7: Wait for service to be healthy
echo -e "${YELLOW}Waiting for orchestrator to become healthy...${NC}"
sleep 5  # Initial wait for container startup

if check_health $HEALTH_CHECK_RETRIES $HEALTH_CHECK_INTERVAL; then
    echo -e "${GREEN}✅ Orchestrator rollback successful${NC}"
    
    # Step 8: Verify service functionality
    echo -e "${YELLOW}Verifying service functionality...${NC}"
    
    # Test gRPC endpoint
    if grpcurl -plaintext localhost:${ORCHESTRATOR_PORT} list > /dev/null 2>&1; then
        echo -e "${GREEN}✅ gRPC endpoints accessible${NC}"
    else
        echo -e "${RED}⚠️  gRPC endpoints not fully accessible${NC}"
    fi
    
    # Check container logs for errors
    if docker logs mistersmith-orchestrator 2>&1 | tail -20 | grep -i error; then
        echo -e "${YELLOW}⚠️  Errors found in logs (see above)${NC}"
    else
        echo -e "${GREEN}✅ No errors in recent logs${NC}"
    fi
    
    # Log successful rollback
    echo "[$(date)] Orchestrator rollback completed successfully" >> /var/log/mistersmith/rollback.log
    exit 0
else
    echo -e "${RED}❌ Orchestrator health check failed after rollback${NC}"
    
    # Collect diagnostic information
    echo -e "${YELLOW}Collecting diagnostic information...${NC}"
    docker logs mistersmith-orchestrator > orchestrator-rollback-failure.log 2>&1
    docker inspect mistersmith-orchestrator >> orchestrator-rollback-failure.log
    
    echo -e "${RED}Diagnostic information saved to orchestrator-rollback-failure.log${NC}"
    echo "[$(date)] Orchestrator rollback failed" >> /var/log/mistersmith/rollback.log
    exit 1
fi