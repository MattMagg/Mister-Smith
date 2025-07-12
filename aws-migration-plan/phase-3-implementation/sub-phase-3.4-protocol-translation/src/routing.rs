// Routing module for protocol translation

use crate::{AwsTarget, TranslationError};
use std::collections::HashMap;

pub struct RoutingRules {
    static_routes: HashMap<String, String>,
    pattern_routes: Vec<(regex::Regex, String)>,
}

impl RoutingRules {
    pub fn new() -> Self {
        Self {
            static_routes: HashMap::new(),
            pattern_routes: Vec::new(),
        }
    }
    
    pub fn add_static_route(&mut self, subject: &str, target: &str) {
        self.static_routes.insert(subject.to_string(), target.to_string());
    }
    
    pub fn add_pattern_route(&mut self, pattern: &str, target: &str) -> Result<(), TranslationError> {
        let regex = regex::Regex::new(pattern)
            .map_err(|e| TranslationError::InvalidSubjectFormat(e.to_string()))?;
        self.pattern_routes.push((regex, target.to_string()));
        Ok(())
    }
    
    pub fn route(&self, subject: &str) -> Option<String> {
        // Check static routes first
        if let Some(target) = self.static_routes.get(subject) {
            return Some(target.clone());
        }
        
        // Check pattern routes
        for (pattern, target) in &self.pattern_routes {
            if pattern.is_match(subject) {
                return Some(target.clone());
            }
        }
        
        None
    }
}