//! Integration Tests for MisterSmith Framework
//!
//! End-to-end testing of framework components working together.

#[cfg(test)]
pub mod claude_cli;
#[cfg(test)]
pub mod agent_lifecycle;
#[cfg(test)]
pub mod concurrent_agents;
#[cfg(test)]
pub mod error_scenarios;
#[cfg(test)]
pub mod claude_executor_test;