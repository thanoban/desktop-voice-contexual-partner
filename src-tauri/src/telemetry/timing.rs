use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageTiming {
    pub stage: String,
    pub duration_ms: u64,
}

pub struct StageTimer {
    stage: String,
    started: Instant,
}

impl StageTimer {
    pub fn start(stage: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            started: Instant::now(),
        }
    }

    pub fn finish(self) -> StageTiming {
        StageTiming {
            stage: self.stage,
            duration_ms: duration_ms(self.started.elapsed()),
        }
    }
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::{duration_ms, StageTimer};
    use std::time::Duration;

    #[test]
    fn duration_conversion_is_milliseconds() {
        assert_eq!(duration_ms(Duration::from_micros(1_999)), 1);
    }

    #[test]
    fn timer_preserves_stage_name() {
        let result = StageTimer::start("generation").finish();
        assert_eq!(result.stage, "generation");
    }
}
