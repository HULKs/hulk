use std::{sync::Mutex, time::SystemTime};

use hardware::HardwareInterface;

#[derive(Default)]
pub struct Interface {
    current_value: Mutex<usize>,
}

impl HardwareInterface for Interface {
    fn get_now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn get_random_number(&self) -> usize {
        let mut value = self.current_value.lock().unwrap();
        *value += 1;
        *value
    }

    fn print_number(&self, number: usize) {
        println!("nao number: {number}");
    }
}
