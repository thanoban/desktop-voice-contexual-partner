use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceProfile {
    PrivateLocal,
    Hybrid,
    CloudVoice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub provider_id: String,
    pub model_id: String,
    pub tested_revision: Option<String>,
    pub text: bool,
    pub audio_input: bool,
    pub audio_output: bool,
    pub vision: bool,
    pub tools: bool,
    pub streaming: bool,
    pub cancellation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageRecord {
    pub provider_id: String,
    pub run_id: Option<String>,
    pub input_units: Option<u64>,
    pub output_units: Option<u64>,
    pub cost_micros: Option<u64>,
    pub currency: Option<String>,
    pub provenance: String,
    pub recorded_at: i64,
}
