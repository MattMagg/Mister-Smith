//! Real-Time Collaboration Example - The MisterSmith Difference

use async_nats::Client;
use tokio::sync::broadcast;

/// What makes MisterSmith special: Agents that talk DURING work
pub async fn production_bug_hunt() -> Result<()> {
    // Setup NATS for real-time communication
    let nats = async_nats::connect("nats://localhost:4222").await?;
    
    // Orchestrator (you) starts the investigation
    println!("🎯 Orchestrator: Debug intermittent 502 errors on uploads");
    
    // Spawn collaborative agents (not isolated tasks!)
    let (tx, _) = broadcast::channel(100);
    
    // DevOps Agent starts investigating
    tokio::spawn({
        let nats = nats.clone();
        let tx = tx.clone();
        async move {
            println!("🔧 DevOpsAgent: Checking API gateway logs...");
            tokio::time::sleep(Duration::from_secs(2)).await;
            
            // DISCOVERS SOMETHING - BROADCASTS IMMEDIATELY
            nats.publish(
                "team.discovery",
                r#"{"agent": "devops", "finding": "Timeouts at exactly 60s"}"#.into()
            ).await?;
            
            println!("🔧 DevOpsAgent: Found timeouts at 60s mark!");
        }
    });
    
    // Debug Agent working simultaneously
    tokio::spawn({
        let nats = nats.clone();
        async move {
            // LISTENING while working
            let mut sub = nats.subscribe("team.discovery").await?;
            
            println!("🐛 DebugAgent: Analyzing background worker logs...");
            
            // Gets DevOps discovery IN REAL TIME
            if let Some(msg) = sub.next().await {
                println!("🐛 DebugAgent: Interesting! DevOps found 60s timeouts...");
                
                // CONNECTS THE DOTS
                println!("🐛 DebugAgent: Wait! Queue depth spikes at same times!");
                
                // SHARES INSIGHT IMMEDIATELY
                nats.publish(
                    "team.insight",
                    r#"{"correlation": "timeouts align with queue spikes"}"#.into()
                ).await?;
                
                // ASKS FOR HELP
                let response = nats.request(
                    "team.question",
                    "What could cause both symptoms?".into(),
                    Duration::from_secs(5)
                ).await?;
                
                println!("🐛 DebugAgent: Team suggests checking Redis...");
                
                // INVESTIGATES BASED ON COLLABORATION
                println!("🐛 DebugAgent: FOUND IT! Redis connection pool exhausted!");
                
                // PROPOSES SOLUTION
                nats.publish(
                    "team.solution",
                    r#"{"fix": "Increase Redis pool from 10 to 50"}"#.into()
                ).await?;
            }
        }
    });
    
    // Performance Agent joins the hunt
    tokio::spawn({
        let nats = nats.clone();
        async move {
            let mut sub = nats.subscribe("team.insight").await?;
            
            println!("📊 PerfAgent: Monitoring system metrics...");
            
            // RECEIVES correlation insight
            if let Some(_) = sub.next().await {
                println!("📊 PerfAgent: Correlation confirmed! Memory spike too!");
                
                // VALIDATES proposed solution
                let mut solution_sub = nats.subscribe("team.solution").await?;
                if let Some(_) = solution_sub.next().await {
                    println!("📊 PerfAgent: Modeling fix... Yes! This will work!");
                    
                    nats.publish(
                        "team.consensus",
                        r#"{"validated": true, "confidence": 0.95}"#.into()
                    ).await?;
                }
            }
        }
    });
    
    // Orchestrator monitors and guides
    let mut discovery_sub = nats.subscribe("team.>").await?; // Wildcard subscription
    
    while let Some(msg) = discovery_sub.next().await {
        match msg.subject.as_str() {
            "team.discovery" => println!("🎯 Orchestrator: Good find, keep digging!"),
            "team.insight" => println!("🎯 Orchestrator: Excellent correlation!"),
            "team.question" => {
                // Orchestrator can guide
                nats.publish("team.guidance", "Check shared resources".into()).await?;
            }
            "team.solution" => println!("🎯 Orchestrator: Test in staging first!"),
            "team.consensus" => {
                println!("🎯 Orchestrator: Team consensus reached! Implementing fix...");
                break;
            }
            _ => {}
        }
    }
    
    println!("\n✅ Bug fixed in 10 minutes through collaboration!");
    println!("   Without MisterSmith: 2+ hours of isolated investigation");
    
    Ok(())
}

/// The patterns that make this powerful
pub mod collaboration_patterns {
    pub enum Pattern {
        /// Agents share discoveries immediately
        BroadcastDiscovery {
            agent: String,
            finding: String,
            timestamp: Instant,
        },
        
        /// Agents ask for help mid-task
        RequestAssistance {
            agent: String,
            question: String,
            context: Vec<String>,
        },
        
        /// Agents build on each other's work
        BuildOnInsight {
            original: String,
            enhanced_by: String,
            new_understanding: String,
        },
        
        /// Orchestrator guides without micromanaging
        GentleGuidance {
            suggestion: String,
            not_command: bool,
        },
        
        /// Team reaches consensus
        EmergentConsensus {
            proposal: String,
            validators: Vec<String>,
            confidence: f64,
        },
    }
}

/// Why this is revolutionary
pub fn why_this_matters() -> &'static str {
    "In vanilla Claude CLI, tasks are like developers working from home with email.
     
     In MisterSmith, agents are like a team in a war room with whiteboards,
     shouting discoveries, building on insights, and solving problems TOGETHER.
     
     The difference isn't just speed - it's finding solutions that isolated 
     intelligence simply cannot discover."
}

// Real-time collaboration: Where 1 + 1 = 3