use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionLocality {
    LocalProcess,
    LocalNetwork,
    Remote,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownstreamDataPolicy {
    LocalVerified,
    MayUseCloud,
    Cloud,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionHealth {
    Unknown,
    Healthy,
    Degraded,
    Unavailable,
    AuthenticationRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    Tool,
    Workflow,
    Agent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Read,
    Write,
    Send,
    Delete,
    Execute,
    Schedule,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntegrationConnection {
    pub id: String,
    pub adapter_id: String,
    pub display_name: String,
    pub endpoint: Option<String>,
    pub credential_ref: Option<String>,
    pub enabled: bool,
    pub locality: ConnectionLocality,
    pub downstream_data_policy: DownstreamDataPolicy,
    pub health: ConnectionHealth,
    pub supported_version: Option<String>,
    pub tested_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Capability {
    pub id: String,
    pub connection_id: String,
    pub display_name: String,
    pub kind: CapabilityKind,
    pub input_schema: Value,
    pub output_schema: Option<Value>,
    pub schema_revision: String,
    pub schema_hash: String,
    pub effects: Vec<Effect>,
    pub supports_progress: bool,
    pub supports_cancellation: bool,
    pub destinations: Vec<String>,
}

impl Capability {
    pub fn requires_review(&self) -> bool {
        self.effects.is_empty()
            || self.effects.iter().any(|effect| {
                matches!(
                    effect,
                    Effect::Write
                        | Effect::Send
                        | Effect::Delete
                        | Effect::Execute
                        | Effect::Schedule
                        | Effect::Unknown
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{Capability, CapabilityKind, Effect};

    fn capability(effects: Vec<Effect>) -> Capability {
        Capability {
            id: "fake.read".into(),
            connection_id: "connection-1".into(),
            display_name: "Read".into(),
            kind: CapabilityKind::Tool,
            input_schema: serde_json::json!({"type": "object"}),
            output_schema: None,
            schema_revision: "1".into(),
            schema_hash: "fixture".into(),
            effects,
            supports_progress: false,
            supports_cancellation: false,
            destinations: vec![],
        }
    }

    #[test]
    fn read_only_capability_does_not_require_review() {
        assert!(!capability(vec![Effect::Read]).requires_review());
    }

    #[test]
    fn unknown_or_mutating_capability_requires_review() {
        assert!(capability(vec![]).requires_review());
        assert!(capability(vec![Effect::Unknown]).requires_review());
        assert!(capability(vec![Effect::Read, Effect::Send]).requires_review());
    }
}
