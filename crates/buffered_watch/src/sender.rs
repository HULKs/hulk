use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use parking_lot::{RwLock, RwLockReadGuard};
use tokio::sync::watch;

use crate::{
    find_oldest_free_buffer, receiver::lock_a_readable_buffer, ReceiverGuard, Shared, State,
};

/// Sends values to the associated Receivers
pub struct Sender<T> {
    pub(crate) shared: Arc<RwLock<Shared<T>>>,
    pub(crate) notifier: watch::Sender<()>,
}

unsafe impl<T> Sync for Sender<T> where T: Sync {}
unsafe impl<T> Send for Sender<T> where T: Send + Sync {}

impl<T> Sender<T> {
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

    /// Borrows a buffer to write to
    pub fn borrow_mut(&mut self) -> SenderGuard<T> {
        let shared = self.shared.read();
        let index = {
            let states = &mut *shared.states.lock();
            lock_a_free_buffer(states)
        };

        // Safety: access is managed by the `shared.states`, we are allowed to dereference mutably
        // as we are the only one with reference to the buffer (`LockedForWriting`).
        let buffer = unsafe { &mut *shared.buffers[index].get() };

        SenderGuard {
            shared,
            notifier: &self.notifier,
            buffer_index: index,
            buffer,
        }
    }
}

fn lock_a_free_buffer(states: &mut [State]) -> usize {
    let (index_with_maximum_age, _) = find_oldest_free_buffer(states);
    states[index_with_maximum_age] = State::LockedForWriting;
    index_with_maximum_age
}

/// RAII guard for writing to a buffer
pub struct SenderGuard<'lock, T> {
    shared: RwLockReadGuard<'lock, Shared<T>>,
    notifier: &'lock watch::Sender<()>,
    buffer_index: usize,
    buffer: &'lock mut T,
}

impl<'lock, T> Deref for SenderGuard<'lock, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.buffer.deref()
    }
}

impl<'lock, T> DerefMut for SenderGuard<'lock, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.buffer.deref_mut()
    }
}

impl<'lock, T> Drop for SenderGuard<'lock, T> {
    fn drop(&mut self) {
        {
            let mut states = self.shared.states.lock();
            for (state_index, state) in states.iter_mut().enumerate() {
                if state_index == self.buffer_index {
                    *state = State::Free { age: 0 };
                } else {
                    match state {
                        State::Free { age } => {
                            *age += 1;
                        }
                        State::LockedForReading {
                            age,
                            number_of_readers: _,
                        } => {
                            *age += 1;
                        }
                        _ => panic!("we are the writer, and we are dropping our buffer guard, there cannot be any other buffer that is locked for writing")
                    }
                }
            }
        }
        let _ = self.notifier.send(());
    }
}
