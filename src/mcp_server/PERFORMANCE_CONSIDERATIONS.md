# Discovery State Management - Performance Considerations

## Overview

The `DiscoveryStore` implementation prioritizes thread-safety and efficient lookups while maintaining bounded memory usage. This document outlines the performance characteristics and trade-offs.

## Performance Characteristics

### 1. **Storage Complexity**
- **Insert**: O(1) amortized
  - HashMap insertions for main storage
  - Vec push operations for indices
  - VecDeque push for ordering
- **Eviction**: O(k) where k is the number of indices
  - Requires updating multiple index structures
  - Only triggered when reaching capacity (1000 discoveries)

### 2. **Query Performance**
- **Get by ID**: O(1) - Direct HashMap lookup
- **Get by Agent**: O(n) where n is discoveries per agent
  - O(1) to find agent's discovery IDs
  - O(n) to retrieve each discovery
- **Get by Type**: O(m) where m is discoveries per type
  - Similar to agent queries
- **Subscription Matching**: O(s*f) where s is subscribers, f is filter checks
  - Each filter check is constant time

### 3. **Memory Usage**
- **Base overhead**: ~200 bytes per discovery (excluding content)
- **Index overhead**: ~24 bytes per discovery per index
- **Total capacity**: 1000 discoveries (configurable via MAX_DISCOVERIES)
- **Estimated max memory**: ~500KB-2MB depending on content size

## Concurrency Considerations

### 1. **Lock Granularity**
The implementation uses separate RwLocks for different data structures:
```rust
discoveries: Arc<RwLock<HashMap<String, Discovery>>>,
by_agent: Arc<RwLock<HashMap<String, Vec<String>>>>,
by_type: Arc<RwLock<HashMap<DiscoveryType, Vec<String>>>>,
subscriptions: Arc<RwLock<HashMap<String, DiscoveryFilter>>>,
```

**Benefits**:
- Read operations don't block each other
- Different indices can be updated concurrently
- Subscription management independent of discovery storage

**Trade-offs**:
- Multiple locks must be acquired for write operations
- Potential for lock ordering issues (mitigated by consistent ordering)

### 2. **Read vs Write Performance**
- **Reads**: Can proceed concurrently, excellent for read-heavy workloads
- **Writes**: Sequential due to write locks, but fast due to simple operations
- **Recommendation**: Ideal for scenarios with 80%+ read operations

## Optimization Opportunities

### 1. **Batch Operations**
For high-throughput scenarios, consider implementing:
```rust
pub async fn store_discoveries_batch(&self, discoveries: Vec<Discovery>) -> Result<Vec<String>>
```
Benefits: Amortize lock acquisition costs

### 2. **Partial Indices**
For very large deployments, consider:
- Bloom filters for quick negative lookups
- Time-based partitioning for recent discoveries
- Compressed indices for older data

### 3. **Subscription Optimization**
Current implementation checks all subscriptions for each discovery. For many subscribers:
- Pre-compute subscription indices by type/role
- Use subscription trees for hierarchical filtering
- Cache subscription match results

## Bottlenecks and Mitigations

### 1. **Eviction Under Load**
**Issue**: FIFO eviction requires updating multiple indices
**Mitigation**: 
- Increase MAX_DISCOVERIES for high-volume deployments
- Implement background eviction thread
- Use time-based eviction policies

### 2. **Large Agent/Type Indices**
**Issue**: Agents producing many discoveries create large index vectors
**Mitigation**:
- Implement per-agent discovery limits
- Use circular buffers for agent histories
- Archive old discoveries to persistent storage

### 3. **Subscription Evaluation**
**Issue**: O(n) subscription checking for each discovery
**Mitigation**:
- Pre-filter subscriptions by type
- Use bitset for fast type matching
- Implement subscription priority levels

## Recommended Usage Patterns

### 1. **High-Frequency Discovery Producers**
```rust
// Batch discoveries when possible
let discoveries = collect_discoveries_for_period().await;
for discovery in discoveries {
    store.store_discovery(discovery).await?;
}
```

### 2. **Efficient Queries**
```rust
// Prefer specific queries over broad filters
let agent_discoveries = store.get_discoveries_for_agent("specific-agent").await;

// Avoid repeated full scans
let filter = DiscoveryFilter { /* specific criteria */ };
let matching = store.get_discoveries_by_filter(&filter).await;
```

### 3. **Subscription Management**
```rust
// Use focused subscriptions
let filter = DiscoveryFilter {
    types: Some(vec![DiscoveryType::Anomaly]), // Specific types
    min_confidence: Some(0.8),                  // High threshold
    // Avoid broad keyword searches
};
```

## Monitoring Recommendations

### Key Metrics to Track:
1. **Store capacity** - `stats.capacity_used`
2. **Eviction rate** - Log when evictions occur
3. **Query latency** - Time discovery retrievals
4. **Lock contention** - Monitor RwLock wait times
5. **Subscription match rate** - Ratio of matches to checks

### Performance Baselines:
- Insert latency: < 1ms under normal load
- Query by ID: < 100μs
- Query by agent/type: < 5ms for typical result sets
- Subscription matching: < 1ms for 100 subscribers

## Future Enhancements

1. **Persistent Storage Integration**
   - Write-through cache to PostgreSQL
   - Async write-behind for durability
   - Query federation across memory and disk

2. **Distributed State**
   - Redis-backed shared state
   - Consistent hashing for sharding
   - Multi-node discovery clusters

3. **Advanced Indexing**
   - Full-text search on content
   - Semantic similarity matching
   - Graph-based discovery relationships

## Conclusion

The current implementation provides excellent performance for:
- Moderate discovery rates (< 100/second)
- Read-heavy workloads
- Bounded memory environments

For extreme scale, consider the optimization strategies outlined above or implementing a tiered storage approach with hot/warm/cold discovery pools.