//! Cancellable ownership of TTS synthesis and playback subprocesses.
//!
//! Every subprocess that produces or plays speech is run through [`run_tracked`]. A global
//! epoch counter marks cancellation: [`stop`] bumps the epoch and kills the live child, so
//! work that started earlier never spawns another stage or reports success after a stop.

use anyhow::{anyhow, Result};
use serde::Serialize;
use std::io::Write;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopOutcome {
    /// A speech subprocess was running and has been killed.
    Stopped,
    /// Nothing was synthesizing or playing.
    Idle,
}

#[derive(Debug)]
pub enum RunOutcome {
    Completed(ExitStatus),
    Cancelled,
}

#[derive(Default)]
pub struct Playback {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    epoch: u64,
    child: Option<Arc<Mutex<Child>>>,
}

impl Playback {
    pub fn epoch(&self) -> u64 {
        self.state.lock().unwrap().epoch
    }

    pub fn is_cancelled(&self, epoch: u64) -> bool {
        self.epoch() != epoch
    }

    /// Cancels all speech work started before this call and kills the running subprocess.
    pub fn stop(&self) -> StopOutcome {
        let child = {
            let mut state = self.state.lock().unwrap();
            state.epoch += 1;
            state.child.take()
        };
        match child {
            Some(child) => {
                let mut child = child.lock().unwrap();
                let _ = child.kill();
                let _ = child.wait();
                StopOutcome::Stopped
            }
            None => StopOutcome::Idle,
        }
    }

    /// Runs `command` to completion unless `epoch` is cancelled first. Optional `stdin` is
    /// written then closed so the child sees EOF.
    pub fn run_tracked(
        &self,
        mut command: Command,
        stdin: Option<&[u8]>,
        epoch: u64,
    ) -> Result<RunOutcome> {
        let shared = {
            let mut state = self.state.lock().unwrap();
            if state.epoch != epoch {
                return Ok(RunOutcome::Cancelled);
            }
            if stdin.is_some() {
                command.stdin(Stdio::piped());
            }
            let mut child = command
                .spawn()
                .map_err(|error| anyhow!("Failed to start speech process: {}", error))?;
            let pipe = child.stdin.take();
            let shared = Arc::new(Mutex::new(child));
            state.child = Some(shared.clone());
            (shared, pipe)
        };
        let (child, pipe) = shared;
        if let (Some(mut pipe), Some(bytes)) = (pipe, stdin) {
            let _ = pipe.write_all(bytes);
        }

        let outcome = loop {
            {
                let mut guard = child.lock().unwrap();
                if let Some(status) = guard.try_wait()? {
                    break Some(status);
                }
            }
            if self.is_cancelled(epoch) {
                break None;
            }
            std::thread::sleep(Duration::from_millis(15));
        };

        let mut state = self.state.lock().unwrap();
        if state
            .child
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, &child))
        {
            state.child = None;
        }
        Ok(match outcome {
            Some(_) if state.epoch != epoch => RunOutcome::Cancelled,
            Some(status) => RunOutcome::Completed(status),
            None => RunOutcome::Cancelled,
        })
    }
}

pub fn global() -> &'static Playback {
    static PLAYBACK: OnceLock<Playback> = OnceLock::new();
    PLAYBACK.get_or_init(Playback::default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn long_running() -> Command {
        #[cfg(target_os = "windows")]
        {
            let mut command = Command::new("ping");
            command.args(["-n", "30", "127.0.0.1"]);
            command.stdout(Stdio::null());
            command
        }
        #[cfg(not(target_os = "windows"))]
        {
            let mut command = Command::new("sleep");
            command.arg("30");
            command
        }
    }

    #[test]
    fn stop_with_nothing_running_reports_idle() {
        assert_eq!(Playback::default().stop(), StopOutcome::Idle);
    }

    #[test]
    fn stop_kills_running_process_and_reports_stopped() {
        let playback = Arc::new(Playback::default());
        let epoch = playback.epoch();
        let worker = {
            let playback = playback.clone();
            std::thread::spawn(move || playback.run_tracked(long_running(), None, epoch))
        };
        std::thread::sleep(Duration::from_millis(300));
        let started = Instant::now();
        assert_eq!(playback.stop(), StopOutcome::Stopped);
        let outcome = worker.join().unwrap().unwrap();
        assert!(matches!(outcome, RunOutcome::Cancelled));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn work_from_a_cancelled_epoch_never_spawns() {
        let playback = Playback::default();
        let epoch = playback.epoch();
        playback.stop();
        let outcome = playback.run_tracked(long_running(), None, epoch).unwrap();
        assert!(matches!(outcome, RunOutcome::Cancelled));
    }

    #[test]
    fn completed_process_reports_status() {
        let playback = Playback::default();
        #[cfg(target_os = "windows")]
        let command = {
            let mut command = Command::new("cmd");
            command.args(["/C", "exit 0"]);
            command
        };
        #[cfg(not(target_os = "windows"))]
        let command = Command::new("true");
        let epoch = playback.epoch();
        match playback.run_tracked(command, None, epoch).unwrap() {
            RunOutcome::Completed(status) => assert!(status.success()),
            RunOutcome::Cancelled => panic!("unexpected cancellation"),
        }
    }
}
