use std::{sync::Mutex, time::SystemTime};

pub trait HardwareInterface {
    fn get_now(&self) -> SystemTime;
    fn get_random_number(&self) -> usize;
    fn print_number(&self, number: usize);
}

#[derive(Default)]
pub struct NaoInterface {
    current_value: Mutex<usize>,
}

impl HardwareInterface for NaoInterface {
    fn get_now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn get_random_number(&self) -> usize {
        let mut value = self.current_value.lock().unwrap();
        *value += 1;
        *value
    }

    fn print_number(&self, number: usize) {
        println!("number: {number}");
    }
}
