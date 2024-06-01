//! A single-producer, multiple-consumer channel that serves the last sent value to the consumers.
//!
//! Buffered watch channels enable communication between one sender and multiple receivers.
//! Receivers see the most recent value written by the sender. Utilizing multiple buffers prevents
//! blocking during read or write operations for both sender and receivers, without need for
//! memory allocations.
//!
//! # Usage
//!
//! [`channel`] returns a [`Sender`] / [`Receiver`] pair. These are the producer
//! and consumer halves of the channel. The channel is created with an initial
//! value, which is cloned to
//!
//! # Cloning a receiver
//!
//! Cloning a receiver triggers the allocation of a new internal buffer, ensuring continuous
//! non-blocking operation. However, cloning requires an exclusive lock on the shared state,
//! potentially leading to blocking for the sender or other receivers.
//!
//! Potential Deadlock:
//! ```no_run
//! let (mut sender, receiver) = buffered_watch::channel(0);
//! let guard = sender.borrow_mut();
//! let receiver2 = receiver.clone();
//! ```
//!
//! Receivers can await changes in the channel, i.e., new values written by the sender. Upon
//! reading from the channel, a receiver marks the buffer as seen, preventing immediate return upon
//! subsequent waits for change.
//! Reading after having waited for a change may still drop an intermediate value if the sender
//! writes to the channel between waiting and reading from the channel.
//!
//! # Example:
//!
//! ```
//! use buffered_watch::channel;
//! use std::thread;
//! use std::sync::{Arc, Mutex};
//! use tokio::time::{Duration, sleep};
//!
//! # #[tokio::main]
//! # async fn main() {
//! let (mut sender, mut receiver) = buffered_watch::channel(0);
//!
//! let sender_task = tokio::spawn(async move {
//!     for i in 0..5 {
//!         // simulate some processing time
//!         sleep(Duration::from_millis(100)).await;
//!
//!         let mut slot = sender.borrow_mut();
//!         // write to the buffer
//!         *slot = i;
//!         // drop the buffer lock
//!     }
//! });
//!
//! let receiver_task = tokio::spawn(async move {
//!     loop {
//!         if let Err(_) = receiver.wait_for_change().await {
//!             break;
//!         }
//!         let new_value = receiver.borrow_and_mark_as_seen();
//!         println!("received new value: {}", *new_value);
//!     }
//! });
//!
//! sender_task.await;
//! receiver_task.await;
//! # }
//! ```

use std::{
    cell::UnsafeCell,
    error::Error,
    fmt::{self, Display, Formatter},
    sync::Arc,
};

use parking_lot::{Mutex, RwLock};
pub use receiver::{Receiver, ReceiverGuard};
pub use sender::{Sender, SenderGuard};
use tokio::sync::watch;

mod receiver;
mod sender;

/// Error produced when waiting for a change and the sender has been dropped.
#[derive(Debug, Clone)]
pub struct NoSender;

impl Display for NoSender {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "no writer is present")
    }
}

impl Error for NoSender {}

struct Shared<T> {
    buffers: Vec<UnsafeCell<T>>,
    states: Mutex<Vec<State>>,
}

#[derive(Clone, Copy)]
enum State {
    Free {
        age: usize,
    },
    LockedForWriting,
    LockedForReading {
        age: usize,
        number_of_readers: usize,
    },
}

fn find_oldest_free_buffer(states: &[State]) -> (usize, usize) {
    states
        .iter()
        .enumerate()
        .filter_map(|(index, state)| match state {
            State::Free { age } => Some((index, *age)),
            _ => None,
        })
        .max_by_key(|(_, age)| *age)
        .unwrap()
}

fn find_newest_readable_buffer(states: &[State]) -> usize {
    states
        .iter()
        .enumerate()
        .filter_map(|(index, state)| match state {
            State::Free { age } => Some((index, age)),
            State::LockedForWriting => None,
            State::LockedForReading { age, .. } => Some((index, age)),
        })
        .min_by_key(|(_, age)| *age)
        .map(|(index, _)| index)
        .expect("at least one readable buffer")
}

/// Creates a new buffered watch channel, returning the sender and receiver.
///
/// The initial value is cloned to each buffer.
pub fn channel<T>(initial: T) -> (Sender<T>, Receiver<T>)
where
    T: Clone,
{
    let number_of_buffers = 3;

    let buffers: Vec<_> = (0..number_of_buffers)
        .map(|_| UnsafeCell::new(initial.clone()))
        .collect();
    let states = Mutex::new(vec![State::Free { age: 0 }; number_of_buffers]);

    let shared = Arc::new(RwLock::new(Shared { buffers, states }));
    let (notifier_sender, notifier_receiver) = watch::channel(());

    let sender = Sender {
        shared: shared.clone(),
        notifier: notifier_sender,
    };
    let receiver = Receiver {
        shared,
        notifier: notifier_receiver,
    };
    (sender, receiver)
}
