use std::{
    ops::RangeInclusive,
    time::{Duration, SystemTime},
};

use eframe::{
    egui::{CentralPanel, Context, Slider},
    Frame,
};

pub struct ReplayerApplication<OnChange> {
    start: SystemTime,
    end: SystemTime,
    current: SystemTime,
    on_change: OnChange,
}

impl<OnChange> ReplayerApplication<OnChange>
where
    OnChange: FnMut(SystemTime),
{
    pub fn new(
        start: SystemTime,
        end: SystemTime,
        current: SystemTime,
        on_change: OnChange,
    ) -> Self {
        Self {
            start,
            end,
            current,
            on_change,
        }
    }
}

impl<OnChange> eframe::App for ReplayerApplication<OnChange>
where
    OnChange: FnMut(SystemTime),
{
    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(context, |ui| {
            ui.heading("Replayer");
            ui.horizontal(|ui| {
                ui.style_mut().spacing.slider_width = ui.available_size().x - 100.0;
                let length = self.end.duration_since(self.start).unwrap().as_secs_f64();
                let mut current = self
                    .current
                    .duration_since(self.start)
                    .unwrap()
                    .as_secs_f64();
                let changed = ui
                    .add(Slider::new(&mut current, RangeInclusive::new(0.0, length)))
                    .changed();
                self.current = self.start + Duration::from_secs_f64(current);
                if changed {
                    (self.on_change)(self.current);
                }
            })
        });
    }
}
