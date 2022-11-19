use std::time::SystemTime;

pub trait HardwareInterface {
    fn get_now(&self) -> SystemTime;
    fn get_random_number(&self) -> usize;
    fn print_number(&self, number: usize);
}
