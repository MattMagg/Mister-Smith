//! Quick compile test for supervision module

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_supervision_module_compiles() {
        // Create basic components
        let tree = SupervisionTree::new(SupervisionTreeConfig::default());
        let root = Arc::new(RootSupervisor::new(SupervisionStrategy::OneForAll));
        let root_node = Arc::new(SupervisorNode::new(
            NodeId("root".to_string()),
            NodeType::RootSupervisor,
            None,
            root,
        ));
        
        // Basic operations
        tree.set_root(root_node).await.unwrap();
        assert_eq!(tree.all_nodes().await.len(), 1);
        
        // Circuit breaker
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        let result = cb.call(async { Ok::<_, std::io::Error>(42) }).await;
        assert!(result.is_ok());
        
        // Failure detector
        let detector = FailureDetector::new(FailureDetectorConfig::default());
        let node_id = NodeId("test".to_string());
        detector.monitor_node(node_id.clone()).await.unwrap();
        detector.record_heartbeat(&node_id).await.unwrap();
    }
}