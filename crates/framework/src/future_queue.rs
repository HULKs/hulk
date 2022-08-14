use std::{sync::Arc, time::SystemTime};

use parking_lot::Mutex;

use crate::Update;

struct Slot<T> {
    timestamp: Option<SystemTime>,
    data: Option<T>,
}

impl<T> Slot<T> {
    fn empty() -> Self {
        Self {
            timestamp: None,
            data: None,
        }
    }
}

pub struct Item<T> {
    pub timestamp: SystemTime,
    pub data: T,
}

impl<T> From<Slot<T>> for Item<T> {
    fn from(slot: Slot<T>) -> Self {
        Self {
            timestamp: slot.timestamp.unwrap(),
            data: slot.data.unwrap(),
        }
    }
}

pub fn future_queue<T>() -> (Producer<T>, Consumer<T>) {
    let slots = Arc::new(Mutex::new(vec![]));
    (
        Producer::<T> {
            slots: slots.clone(),
        },
        Consumer::<T> { slots },
    )
}

pub struct Producer<T> {
    slots: Arc<Mutex<Vec<Slot<T>>>>,
}

impl<T> Producer<T> {
    pub fn announce(&self) {
        let mut slots = self.slots.lock();
        slots.push(Slot::empty());
    }

    pub fn finalize(&self, data: T) {
        let mut slots = self.slots.lock();
        slots.last_mut().unwrap().data = Some(data);
    }
}

pub struct Consumer<T> {
    slots: Arc<Mutex<Vec<Slot<T>>>>,
}

