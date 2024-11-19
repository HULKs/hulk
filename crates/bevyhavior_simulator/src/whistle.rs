use std::time::Duration;

use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct WhistleResource {
    pub last_whistle: Option<Duration>,
}

impl WhistleResource {
    pub fn whistle(&mut self, time: Time) {
        self.last_whistle = Some(time.elapsed());
    }
}
