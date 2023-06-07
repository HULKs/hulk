use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use parking_lot::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Clone, Copy)]
enum State {
    Free { age: usize },
    Writeable { age: usize },
    Readable { age: usize, amount: usize },
}

impl State {
    fn get_age(&self) -> usize {
        match self {
            State::Free { age } => *age,
            State::Writeable { age } => *age,
            State::Readable { age, amount: _ } => *age,
        }
    }
}

pub struct Writer<Slot> {
    slots: Arc<Vec<RwLock<Slot>>>,
    states: Arc<Mutex<Vec<State>>>,
}

pub struct WriterGuard<'locked, Slot> {
    states: &'locked Arc<Mutex<Vec<State>>>,
    slot_index: usize,
    slot: RwLockWriteGuard<'locked, Slot>,
}

impl<'locked, Slot> Deref for WriterGuard<'locked, Slot> {
    type Target = Slot;

    fn deref(&self) -> &Self::Target {
        self.slot.deref()
    }
}

impl<'locked, Slot> DerefMut for WriterGuard<'locked, Slot> {
    fn deref_mut(&mut self) -> &mut Slot {
        self.slot.deref_mut()
    }
}

impl<'locked, Slot> Drop for WriterGuard<'locked, Slot> {
    fn drop(&mut self) {
        let mut states = self.states.lock();
        for (state_index, state) in states.iter_mut().enumerate() {
            if state_index == self.slot_index {
                assert!(matches!(state, State::Writeable { age: _ }));
                *state = State::Free { age: 0 };
            } else {
                match state {
                    State::Free { age } => {
                        *age += 1;
                    }
                    State::Readable { age, amount: _ } => {
                        *age += 1;
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}

impl<Slot> Writer<Slot> {
    pub fn next(&self) -> WriterGuard<Slot> {
        let index = {
            let mut states = self.states.lock();
            let index_with_maximum_age = states
                .iter()
                .enumerate()
                .filter(|(_index, state)| matches!(state, State::Free { age: _ }))
                .max_by_key(|(_index, state)| state.get_age())
                .unwrap()
                .0;
            states[index_with_maximum_age] = State::Writeable {
                age: states[index_with_maximum_age].get_age(),
            };
            index_with_maximum_age
        };

        WriterGuard::<Slot> {
            states: &self.states,
            slot_index: index,
            slot: self.slots[index].write(),
        }
    }
}

pub struct Reader<T> {
    slots: Arc<Vec<RwLock<T>>>,
    states: Arc<Mutex<Vec<State>>>,
}

impl<T> Clone for Reader<T> {
    fn clone(&self) -> Self {
        Self {
            slots: self.slots.clone(),
            states: self.states.clone(),
        }
    }
}

pub struct ReaderGuard<'locked, Slot> {
    states: &'locked Arc<Mutex<Vec<State>>>,
    slot_index: usize,
    slot: RwLockReadGuard<'locked, Slot>,
}

impl<'locked, Slot> Deref for ReaderGuard<'locked, Slot> {
    type Target = Slot;

    fn deref(&self) -> &Self::Target {
        self.slot.deref()
    }
}

impl<'locked, Slot> Drop for ReaderGuard<'locked, Slot> {
    fn drop(&mut self) {
        let mut states = self.states.lock();
        let state = &mut states[self.slot_index];
        match state {
            State::Readable { age, amount } => {
                *amount -= 1;
                if *amount == 0 {
                    *state = State::Free { age: *age };
                }
            }
            _ => unreachable!(),
        }
    }
}

impl<Slot> Reader<Slot> {
    pub fn next(&self) -> ReaderGuard<Slot> {
        let index = {
            let mut states = self.states.lock();
            let index_with_minimum_age = states
                .iter()
                .enumerate()
                .filter(|(_index, state)| {
                    matches!(state, State::Free { age: _ })
                        || matches!(state, State::Readable { age: _, amount: _ })
                })
                .min_by_key(|(_index, state)| state.get_age())
                .unwrap()
                .0;
            match states[index_with_minimum_age] {
                State::Free { age } => {
                    states[index_with_minimum_age] = State::Readable { age, amount: 1 };
                }
                State::Readable { age, amount } => {
                    states[index_with_minimum_age] = State::Readable {
                        age,
                        amount: amount + 1,
                    };
                }
                _ => unreachable!(),
            }
            index_with_minimum_age
        };

        ReaderGuard::<Slot> {
            states: &self.states,
            slot_index: index,
            slot: self.slots[index].read(),
        }
    }
}

pub fn multiple_buffer_with_slots<Slots>(slots: Slots) -> (Writer<Slots::Item>, Reader<Slots::Item>)
where
    Slots: IntoIterator,
{
    let slots: Arc<Vec<RwLock<Slots::Item>>> =
        Arc::new(slots.into_iter().map(RwLock::new).collect());
    let states = Arc::new(Mutex::new(vec![State::Free { age: 0 }; slots.len()]));
    let reader_slots = slots.clone();
    let reader_states = states.clone();
    (
        Writer::<Slots::Item> { slots, states },
        Reader::<Slots::Item> {
            slots: reader_slots,
            states: reader_states,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_slots_sequential_reader_writer() {
        let (writer, reader) = multiple_buffer_with_slots([0, 1, 2]);
        {
            let mut slot = writer.next();
            *slot = 42;
        }
        {
            let slot = reader.next();
            assert_eq!(*slot, 42);
        }
        {
            #[allow(clippy::redundant_clone)]
            let reader2 = reader.clone();
            let slot = reader2.next();
            assert_eq!(*slot, 42);
        }
        {
            let mut slot = writer.next();
            *slot = 1337;
        }
        {
            #[allow(clippy::redundant_clone)]
            let reader3 = reader.clone();
            let slot = reader3.next();
            assert_eq!(*slot, 1337);
        }
        {
            let reader4 = reader;
            let slot = reader4.next();
            assert_eq!(*slot, 1337);
        }
    }

    #[test]
    fn writer_overwrites_non_reader_slots() {
        let (writer, reader) = multiple_buffer_with_slots([0, 1, 2]);
        {
            let mut slot = writer.next();
            *slot = 42;
        }
        let reader_slot = reader.next();
        assert_eq!(*reader_slot, 42);
        {
            let mut slot = writer.next();
            *slot = 1337;
        }
        assert_eq!(*reader_slot, 42);
        {
            let mut slot = writer.next();
            *slot = 1337;
        }
        assert_eq!(*reader_slot, 42);
    }

    #[test]
    fn readers_share_same_slot() {
        let (_writer, reader) = multiple_buffer_with_slots([0, 1, 2]);
        #[allow(clippy::redundant_clone)]
        let reader2 = reader.clone();
        let reader_slot = reader.next();
        let reader2_slot = reader2.next();
        assert_eq!(*reader_slot, *reader2_slot);
    }
}
