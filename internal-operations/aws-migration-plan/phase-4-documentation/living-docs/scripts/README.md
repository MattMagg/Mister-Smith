# Living Documentation Automation Scripts

## Overview

This directory contains automation scripts for maintaining living documentation throughout the AWS migration process. These scripts ensure documentation stays current and accurate.

## Scripts

### 1. generate-index.sh
**Purpose**: Aggregates all phase trackers into a master index  
**Schedule**: Every 4 hours via CI/CD  
**Usage**:
```bash
./generate-index.sh
```
**Output**: Updates `/aws-migration-plan/index.md`

### 2. update-status.sh
**Purpose**: Updates current phase status and metrics  
**Usage**:
```bash
./update-status.sh <phase> <status> <progress> "<notes>"

# Example:
./update-status.sh 4 IN_PROGRESS 75 "Completed runbook templates"
```
**Valid Status Values**: NOT_STARTED, IN_PROGRESS, BLOCKED, COMPLETE, FAILED

### 3. collect-metrics.sh
**Purpose**: Gathers progress metrics from all phases  
**Schedule**: Every 6 hours  
**Usage**:
```bash
./collect-metrics.sh
```
**Output**: 
- Updates `progress-metrics/dashboard.json`
- Appends to `progress-metrics/kpi-tracking.csv`

### 4. verify-all.sh
**Purpose**: Runs all verification scripts  
**Schedule**: Daily at 2 AM UTC  
**Usage**:
```bash
./verify-all.sh
```
**Output**:
- Updates `verification-results/latest-validation.json`
- Archives results in `verification-results/historical-results/`

## Environment Variables

```bash
# Optional: Override base directory
export AWS_MIGRATION_DIR="/path/to/migration/docs"

# AWS CLI configuration (required for some scripts)
export AWS_REGION="us-east-1"
export AWS_PROFILE="migration-admin"
```

## CI/CD Integration

### GitHub Actions Example
```yaml
name: Update Migration Documentation

on:
  schedule:
    - cron: '0 */4 * * *'  # Every 4 hours
  workflow_dispatch:       # Manual trigger

jobs:
  update-docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Configure AWS
        uses: aws-actions/configure-aws-credentials@v2
        with:
          aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
          aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          aws-region: us-east-1
      
      - name: Generate Index
        run: |
          cd aws-migration-plan/phase-4-documentation/living-docs/scripts
          ./generate-index.sh
      
      - name: Collect Metrics
        run: |
          cd aws-migration-plan/phase-4-documentation/living-docs/scripts
          ./collect-metrics.sh
      
      - name: Run Verifications
        run: |
          cd aws-migration-plan/phase-4-documentation/living-docs/scripts
          ./verify-all.sh || true  # Don't fail the workflow
      
      - name: Commit Updates
        run: |
          git config user.name "Migration Bot"
          git config user.email "bot@mistersmith.com"
          git add -A
          git diff --staged --quiet || git commit -m "🤖 Auto-update migration documentation [skip ci]"
          git push
```

## Script Dependencies

### Required Tools
- bash 4.0+
- jq (JSON processor)
- AWS CLI v2
- curl
- bc (basic calculator)
- grep, sed, awk (standard Unix tools)

### Install Dependencies
```bash
# macOS
brew install jq awscli

# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y jq awscli bc

# Amazon Linux 2
sudo yum install -y jq aws-cli bc
```

## Error Handling

All scripts include:
- `set -euo pipefail` for strict error handling
- Input validation
- Meaningful error messages
- Non-zero exit codes on failure

## Logging

Scripts output to stdout/stderr. For persistent logging:
```bash
# Run with logging
./script.sh 2>&1 | tee -a /var/log/migration-scripts.log
```

## Monitoring

### CloudWatch Integration
Scripts can send metrics to CloudWatch:
```bash
# Example: Send custom metric
aws cloudwatch put-metric-data \
  --namespace "MisterSmith/Migration" \
  --metric-name "ScriptExecutionTime" \
  --value $DURATION \
  --dimensions Script=generate-index
```

### Notifications
Scripts integrate with Claude Flow for notifications:
```bash
npx claude-flow hooks notification --message "Status update complete"
```

## Testing

### Dry Run Mode
Most scripts support dry run:
```bash
DRY_RUN=1 ./update-status.sh 4 IN_PROGRESS 50 "Test update"
```

### Unit Tests
See `tests/` directory for script tests.

## Troubleshooting

### Common Issues

1. **Permission Denied**
   ```bash
   chmod +x *.sh
   ```

2. **jq: command not found**
   ```bash
   # Install jq (see Dependencies section)
   ```

3. **AWS credentials not configured**
   ```bash
   aws configure
   # Or use environment variables
   ```

4. **Script fails silently**
   ```bash
   # Run with debug mode
   bash -x ./script.sh
   ```

## Contributing

1. Test changes locally first
2. Ensure scripts are idempotent
3. Add error handling for new features
4. Update this README
5. Submit PR with test results

## Support

- **Slack**: #aws-migration-automation
- **Email**: devops@mistersmith.com
- **On-call**: See PagerDuty schedule

---

**Last Updated**: 2025-01-11  
**Maintained By**: MisterSmith DevOps Team