use std::time::Duration;

use bevy::{ecs::system::ResMut, time::Time};

#[derive(Default)]
pub struct Ticks(u32);

pub fn update_time(mut time: ResMut<Time<Ticks>>, mut generic_time: ResMut<Time>) {
    time.context_mut().0 += 1;
    time.advance_by(Duration::from_millis(12));

    *generic_time = time.as_generic();
}

pub trait TicksTime {
    fn ticks(&self) -> u32;
}

impl TicksTime for Time<Ticks> {
    fn ticks(&self) -> u32 {
        self.context().0
    }
}
