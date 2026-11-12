use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
use zeroize::Zeroize;
#[derive(Default)]
pub struct CommandQueue {
    pending: VecDeque<String>,
    sent: Option<Instant>,
}
impl CommandQueue {
    pub fn push(&mut self, cmd: String) {
        self.pending.push_back(cmd);
    }
    pub fn next_to_send(&mut self, now: Instant) -> Option<String> {
        if self
            .sent
            .is_some_and(|t| now.saturating_duration_since(t) < Duration::from_millis(400))
        {
            return None;
        }
        let next = self.pending.pop_front()?;
        self.sent = Some(now);
        Some(next)
    }
    pub fn on_reply(&mut self) {
        self.sent = None;
    }
}
impl Drop for CommandQueue {
    fn drop(&mut self) {
        for s in &mut self.pending {
            s.zeroize();
        }
    }
}
