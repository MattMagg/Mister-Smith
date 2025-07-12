// Message transformation module for protocol translation

use crate::{NatsMessageEnvelope, AwsTarget, TranslationError};
use aws_sdk_eventbridge::types::PutEventsRequestEntry;
use chrono::{DateTime, Utc};
use serde_json::json;

pub struct MessageTransformations;

impl MessageTransformations {
    pub fn envelope_to_eventbridge(
        envelope: &NatsMessageEnvelope,
    ) -> Result<PutEventsRequestEntry, TranslationError> {
        let detail = json!({
            "messageId": envelope.id,
            "subject": envelope.subject,
            "senderId": envelope.sender_id,
            "correlationId": envelope.correlation_id,
            "payload": envelope.payload,
            "headers": envelope.headers,
            "originalTimestamp": envelope.timestamp,
        });
        
        let source = extract_source(&envelope.subject)?;
        let detail_type = extract_detail_type(&envelope.subject)?;
        
        Ok(PutEventsRequestEntry::builder()
            .time(DateTime::<Utc>::from_timestamp(envelope.timestamp, 0))
            .source(source)
            .detail_type(detail_type)
            .detail(serde_json::to_string(&detail)?)
            .event_bus_name("mistersmith-event-bus")
            .build())
    }
}

fn extract_source(subject: &str) -> Result<String, TranslationError> {
    let parts: Vec<&str> = subject.split('.').collect();
    if parts.is_empty() {
        return Err(TranslationError::InvalidSubjectFormat(subject.to_string()));
    }
    Ok(format!("mistersmith.{}", parts[0]))
}

fn extract_detail_type(subject: &str) -> Result<String, TranslationError> {
    let parts: Vec<&str> = subject.split('.').collect();
    if parts.len() < 2 {
        return Err(TranslationError::InvalidSubjectFormat(subject.to_string()));
    }
    Ok(parts[1..].join(" ").replace('-', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" "))
}