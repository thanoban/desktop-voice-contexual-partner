use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::sync::Notify;

#[derive(Debug, Default)]
pub struct CancellationToken {
    cancelled: AtomicBool,
    notify: Notify,
}

impl CancellationToken {
    pub fn cancel(&self) {
        if !self.cancelled.swap(true, Ordering::AcqRel) {
            self.notify.notify_waiters();
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub async fn cancelled(&self) {
        if self.is_cancelled() {
            return;
        }
        self.notify.notified().await;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelRequestStatus {
    Requested,
    NoActiveTurn,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConversationSnapshot {
    pub active_turn_id: Option<String>,
    pub cancellation_requested: bool,
}

#[derive(Default)]
struct ControllerState {
    active: Option<ActiveTurn>,
}

struct ActiveTurn {
    id: String,
    cancellation: Arc<CancellationToken>,
}

#[derive(Default)]
pub struct ConversationController {
    state: Mutex<ControllerState>,
}

impl ConversationController {
    pub fn while_idle<T>(
        &self,
        operation: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "Conversation controller lock failed".to_string())?;
        if state.active.is_some() {
            return Err(
                "Wait for the active conversation to finish before switching projects".into(),
            );
        }
        let result = operation();
        drop(state);
        result
    }

    pub fn begin(self: &Arc<Self>) -> Result<TurnLease, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Conversation controller lock failed".to_string())?;
        if state.active.is_some() {
            return Err("Another conversation turn is already active".into());
        }

        let id = uuid::Uuid::new_v4().to_string();
        let cancellation = Arc::new(CancellationToken::default());
        state.active = Some(ActiveTurn {
            id: id.clone(),
            cancellation: Arc::clone(&cancellation),
        });
        Ok(TurnLease {
            controller: Arc::clone(self),
            id,
            cancellation,
        })
    }

    pub fn cancel_active(&self) -> CancelRequestStatus {
        let Ok(state) = self.state.lock() else {
            return CancelRequestStatus::NoActiveTurn;
        };
        let Some(active) = &state.active else {
            return CancelRequestStatus::NoActiveTurn;
        };
        active.cancellation.cancel();
        CancelRequestStatus::Requested
    }

    pub fn snapshot(&self) -> ConversationSnapshot {
        let Ok(state) = self.state.lock() else {
            return ConversationSnapshot {
                active_turn_id: None,
                cancellation_requested: false,
            };
        };
        ConversationSnapshot {
            active_turn_id: state.active.as_ref().map(|turn| turn.id.clone()),
            cancellation_requested: state
                .active
                .as_ref()
                .is_some_and(|turn| turn.cancellation.is_cancelled()),
        }
    }

    fn finish(&self, id: &str) {
        if let Ok(mut state) = self.state.lock() {
            if state.active.as_ref().is_some_and(|turn| turn.id == id) {
                state.active = None;
            }
        }
    }
}

pub struct TurnLease {
    controller: Arc<ConversationController>,
    id: String,
    cancellation: Arc<CancellationToken>,
}

impl TurnLease {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn cancellation(&self) -> Arc<CancellationToken> {
        Arc::clone(&self.cancellation)
    }
}

impl Drop for TurnLease {
    fn drop(&mut self) {
        self.controller.finish(&self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::{CancelRequestStatus, ConversationController};
    use std::sync::Arc;

    #[test]
    fn only_one_turn_can_be_active() {
        let controller = Arc::new(ConversationController::default());
        let lease = controller.begin().expect("first turn");
        assert!(controller.begin().is_err());
        drop(lease);
        assert!(controller.begin().is_ok());
    }

    #[test]
    fn cancellation_is_visible_in_snapshot() {
        let controller = Arc::new(ConversationController::default());
        let _lease = controller.begin().expect("turn");
        assert_eq!(controller.cancel_active(), CancelRequestStatus::Requested);
        assert!(controller.snapshot().cancellation_requested);
    }

    #[test]
    fn cancellation_without_turn_is_truthful() {
        let controller = ConversationController::default();
        assert_eq!(
            controller.cancel_active(),
            CancelRequestStatus::NoActiveTurn
        );
    }

    #[test]
    fn project_switch_operation_cannot_run_during_active_turn() {
        let controller = Arc::new(ConversationController::default());
        let lease = controller.begin().unwrap();
        let mut called = false;
        assert!(controller
            .while_idle(|| {
                called = true;
                Ok(())
            })
            .is_err());
        assert!(!called);
        drop(lease);
        assert!(controller.while_idle(|| Ok(())).is_ok());
    }
}
