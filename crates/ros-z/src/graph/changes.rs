use std::sync::{Arc, Weak};

use parking_lot::Mutex;

type GraphChangeCallback = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone, Default)]
pub(crate) struct GraphChangeCallbacks {
    inner: Arc<Mutex<Vec<Weak<dyn Fn() + Send + Sync>>>>,
}

#[must_use = "dropping GraphChangeListener unregisters the graph change callback"]
pub struct GraphChangeListener {
    _callback: GraphChangeCallback,
}

impl GraphChangeCallbacks {
    pub(crate) fn register(&self, callback: GraphChangeCallback) -> GraphChangeListener {
        self.inner.lock().push(Arc::downgrade(&callback));
        GraphChangeListener {
            _callback: callback,
        }
    }

    pub(crate) fn notify(&self) {
        let callbacks = {
            let mut guard = self.inner.lock();
            let callbacks = guard.iter().filter_map(Weak::upgrade).collect::<Vec<_>>();
            guard.retain(|callback| callback.strong_count() > 0);
            callbacks
        };

        for callback in callbacks {
            callback();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn notify_invokes_registered_callback() {
        let callbacks = GraphChangeCallbacks::default();
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();
        let _listener = callbacks.register(Arc::new(move || {
            count_clone.fetch_add(1, Ordering::Relaxed);
        }));

        callbacks.notify();

        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn dropped_listener_stops_receiving_notifications() {
        let callbacks = GraphChangeCallbacks::default();
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();
        let listener = callbacks.register(Arc::new(move || {
            count_clone.fetch_add(1, Ordering::Relaxed);
        }));
        drop(listener);

        callbacks.notify();

        assert_eq!(count.load(Ordering::Relaxed), 0);
    }
}
