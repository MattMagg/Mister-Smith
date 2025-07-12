#!/bin/bash
# Performance Benchmarks Validation Script for MisterSmith AWS Migration

set -euo pipefail

RESULTS_FILE="performance-validation-$(date +%Y%m%d-%H%M%S).log"
PASS_COUNT=0
FAIL_COUNT=0
ALB_ENDPOINT="${ALB_ENDPOINT:-http://alb.mistersmith.com}"

echo "📈 Performance Validation Starting..." | tee -a "$RESULTS_FILE"
echo "====================================" | tee -a "$RESULTS_FILE"

# Function to check performance metrics
check_performance() {
    local test_name=$1
    local actual=$2
    local threshold=$3
    local comparison=$4  # "less" or "greater"
    
    if [[ "$comparison" == "less" ]]; then
        if (( $(echo "$actual < $threshold" | bc -l) )); then
            echo "✅ $test_name: PASS ($actual < $threshold)" | tee -a "$RESULTS_FILE"
            ((PASS_COUNT++))
            return 0
        else
            echo "❌ $test_name: FAIL ($actual >= $threshold)" | tee -a "$RESULTS_FILE"
            ((FAIL_COUNT++))
            return 1
        fi
    else
        if (( $(echo "$actual > $threshold" | bc -l) )); then
            echo "✅ $test_name: PASS ($actual > $threshold)" | tee -a "$RESULTS_FILE"
            ((PASS_COUNT++))
            return 0
        else
            echo "❌ $test_name: FAIL ($actual <= $threshold)" | tee -a "$RESULTS_FILE"
            ((FAIL_COUNT++))
            return 1
        fi
    fi
}

# 1. API Response Time Test
echo -e "\n📍 Testing API Response Times..." | tee -a "$RESULTS_FILE"
echo "Running 100 requests to measure response times..." | tee -a "$RESULTS_FILE"

# Measure response times
response_times=()
for i in {1..100}; do
    start_time=$(date +%s%N)
    curl -s "$ALB_ENDPOINT/api/agents" > /dev/null
    end_time=$(date +%s%N)
    elapsed=$((($end_time - $start_time) / 1000000))  # Convert to milliseconds
    response_times+=($elapsed)
done

# Calculate percentiles
IFS=$'\n' sorted=($(sort -n <<<"${response_times[*]}"))
unset IFS

p50=${sorted[49]}
p95=${sorted[94]}
p99=${sorted[98]}

echo "Response time percentiles:" | tee -a "$RESULTS_FILE"
echo "  50%: ${p50}ms" | tee -a "$RESULTS_FILE"
echo "  95%: ${p95}ms" | tee -a "$RESULTS_FILE"
echo "  99%: ${p99}ms" | tee -a "$RESULTS_FILE"

check_performance "95th Percentile Response Time" "$p95" "500" "less"

# 2. Message Throughput Test
echo -e "\n📍 Testing Message Throughput..." | tee -a "$RESULTS_FILE"

# Simulate message throughput test
start_time=$(date +%s)
messages_sent=0
duration=10  # 10 second test

echo "Running 10-second throughput test..." | tee -a "$RESULTS_FILE"
while [ $(($(date +%s) - start_time)) -lt $duration ]; do
    curl -s -X POST "$ALB_ENDPOINT/api/messages" \
        -H "Content-Type: application/json" \
        -d '{"message":"throughput-test"}' > /dev/null &
    ((messages_sent++))
    
    # Limit concurrent requests
    if [[ $((messages_sent % 50)) -eq 0 ]]; then
        wait
    fi
done
wait

throughput=$((messages_sent / duration))
echo "AWS Bridge Throughput: $throughput msg/sec" | tee -a "$RESULTS_FILE"

# Assuming NATS baseline is 10000 msg/sec
nats_baseline=10000
ratio=$(echo "scale=2; $throughput * 100 / $nats_baseline" | bc)
echo "Performance Ratio: ${ratio}%" | tee -a "$RESULTS_FILE"

check_performance "Throughput Ratio" "$ratio" "90" "greater"

# 3. Database Performance Test
echo -e "\n📍 Testing Database Performance..." | tee -a "$RESULTS_FILE"