impl<T> Consumer<T> {
    pub fn consume(&self, now: SystemTime) -> Update<T> {
        let mut slots = self.slots.lock();

        for object in slots.iter_mut() {
            if object.timestamp.is_none() {
                object.timestamp = Some(now);
            }
        }

        let first_empty_data = slots
            .iter()
            .position(|object| object.data.is_none())
            .unwrap_or(slots.len());
        let finished = slots.drain(..first_empty_data).map(Item::from).collect();
        let first_timestamp_of_empty_data = slots.first().map(|slot| slot.timestamp.unwrap());

        Update {
            items: finished,
            first_timestamp_of_non_finalized_database: first_timestamp_of_empty_data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn producer_and_consumer_equally_fast() {
        let (producer, consumer) = future_queue();
        assert!(producer.slots.lock().is_empty());

        producer.announce();
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 1);
            assert!(slots[0].timestamp.is_none());
            assert!(slots[0].data.is_none());
        }

        let instant_a = SystemTime::now();
        let Update {
            items: databases,
            first_timestamp_of_non_finalized_database,
        } = consumer.consume(instant_a);
        assert!(databases.is_empty());
        assert_eq!(first_timestamp_of_non_finalized_database, Some(instant_a));
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 1);
            assert_eq!(slots[0].timestamp, Some(instant_a));
            assert!(slots[0].data.is_none());
        }

        producer.finalize(42);
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 1);
            assert_eq!(slots[0].timestamp, Some(instant_a));
            assert_eq!(slots[0].data, Some(42));
        }

        producer.announce();
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 2);
            assert_eq!(slots[0].timestamp, Some(instant_a));
            assert_eq!(slots[0].data, Some(42));
            assert!(slots[1].timestamp.is_none());
            assert!(slots[1].data.is_none());
        }

        let instant_b = SystemTime::now();
        let Update {
            items: databases,
            first_timestamp_of_non_finalized_database,
        } = consumer.consume(instant_b);
        assert_eq!(databases.len(), 1);
        assert_eq!(databases[0].timestamp, instant_a);
        assert_eq!(databases[0].data, 42);
        assert_eq!(first_timestamp_of_non_finalized_database, Some(instant_b));
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 1);
            assert_eq!(slots[0].timestamp, Some(instant_b));
            assert!(slots[0].data.is_none());
        }

        producer.finalize(1337);
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 1);
            assert_eq!(slots[0].timestamp, Some(instant_b));
            assert_eq!(slots[0].data, Some(1337));
        }
    }

    #[test]
    fn producer_faster_than_consumer() {
        let (producer, consumer) = future_queue();
        assert!(producer.slots.lock().is_empty());

        producer.announce();
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 1);
            assert!(slots[0].timestamp.is_none());
            assert!(slots[0].data.is_none());
        }

        let instant_a = SystemTime::now();
        let Update {
            items: databases,
            first_timestamp_of_non_finalized_database,
        } = consumer.consume(instant_a);
        assert!(databases.is_empty());
        assert_eq!(first_timestamp_of_non_finalized_database, Some(instant_a));
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 1);
            assert_eq!(slots[0].timestamp, Some(instant_a));
            assert!(slots[0].data.is_none());
        }

        producer.finalize(42);
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 1);
            assert_eq!(slots[0].timestamp, Some(instant_a));
            assert_eq!(slots[0].data, Some(42));
        }

        producer.announce();
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 2);
            assert_eq!(slots[0].timestamp, Some(instant_a));
            assert_eq!(slots[0].data, Some(42));
            assert!(slots[1].timestamp.is_none());
            assert!(slots[1].data.is_none());
        }

        producer.finalize(1337);
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 2);
            assert_eq!(slots[0].timestamp, Some(instant_a));
            assert_eq!(slots[0].data, Some(42));
            assert!(slots[1].timestamp.is_none());
            assert_eq!(slots[1].data, Some(1337));
        }

        producer.announce();
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 3);
            assert_eq!(slots[0].timestamp, Some(instant_a));
            assert_eq!(slots[0].data, Some(42));
            assert!(slots[1].timestamp.is_none());
            assert_eq!(slots[1].data, Some(1337));
            assert!(slots[2].timestamp.is_none());
            assert!(slots[2].data.is_none());
        }

        producer.finalize(17);
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 3);
            assert_eq!(slots[0].timestamp, Some(instant_a));
            assert_eq!(slots[0].data, Some(42));
            assert!(slots[1].timestamp.is_none());
            assert_eq!(slots[1].data, Some(1337));
            assert!(slots[2].timestamp.is_none());
            assert_eq!(slots[2].data, Some(17));
        }

        let instant_b = SystemTime::now();
        let Update {
            items: databases,
            first_timestamp_of_non_finalized_database,
        } = consumer.consume(instant_b);
        assert_eq!(databases.len(), 3);
        assert_eq!(databases[0].timestamp, instant_a);
        assert_eq!(databases[0].data, 42);
        assert_eq!(databases[1].timestamp, instant_b);
        assert_eq!(databases[1].data, 1337);
        assert_eq!(databases[2].timestamp, instant_b);
        assert_eq!(databases[2].data, 17);
        assert!(first_timestamp_of_non_finalized_database.is_none());
        {
            let slots = producer.slots.lock();
            assert!(slots.is_empty());
        }
    }

    #[test]
    fn consumer_faster_than_producer() {
        let (producer, consumer) = future_queue();
        assert!(producer.slots.lock().is_empty());

        producer.announce();
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 1);
            assert!(slots[0].timestamp.is_none());
            assert!(slots[0].data.is_none());
        }

        let instant_a = SystemTime::now();
        let Update {
            items: databases,
            first_timestamp_of_non_finalized_database,
        } = consumer.consume(instant_a);
        assert!(databases.is_empty());
        assert_eq!(first_timestamp_of_non_finalized_database, Some(instant_a));
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 1);
            assert_eq!(slots[0].timestamp, Some(instant_a));
            assert!(slots[0].data.is_none());
        }

        let instant_b = SystemTime::now();
        let Update {
            items: databases,
            first_timestamp_of_non_finalized_database,
        } = consumer.consume(instant_b);
        assert!(databases.is_empty());
        assert_eq!(first_timestamp_of_non_finalized_database, Some(instant_a));
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 1);
            assert_eq!(slots[0].timestamp, Some(instant_a));
            assert!(slots[0].data.is_none());
        }

        producer.finalize(42);
        {
            let slots = producer.slots.lock();
            assert_eq!(slots.len(), 1);
            assert_eq!(slots[0].timestamp, Some(instant_a));
            assert_eq!(slots[0].data, Some(42));
        }

        let instant_c = SystemTime::now();
        let Update {
            items: databases,
            first_timestamp_of_non_finalized_database,
        } = consumer.consume(instant_c);
        assert_eq!(databases.len(), 1);
        assert_eq!(databases[0].timestamp, instant_a);
        assert_eq!(databases[0].data, 42);
        assert!(first_timestamp_of_non_finalized_database.is_none());
        {
            let slots = producer.slots.lock();
            assert!(slots.is_empty());
        }

        let instant_d = SystemTime::now();
        let Update {
            items: databases,
            first_timestamp_of_non_finalized_database,
        } = consumer.consume(instant_d);
        assert!(databases.is_empty());
        assert!(first_timestamp_of_non_finalized_database.is_none());
        {
            let slots = producer.slots.lock();
            assert!(slots.is_empty());
        }
    }
}
