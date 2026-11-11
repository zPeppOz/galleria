use galleria_ipc::{LogEntry, LogLevel, LogSource};
use std::{
    collections::VecDeque,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct LogBuffer {
    capacity: usize,
    seq: u64,
    entries: VecDeque<LogEntry>,
}
impl LogBuffer {
    pub fn with_capacity(lines: usize) -> Self {
        Self {
            capacity: lines,
            seq: 0,
            entries: VecDeque::with_capacity(lines),
        }
    }
    pub fn push(&mut self, level: LogLevel, source: LogSource, message: String) -> LogEntry {
        self.seq += 1;
        let entry = LogEntry {
            seq: self.seq,
            ts: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .to_string(),
            level,
            source,
            message,
        };
        if self.capacity > 0 {
            if self.entries.len() == self.capacity {
                self.entries.pop_front();
            }
            self.entries.push_back(entry.clone());
        }
        entry
    }
    pub fn after(&self, seq: Option<u64>, limit: usize) -> (Vec<LogEntry>, u64) {
        let entries: Vec<_> = self
            .entries
            .iter()
            .filter(|e| seq.is_none_or(|s| e.seq > s))
            .take(limit.min(1000))
            .cloned()
            .collect();
        let last = entries
            .last()
            .map(|e| e.seq)
            .unwrap_or(seq.unwrap_or(self.seq));
        (entries, last)
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
