use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::Emitter;
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
        // Register before reading the flag so notify_waiters cannot slip between
        // the check and the first poll of this future.
        let notified = self.notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if !self.is_cancelled() {
            notified.await;
        }
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
    pub audio: AudioSnapshot,
    pub revision: u64,
    pub project_id: String,
    pub session_id: String,
    pub messages: Vec<TranscriptMessage>,
    pub streaming_content: String,
    pub generating: bool,
    pub error: Option<String>,
    pub active_turn_id: Option<String>,
    pub cancellation_requested: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AudioSnapshot {
    pub listening: bool,
    pub transcribing: bool,
    pub speaking: bool,
}

#[derive(Clone, Copy)]
pub enum AudioPhase {
    Listening,
    Transcribing,
    Speaking,
}

#[derive(Default)]
struct AudioOwners {
    listening: Option<String>,
    transcribing: Option<String>,
    speaking: Option<String>,
}

impl AudioOwners {
    fn snapshot(&self) -> AudioSnapshot {
        AudioSnapshot {
            listening: self.listening.is_some(),
            transcribing: self.transcribing.is_some(),
            speaking: self.speaking.is_some(),
        }
    }
    fn clear(&mut self, owner: &str) {
        for slot in [
            &mut self.listening,
            &mut self.transcribing,
            &mut self.speaking,
        ] {
            if slot.as_deref() == Some(owner) {
                *slot = None;
            }
        }
    }
    fn set(&mut self, owner: &str, phase: AudioPhase) {
        self.clear(owner);
        *match phase {
            AudioPhase::Listening => &mut self.listening,
            AudioPhase::Transcribing => &mut self.transcribing,
            AudioPhase::Speaking => &mut self.speaking,
        } = Some(owner.into());
    }
    fn busy(&self) -> bool {
        self.listening.is_some() || self.transcribing.is_some() || self.speaking.is_some()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscriptMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
}

#[derive(Default)]
struct ControllerState {
    audio: AudioOwners,
    active: Option<ActiveTurn>,
    revision: u64,
    project_id: String,
    session_id: String,
    messages: Vec<TranscriptMessage>,
    streaming_content: String,
    generating: bool,
    error: Option<String>,
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
    pub fn begin_audio(
        self: &Arc<Self>,
        phase: AudioPhase,
        load_scope: impl FnOnce() -> Result<(String, String, Vec<TranscriptMessage>), String>,
    ) -> Result<AudioLease, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Conversation lock failed".to_string())?;
        if state.audio.busy() {
            return Err("Another audio operation is already active".into());
        }
        // Same lock ordering as project selection: controller before database.
        let (project_id, session_id, messages) = load_scope()?;
        if state.project_id != project_id || state.session_id != session_id {
            state.project_id = project_id;
            state.session_id = session_id;
            state.messages = messages;
            state.streaming_content.clear();
            state.error = None;
        }
        let owner = uuid::Uuid::new_v4().to_string();
        state.audio.set(&owner, phase);
        state.revision += 1;
        Ok(AudioLease {
            controller: Arc::clone(self),
            owner,
            event_app: None,
        })
    }

    pub fn hydrate(
        &self,
        project_id: &str,
        session_id: &str,
        messages: Vec<TranscriptMessage>,
        generating: bool,
    ) {
        if let Ok(mut state) = self.state.lock() {
            state.revision += 1;
            state.project_id = project_id.into();
            state.session_id = session_id.into();
            state.messages = messages;
            state.streaming_content.clear();
            state.generating = generating;
            state.error = None;
        }
    }

    pub fn append(&self, turn_id: &str, content: &str) {
        if let Ok(mut state) = self.state.lock() {
            if state.active.as_ref().is_some_and(|turn| turn.id == turn_id) && state.generating {
                state.streaming_content.push_str(content);
                state.revision += 1;
            }
        }
    }

