use std::{collections::VecDeque, sync::Arc};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum DebugEvent {
    StatusChanged,
    ValueUpdated,
    GraphChanged,
    NamespaceChanged,
    ConnectionChanged,
    SchemaChanged,
    Diagnostic(String),
}

pub type WakeCallback = Arc<dyn Fn() + Send + Sync>;

pub struct EventBuffer {
    capacity: usize,
    events: VecDeque<DebugEvent>,
}

impl EventBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            events: VecDeque::new(),
        }
    }

    pub fn push(&mut self, event: DebugEvent) {
        if self.capacity == 0 {
            return;
        }

        if self.events.len() == self.capacity {
            self.events.pop_front();
        }

        self.events.push_back(event);
    }

    pub fn drain(&mut self) -> Vec<DebugEvent> {
        self.events.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{DebugEvent, EventBuffer};

    #[test]
    fn drain_returns_and_clears_events() {
        let mut buffer = EventBuffer::new(2);
        buffer.push(DebugEvent::StatusChanged);
        buffer.push(DebugEvent::Diagnostic("schema unavailable".to_string()));

        let drained = buffer.drain();
        assert_eq!(drained.len(), 2);
        assert!(matches!(drained[0], DebugEvent::StatusChanged));
        assert!(
            matches!(&drained[1], DebugEvent::Diagnostic(message) if message == "schema unavailable")
        );
        assert!(buffer.drain().is_empty());
    }

    #[test]
    fn capacity_zero_stores_nothing() {
        let mut buffer = EventBuffer::new(0);

        buffer.push(DebugEvent::ValueUpdated);

        assert!(buffer.drain().is_empty());
    }

    #[test]
    fn push_drops_oldest_event_when_capacity_is_exceeded() {
        let mut buffer = EventBuffer::new(2);
        buffer.push(DebugEvent::StatusChanged);
        buffer.push(DebugEvent::ValueUpdated);
        buffer.push(DebugEvent::GraphChanged);

        let drained = buffer.drain();
        assert_eq!(drained.len(), 2);
        assert!(matches!(drained[0], DebugEvent::ValueUpdated));
        assert!(matches!(drained[1], DebugEvent::GraphChanged));
    }
}