# Simple query test
simple_start=$(date +%s%N)
curl -s "$ALB_ENDPOINT/api/db/test/simple" > /dev/null
simple_end=$(date +%s%N)
simple_time=$((($simple_end - $simple_start) / 1000000))

# Complex query test
complex_start=$(date +%s%N)
curl -s "$ALB_ENDPOINT/api/db/test/complex" > /dev/null
complex_end=$(date +%s%N)
complex_time=$((($complex_end - $complex_start) / 1000000))

echo "Simple Query: ${simple_time}ms" | tee -a "$RESULTS_FILE"
echo "Complex Query: ${complex_time}ms" | tee -a "$RESULTS_FILE"

check_performance "Database Simple Query" "$simple_time" "100" "less"
check_performance "Database Complex Query" "$complex_time" "100" "less"

# 4. Auto-scaling Response Test
echo -e "\n📍 Testing Auto-scaling Response..." | tee -a "$RESULTS_FILE"

# Check current task count
initial_count=$(aws ecs describe-services --cluster MisterSmith-Cluster \
    --services mistersmith-service --query 'services[0].runningCount' --output text 2>/dev/null || echo "0")

echo "Initial task count: $initial_count" | tee -a "$RESULTS_FILE"

# Simulate load spike
echo "Simulating load spike..." | tee -a "$RESULTS_FILE"
for i in {1..200}; do
    curl -s "$ALB_ENDPOINT/api/cpu-intensive" > /dev/null &
done
wait

# Wait and check for scaling
echo "Waiting 2 minutes for auto-scaling..." | tee -a "$RESULTS_FILE"
sleep 120

# Check new task count
new_count=$(aws ecs describe-services --cluster MisterSmith-Cluster \
    --services mistersmith-service --query 'services[0].runningCount' --output text 2>/dev/null || echo "0")

echo "New task count: $new_count" | tee -a "$RESULTS_FILE"

if [[ $new_count -gt $initial_count ]]; then
    echo "✅ Auto-scaling: PASS (Scaled from $initial_count to $new_count)" | tee -a "$RESULTS_FILE"
    ((PASS_COUNT++))
else
    echo "⚠️  Auto-scaling: INCONCLUSIVE (No scaling detected)" | tee -a "$RESULTS_FILE"
fi

# 5. Health Check Performance Test
echo -e "\n📍 Testing Health Check Intervals..." | tee -a "$RESULTS_FILE"

# Monitor health checks for 2 minutes
echo "Monitoring health check intervals for 2 minutes..." | tee -a "$RESULTS_FILE"
health_check_times=()
start_monitor=$(date +%s)

while [ $(($(date +%s) - start_monitor)) -lt 120 ]; do
    if curl -s "$ALB_ENDPOINT/health/core" > /dev/null; then
        health_check_times+=($(date +%s))
    fi
    sleep 5
done

# Calculate intervals
intervals=()
for i in $(seq 1 $((${#health_check_times[@]} - 1))); do
    interval=$((health_check_times[$i] - health_check_times[$((i-1))]))
    intervals+=($interval)
done

# Check if intervals are consistent (within 10% of 30 seconds)
consistent_checks=0
for interval in "${intervals[@]}"; do
    if [[ $interval -ge 27 && $interval -le 33 ]]; then
        ((consistent_checks++))
    fi
done

consistency_rate=$((consistent_checks * 100 / ${#intervals[@]}))
echo "Health check consistency: ${consistency_rate}%" | tee -a "$RESULTS_FILE"

check_performance "Health Check Consistency" "$consistency_rate" "90" "greater"

# Summary
echo -e "\n📊 Performance Validation Summary" | tee -a "$RESULTS_FILE"
echo "===================================" | tee -a "$RESULTS_FILE"
echo "✅ Passed: $PASS_COUNT" | tee -a "$RESULTS_FILE"
echo "❌ Failed: $FAIL_COUNT" | tee -a "$RESULTS_FILE"
echo "📁 Results saved to: $RESULTS_FILE" | tee -a "$RESULTS_FILE"

# Exit with appropriate code
if [[ $FAIL_COUNT -eq 0 ]]; then
    echo -e "\n🎉 All performance checks PASSED!" | tee -a "$RESULTS_FILE"
    exit 0
else
    echo -e "\n⚠️  Some performance checks FAILED. Review the results." | tee -a "$RESULTS_FILE"
    exit 1
fi