    pub fn fail(&self, error: String) {
        if let Ok(mut state) = self.state.lock() {
            state.error = Some(error);
            state.generating = false;
            state.revision += 1;
        }
    }
    pub fn while_idle<T>(
        &self,
        operation: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "Conversation controller lock failed".to_string())?;
        if state.active.is_some() || state.audio.busy() {
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
        if state.active.is_some() || state.audio.busy() {
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
            event_app: None,
            audio: None,
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
                revision: 0,
                audio: AudioSnapshot::default(),
                project_id: String::new(),
                session_id: String::new(),
                messages: vec![],
                streaming_content: String::new(),
                generating: false,
                error: None,
                active_turn_id: None,
                cancellation_requested: false,
            };
        };
        ConversationSnapshot {
            audio: state.audio.snapshot(),
            revision: state.revision,
            project_id: state.project_id.clone(),
            session_id: state.session_id.clone(),
            messages: state.messages.clone(),
            streaming_content: state.streaming_content.clone(),
            generating: state.generating,
            error: state.error.clone(),
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
                state.generating = false;
                state.revision += 1;
            }
        }
    }
}

pub struct TurnLease {
    controller: Arc<ConversationController>,
    id: String,
    cancellation: Arc<CancellationToken>,
    event_app: Option<tauri::AppHandle>,
    audio: Option<AudioLease>,
}

impl TurnLease {
    pub fn with_audio(mut self, audio: AudioLease) -> Self {
        self.audio = Some(audio);
        self
    }
    pub fn set_audio_phase(&self, phase: AudioPhase) {
        if let Some(audio) = &self.audio {
            audio.set_phase(phase);
        }
    }
    /// Publish terminal state even when a command exits early through `?`.
    pub fn with_events(mut self, app: tauri::AppHandle) -> Self {
        self.event_app = Some(app);
        self
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn cancellation(&self) -> Arc<CancellationToken> {
        Arc::clone(&self.cancellation)
    }
}

pub struct AudioLease {
    controller: Arc<ConversationController>,
    owner: String,
    event_app: Option<tauri::AppHandle>,
}

impl AudioLease {
    pub fn with_events(mut self, app: tauri::AppHandle) -> Self {
        self.event_app = Some(app);
        self.publish();
        self
    }
    fn publish(&self) {
        if let Some(app) = &self.event_app {
            let _ = app.emit("conversation:snapshot", self.controller.snapshot());
        }
    }
    pub fn set_phase(&self, phase: AudioPhase) {
        if let Ok(mut state) = self.controller.state.lock() {
            let owns_audio = [
                &state.audio.listening,
                &state.audio.transcribing,
                &state.audio.speaking,
            ]
            .into_iter()
            .any(|slot| slot.as_deref() == Some(&self.owner));
            if owns_audio {
                state.audio.set(&self.owner, phase);
                state.revision += 1;
            }
        }
        self.publish();
    }
}

impl Drop for AudioLease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.controller.state.lock() {
            state.audio.clear(&self.owner);
            state.revision += 1;
        }
        self.publish();
    }
}

