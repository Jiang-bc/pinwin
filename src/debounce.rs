//! Deterministic debounce and throttle state machine.
//!
//! Ported and modernized from `PinWin/BusinessLayer/DebounceDispatcher.cs`.

use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct DebounceTracker {
    delay: Duration,
    last_event: Option<Instant>,
    pending: bool,
}

impl DebounceTracker {
    pub const fn new(delay: Duration) -> Self {
        Self {
            delay,
            last_event: None,
            pending: false,
        }
    }

    pub fn set_delay(&mut self, delay: Duration) {
        self.delay = delay;
    }

    /// Record a new event at `now`, resetting the debounce timer.
    pub fn trigger(&mut self, now: Instant) {
        self.last_event = Some(now);
        self.pending = true;
    }

    /// Check whether the debounce interval has elapsed since the last trigger.
    pub fn poll_ready(&mut self, now: Instant) -> bool {
        if !self.pending {
            return false;
        }
        if let Some(last) = self.last_event
            && now.saturating_duration_since(last) >= self.delay
        {
            self.pending = false;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debounce_fires_only_after_quiet_period() {
        let start = Instant::now();
        let mut d = DebounceTracker::new(Duration::from_millis(500));
        assert!(!d.poll_ready(start));

        d.trigger(start);
        assert!(!d.poll_ready(start + Duration::from_millis(200)));

        // Re-trigger at 300ms resets timer
        d.trigger(start + Duration::from_millis(300));
        assert!(!d.poll_ready(start + Duration::from_millis(600)));
        assert!(d.poll_ready(start + Duration::from_millis(800)));
        // Subsequent poll without new trigger returns false
        assert!(!d.poll_ready(start + Duration::from_millis(900)));
    }
}
