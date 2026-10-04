use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::CONTRACT_SCHEMA_VERSION;

/// Transitional payload for legacy UI events until ordered session events are wired.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectEvent<T> {
    pub project_id: String,
    pub payload: T,
}

impl<T> ProjectEvent<T> {
    pub fn new(project_id: &str, payload: T) -> Self {
        Self {
            project_id: project_id.into(),
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub schema_version: u16,
    pub event_id: String,
    pub sequence: u64,
    pub occurred_at: i64,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub run_id: Option<String>,
    pub kind: String,
    pub payload: Value,
}

impl EventEnvelope {
    pub fn new(kind: impl Into<String>, sequence: u64, payload: Value) -> Self {
        Self {
            schema_version: CONTRACT_SCHEMA_VERSION,
            event_id: uuid::Uuid::new_v4().to_string(),
            sequence,
            occurred_at: super::now_ms(),
            session_id: None,
            turn_id: None,
            run_id: None,
            kind: kind.into(),
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub correlation_id: String,
}

#[cfg(test)]
mod tests {
    use super::EventEnvelope;

    #[test]
    fn event_fixture_deserializes() {
        let fixture = include_str!("../../../contracts/fixtures/event.turn_started.json");
        let event: EventEnvelope = serde_json::from_str(fixture).expect("valid event fixture");
        assert_eq!(event.schema_version, 1);
        assert_eq!(event.kind, "turn.started");
        assert_eq!(event.sequence, 1);
    }
}
