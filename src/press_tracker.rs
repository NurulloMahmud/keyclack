use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Keys held longer than this are assumed to have had their key-up event lost.
pub const STALE_AFTER: Duration = Duration::from_secs(5);

/// Suppresses OS key-repeat so a held key sounds exactly once.
pub struct PressTracker {
    held: HashMap<u16, Instant>,
}

impl PressTracker {
    /// Creates an empty tracker with no keys held.
    pub fn new() -> Self {
        Self {
            held: HashMap::new(),
        }
    }

    /// Returns true if a sound should play (the key was not already held).
    /// Sweeps stale entries before deciding.
    pub fn on_down(&mut self, code: u16) -> bool {
        let now = Instant::now();
        self.held.retain(|_, t| now.duration_since(*t) < STALE_AFTER);
        if self.held.contains_key(&code) {
            return false;
        }
        self.held.insert(code, now);
        true
    }

    /// Marks the key released. A code that was not held is ignored silently.
    pub fn on_up(&mut self, code: u16) {
        self.held.remove(&code);
    }

    /// Number of currently held keys. Test helper.
    #[allow(dead_code)]
    pub fn held_count(&self) -> usize {
        self.held.len()
    }

    #[cfg(test)]
    fn insert_at(&mut self, code: u16, at: Instant) {
        self.held.insert(code, at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_press_sounds() {
        let mut t = PressTracker::new();
        assert_eq!(t.on_down(10), true);
    }

    #[test]
    fn repeat_press_silent() {
        let mut t = PressTracker::new();
        assert_eq!(t.on_down(10), true);
        assert_eq!(t.on_down(10), false);
    }

    #[test]
    fn press_after_release_sounds() {
        let mut t = PressTracker::new();
        assert_eq!(t.on_down(10), true);
        t.on_up(10);
        assert_eq!(t.on_down(10), true);
    }

    #[test]
    fn distinct_keys_independent() {
        let mut t = PressTracker::new();
        assert_eq!(t.on_down(10), true);
        assert_eq!(t.on_down(11), true);
    }

    #[test]
    fn unmatched_up_is_ignored() {
        let mut t = PressTracker::new();
        t.on_up(99);
        assert_eq!(t.held_count(), 0);
    }

    #[test]
    fn stale_key_is_swept() {
        let mut t = PressTracker::new();
        let past = Instant::now() - Duration::from_secs(6);
        t.insert_at(10, past);
        assert_eq!(t.on_down(10), true);
    }
}
