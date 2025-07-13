#!/bin/bash
# rollback-database.sh
# Database rollback procedure for MisterSmith PostgreSQL

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
DB_NAME="mistersmith"
DB_USER="mistersmith"
BACKUP_BUCKET="s3://mistersmith-backups"
BACKUP_DIR="/var/backups/mistersmith"
LOG_FILE="/var/log/mistersmith/db-rollback-$(date +%Y%m%d-%H%M%S).log"

# Ensure log directory exists
mkdir -p /var/log/mistersmith

echo -e "${YELLOW}Starting Database rollback...${NC}" | tee -a $LOG_FILE

# Function to execute SQL as postgres user
exec_sql() {
    sudo -u postgres psql -c "$1" 2>&1 | tee -a $LOG_FILE
}

# Function to check database connectivity
check_db_connection() {
    sudo -u postgres pg_isready -d $DB_NAME > /dev/null 2>&1
}

# Step 1: Identify latest backup
echo -e "${YELLOW}Identifying latest database backup...${NC}" | tee -a $LOG_FILE

# Check local backups first
LOCAL_BACKUP=$(ls -t $BACKUP_DIR/mistersmith-backup-*.sql 2>/dev/null | head -1)

if [ -z "$LOCAL_BACKUP" ]; then
    echo -e "${YELLOW}No local backup found, downloading from S3...${NC}" | tee -a $LOG_FILE
    mkdir -p $BACKUP_DIR
    
    # List and download latest backup from S3
    LATEST_S3_BACKUP=$(aws s3 ls ${BACKUP_BUCKET}/pre-migration/ | grep "mistersmith-backup-" | sort | tail -1 | awk '{print $4}')
    
    if [ -z "$LATEST_S3_BACKUP" ]; then
        echo -e "${RED}No backup found in S3!${NC}" | tee -a $LOG_FILE
        exit 1
    fi
    
    aws s3 cp ${BACKUP_BUCKET}/pre-migration/${LATEST_S3_BACKUP} $BACKUP_DIR/
    LOCAL_BACKUP="$BACKUP_DIR/${LATEST_S3_BACKUP}"
fi

echo -e "${GREEN}Using backup: $LOCAL_BACKUP${NC}" | tee -a $LOG_FILE

# Step 2: Create point-in-time backup before rollback
echo -e "${YELLOW}Creating point-in-time backup before rollback...${NC}" | tee -a $LOG_FILE
PITR_BACKUP="$BACKUP_DIR/mistersmith-pitr-$(date +%Y%m%d-%H%M%S).sql"

if check_db_connection; then
    sudo -u postgres pg_dump -d $DB_NAME > $PITR_BACKUP 2>&1 | tee -a $LOG_FILE
    echo -e "${GREEN}Point-in-time backup created: $PITR_BACKUP${NC}" | tee -a $LOG_FILE
else
    echo -e "${YELLOW}Database not accessible for point-in-time backup${NC}" | tee -a $LOG_FILE
fi

# Step 3: Terminate all active connections
echo -e "${YELLOW}Terminating all active database connections...${NC}" | tee -a $LOG_FILE
exec_sql "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '$DB_NAME' AND pid <> pg_backend_pid();"

# Step 4: Drop current database
echo -e "${YELLOW}Dropping current database...${NC}" | tee -a $LOG_FILE
exec_sql "DROP DATABASE IF EXISTS $DB_NAME;"

# Step 5: Create fresh database
echo -e "${YELLOW}Creating fresh database...${NC}" | tee -a $LOG_FILE
exec_sql "CREATE DATABASE $DB_NAME OWNER $DB_USER;"

# Step 6: Restore from backup
echo -e "${YELLOW}Restoring database from backup...${NC}" | tee -a $LOG_FILE
sudo -u postgres psql -d $DB_NAME < $LOCAL_BACKUP 2>&1 | tee -a $LOG_FILE

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Database restored successfully${NC}" | tee -a $LOG_FILE
else
    echo -e "${RED}❌ Database restore failed${NC}" | tee -a $LOG_FILE
    exit 1
fi

# Step 7: Verify restoration
echo -e "${YELLOW}Verifying database restoration...${NC}" | tee -a $LOG_FILE

# Check table existence
TABLES=$(exec_sql "\dt" | grep -E "(agents|tasks|discoveries|hive_sessions)" | wc -l)
if [ $TABLES -lt 4 ]; then
    echo -e "${RED}❌ Not all required tables found${NC}" | tee -a $LOG_FILE
    exit 1
fi

# Verify data integrity
echo -e "${YELLOW}Running data integrity checks...${NC}" | tee -a $LOG_FILE

# Check record counts
exec_sql "SELECT 'Agents' as table_name, COUNT(*) as count FROM agents UNION ALL SELECT 'Tasks', COUNT(*) FROM tasks UNION ALL SELECT 'Discoveries', COUNT(*) FROM discoveries UNION ALL SELECT 'Sessions', COUNT(*) FROM hive_sessions;"

# Check for orphaned records
ORPHANED=$(exec_sql "SELECT COUNT(*) FROM tasks t LEFT JOIN agents a ON t.assigned_agent_id = a.id WHERE t.assigned_agent_id IS NOT NULL AND a.id IS NULL;" | grep -o '[0-9]*' | head -1)

if [ "$ORPHANED" -gt 0 ]; then
    echo -e "${YELLOW}⚠️  Found $ORPHANED orphaned task records${NC}" | tee -a $LOG_FILE
    # Optionally clean up orphaned records
    exec_sql "UPDATE tasks SET assigned_agent_id = NULL WHERE assigned_agent_id IN (SELECT t.assigned_agent_id FROM tasks t LEFT JOIN agents a ON t.assigned_agent_id = a.id WHERE t.assigned_agent_id IS NOT NULL AND a.id IS NULL);"
fi

# Step 8: Update sequences
echo -e "${YELLOW}Updating database sequences...${NC}" | tee -a $LOG_FILE
exec_sql "SELECT setval('agents_id_seq', (SELECT MAX(id) FROM agents));"
exec_sql "SELECT setval('tasks_id_seq', (SELECT MAX(id) FROM tasks));"
exec_sql "SELECT setval('discoveries_id_seq', (SELECT MAX(id) FROM discoveries));"

# Step 9: Analyze database for query optimization
echo -e "${YELLOW}Analyzing database for query optimization...${NC}" | tee -a $LOG_FILE
exec_sql "ANALYZE;"

# Step 10: Grant permissions
echo -e "${YELLOW}Ensuring correct permissions...${NC}" | tee -a $LOG_FILE
exec_sql "GRANT ALL PRIVILEGES ON DATABASE $DB_NAME TO $DB_USER;"
exec_sql "GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO $DB_USER;"
exec_sql "GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO $DB_USER;"

# Final verification
if check_db_connection; then
    echo -e "${GREEN}✅ Database rollback completed successfully${NC}" | tee -a $LOG_FILE
    
    # Generate summary report
    echo -e "\n${YELLOW}=== Database Rollback Summary ===${NC}" | tee -a $LOG_FILE
    echo "Backup used: $LOCAL_BACKUP" | tee -a $LOG_FILE
    echo "Rollback completed at: $(date)" | tee -a $LOG_FILE
    exec_sql "SELECT version();"
    
    exit 0
else
    echo -e "${RED}❌ Database connectivity check failed after rollback${NC}" | tee -a $LOG_FILE
    exit 1
fi