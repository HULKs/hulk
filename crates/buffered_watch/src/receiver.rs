use std::{ops::Deref, sync::Arc};

use parking_lot::{RwLock, RwLockReadGuard};
use tokio::sync::watch;

use crate::{find_newest_readable_buffer, find_oldest_free_buffer, NoSender, Shared, State};

/// Receives values from the associated Sender
pub struct Receiver<T> {
    pub(crate) shared: Arc<RwLock<Shared<T>>>,
    pub(crate) notifier: watch::Receiver<()>,
}

unsafe impl<T> Sync for Receiver<T> where T: Sync {}
unsafe impl<T> Send for Receiver<T> where T: Send + Sync {}

impl<T> Receiver<T> {
    /// Waits for the next change and marks the buffer as seen
    pub async fn wait_for_change(&mut self) -> Result<(), NoSender> {
        self.notifier.changed().await.map_err(|_| NoSender)
    }

    /// Borrows the latest value
    pub fn borrow(&mut self) -> ReceiverGuard<T> {
        let shared = self.shared.read();
        let index = {
            let states = &mut *shared.states.lock();
            lock_a_readable_buffer(states)
        };
        // Safety: access is managed by the `shared.states`, we are allowed to dereference
        let buffer = unsafe { &*shared.buffers[index].get() };

        ReceiverGuard {
            shared,
            buffer_index: index,
            buffer,
        }
    }

    /// Borrows the latest value and marks the buffer as seen
    pub fn borrow_and_mark_as_seen(&mut self) -> ReceiverGuard<T> {
        let shared = self.shared.read();
        let index = {
            let states = &mut *shared.states.lock();
            self.notifier.mark_unchanged();
            lock_a_readable_buffer(states)
        };
        // Safety: access is managed by the `shared.states`, we are allowed to dereference
        let buffer = unsafe { &*shared.buffers[index].get() };

        ReceiverGuard {
            shared,
            buffer_index: index,
            buffer,
        }
    }
}

pub(crate) fn lock_a_readable_buffer(states: &mut [State]) -> usize {
    let index = find_newest_readable_buffer(states);

    match states[index] {
        State::Free { age } => {
            states[index] = State::LockedForReading {
                age,
                number_of_readers: 1,
            };
        }
        State::LockedForReading {
            age,
            number_of_readers,
        } => {
            states[index] = State::LockedForReading {
                age,
                number_of_readers: number_of_readers + 1,
            };
        }
        _ => panic!("a readable buffer is always free or locked for reading"),
    }
    index
}

impl<T> Clone for Receiver<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let shared = &mut *self.shared.write();
        shared.append_buffer();

        Self {
            shared: self.shared.clone(),
            notifier: self.notifier.clone(),
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        let shared = &mut *self.shared.write();
        let mut states = shared.states.lock();

        let (oldest_index, _) = find_oldest_free_buffer(&states);

        shared.buffers.remove(oldest_index);
        states.remove(oldest_index);
    }
}

/// RAII guard for reading from a buffer
pub struct ReceiverGuard<'lock, T> {
    pub(crate) shared: RwLockReadGuard<'lock, Shared<T>>,
    pub(crate) buffer_index: usize,
    pub(crate) buffer: &'lock T,
}

impl<T> Deref for ReceiverGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.buffer
    }
}

impl<T> Drop for ReceiverGuard<'_, T> {
    fn drop(&mut self) {
        let mut states = self.shared.states.lock();
        let state = states
            .get_mut(self.buffer_index)
            .expect("buffer index is always in bounds");
        match state {
            State::LockedForReading {
                age,
                number_of_readers,
            } => {
                *number_of_readers -= 1;
                if *number_of_readers == 0 {
                    *state = State::Free { age: *age };
                }
            }
            _ => panic!("a reader guard is always created for a readable buffer"),
        }
    }
}
