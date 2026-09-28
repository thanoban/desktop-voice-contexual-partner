use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Queued,
    AwaitingApproval,
    Running,
    Succeeded,
    Failed,
    CancellationRequested,
    Cancelled,
    OutcomeUnknown,
}

impl RunState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }

    pub fn may_transition_to(self, next: Self) -> bool {
        use RunState::*;
        matches!(
            (self, next),
            (Queued, AwaitingApproval | Running | Cancelled | Failed)
                | (AwaitingApproval, Queued | Cancelled)
                | (
                    Running,
                    AwaitingApproval | Succeeded | Failed | CancellationRequested | OutcomeUnknown
                )
                | (
                    CancellationRequested,
                    Cancelled | Succeeded | Failed | OutcomeUnknown
                )
                | (OutcomeUnknown, Succeeded | Failed | Cancelled)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub id: String,
    pub capability_id: String,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub arguments: Value,
    pub objective: Option<String>,
    pub context_refs: Vec<String>,
    pub grant_id: Option<String>,
    pub deadline_at: Option<i64>,
    pub budget_micros: Option<u64>,
    pub ancestry: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionRun {
    pub id: String,
    pub request_id: String,
    pub external_id: Option<String>,
    pub executor_id: String,
    pub project_id: Option<String>,
    pub approved_request_hash: Option<String>,
    pub state: RunState,
    pub sequence: u64,
    pub created_at: i64,
    pub updated_at: i64,
    pub verification_state: String,
    pub artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Pending,
    Approved,
    Denied,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub run_id: String,
    pub operation_hash: String,
    pub target: String,
    pub action: String,
    pub change_preview: Option<String>,
    pub data_destinations: Vec<String>,
    pub expires_at: i64,
    pub downstream_approval_ref: Option<String>,
    pub decision: ApprovalDecision,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub run_id: String,
    pub outcome: RunState,
    pub summary: String,
    pub artifact_refs: Vec<String>,
    pub evidence: Vec<String>,
    pub external_refs: Vec<String>,
    pub limitations: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::RunState;

    #[test]
    fn lifecycle_rejects_retry_from_unknown_outcome() {
        assert!(!RunState::OutcomeUnknown.may_transition_to(RunState::Queued));
        assert!(RunState::OutcomeUnknown.may_transition_to(RunState::Succeeded));
    }

    #[test]
    fn terminal_states_do_not_transition() {
        assert!(RunState::Succeeded.is_terminal());
        assert!(!RunState::Succeeded.may_transition_to(RunState::Running));
    }
}