impl Drop for TurnLease {
    fn drop(&mut self) {
        self.controller.finish(&self.id);
        if let Some(app) = &self.event_app {
            let _ = app.emit("conversation:snapshot", self.controller.snapshot());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CancelRequestStatus, ConversationController};
    use std::sync::Arc;

    #[tokio::test]
    async fn cancellation_releases_all_waiters_and_late_subscribers() {
        let token = Arc::new(super::CancellationToken::default());
        let mut waiters = Vec::new();
        for _ in 0..8 {
            let token = token.clone();
            waiters.push(tokio::spawn(async move { token.cancelled().await }));
        }
        tokio::task::yield_now().await;
        token.cancel();
        for waiter in waiters {
            tokio::time::timeout(std::time::Duration::from_secs(1), waiter)
                .await
                .unwrap()
                .unwrap();
        }
        tokio::time::timeout(std::time::Duration::from_secs(1), token.cancelled())
            .await
            .unwrap();
    }

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

    #[test]
    fn snapshots_preserve_shared_transcript_and_reject_stale_tokens() {
        let controller = Arc::new(ConversationController::default());
        let first = controller.begin().unwrap();
        controller.hydrate(
            "project-a",
            "session-a",
            vec![super::TranscriptMessage {
                id: "user-1".into(),
                role: "user".into(),
                content: "Hello".into(),
                timestamp: 1,
            }],
            true,
        );
        controller.append(first.id(), "Hi");
        let streaming = controller.snapshot();
        assert_eq!(streaming.messages.len(), 1);
        assert_eq!(streaming.streaming_content, "Hi");
        let old_id = first.id().to_string();
        drop(first);
        let second = controller.begin().unwrap();
        controller.hydrate("project-b", "session-b", vec![], true);
        controller.append(&old_id, "private old text");
        assert!(controller.snapshot().streaming_content.is_empty());
        controller.append(second.id(), "New");
        let next = controller.snapshot();
        assert!(next.revision > streaming.revision);
        assert_eq!(next.project_id, "project-b");
        assert_eq!(next.streaming_content, "New");
    }

    #[test]
    fn failure_stops_generation_and_preserves_partial_output() {
        let controller = Arc::new(ConversationController::default());
        let turn = controller.begin().unwrap();
        controller.hydrate("personal", "session", vec![], true);
        controller.append(turn.id(), "Partial");
        controller.fail("Connection lost".into());
        let failed = controller.snapshot();
        assert!(!failed.generating);
        assert_eq!(failed.streaming_content, "Partial");
        assert_eq!(failed.error.as_deref(), Some("Connection lost"));
        controller.append(turn.id(), "late");
        assert_eq!(controller.snapshot().revision, failed.revision);
    }

    #[test]
    fn early_turn_exit_publishes_idle_generation_state() {
        let controller = Arc::new(ConversationController::default());
        let turn = controller.begin().unwrap();
        controller.hydrate("personal", "session", vec![], true);
        let revision = controller.snapshot().revision;
        drop(turn);
        let idle = controller.snapshot();
        assert!(!idle.generating);
        assert!(idle.active_turn_id.is_none());
        assert!(idle.revision > revision);
    }

    fn audio_scope() -> Result<(String, String, Vec<super::TranscriptMessage>), String> {
        Ok(("client-a".into(), "session-a".into(), vec![]))
    }

    #[test]
    fn capture_hydrates_identity_and_transitions_to_transcription_then_cleans_up() {
        let controller = Arc::new(ConversationController::default());
        let turn = controller.begin().unwrap();
        let audio = controller
            .begin_audio(super::AudioPhase::Listening, audio_scope)
            .unwrap();
        let turn = turn.with_audio(audio);
        let listening = controller.snapshot();
        assert_eq!(listening.project_id, "client-a");
        assert_eq!(listening.session_id, "session-a");
        assert!(listening.audio.listening);
        turn.set_audio_phase(super::AudioPhase::Transcribing);
        let transcribing = controller.snapshot();
        assert!(!transcribing.audio.listening);
        assert!(transcribing.audio.transcribing);
        assert!(transcribing.revision > listening.revision);
        drop(turn); // Mirrors errors, timeout, and normal stop completion.
        assert_eq!(controller.snapshot().audio, super::AudioSnapshot::default());
        assert!(controller.begin().is_ok());
    }

    #[test]
    fn speech_audition_blocks_switching_and_drop_cleans_early_synthesis_failure() {
        let controller = Arc::new(ConversationController::default());
        let audio = controller
            .begin_audio(super::AudioPhase::Speaking, audio_scope)
            .unwrap();
        assert!(controller.snapshot().audio.speaking);
        assert!(controller.while_idle(|| Ok(())).is_err());
        assert!(controller.begin().is_err());
        assert!(controller
            .begin_audio(super::AudioPhase::Speaking, audio_scope)
            .is_err());
        drop(audio);
        assert!(!controller.snapshot().audio.speaking);
        assert!(controller.while_idle(|| Ok(())).is_ok());
    }

    #[test]
    fn stale_audio_owner_cannot_clear_or_transition_newer_work() {
        let controller = Arc::new(ConversationController::default());
        let old = controller
            .begin_audio(super::AudioPhase::Speaking, audio_scope)
            .unwrap();
        controller.state.lock().unwrap().audio.clear(&old.owner);
        let current = controller
            .begin_audio(super::AudioPhase::Listening, audio_scope)
            .unwrap();
        old.set_phase(super::AudioPhase::Transcribing);
        drop(old);
        assert!(controller.snapshot().audio.listening);
        assert!(!controller.snapshot().audio.transcribing);
        drop(current);
        assert_eq!(controller.snapshot().audio, super::AudioSnapshot::default());
    }

    #[test]
    fn failed_scope_admission_does_not_publish_busy_audio() {
        let controller = Arc::new(ConversationController::default());
        assert!(controller
            .begin_audio(super::AudioPhase::Listening, || Err(
                "Database unavailable".into()
            ))
            .is_err());
        assert_eq!(controller.snapshot().audio, super::AudioSnapshot::default());
    }
}
