use std::collections::BTreeMap;

use eframe::egui::{vec2, Align, Layout, Rect, Response, Ui, UiBuilder, Vec2, Widget};

use framework::Timing;

use crate::{
    coordinate_systems::{FrameRange, RelativeTime, ViewportRange},
    frames::Frames,
    ticks::{ticks_height, Ticks},
};

pub struct Timeline<'state> {
    indices: &'state BTreeMap<String, Vec<Timing>>,
    frame_range: &'state FrameRange,
    viewport_range: &'state mut ViewportRange,
    position: &'state mut RelativeTime,
}

impl<'state> Timeline<'state> {
    pub fn new(
        indices: &'state BTreeMap<String, Vec<Timing>>,
        frame_range: &'state FrameRange,
        viewport_range: &'state mut ViewportRange,
        position: &'state mut RelativeTime,
    ) -> Self {
        Self {
            indices,
            frame_range,
            viewport_range,
            position,
        }
    }
}

impl Widget for Timeline<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let original_item_spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
            // defer rendering of Ticks to not render out-dated self.position modified by Frames
            let ticks_rect = Rect::from_min_size(
                ui.max_rect().left_top(),
                vec2(ui.available_width(), ticks_height(ui)),
            );
            ui.advance_cursor_after_rect(ticks_rect);

            let response = ui.add(Frames::new(
                self.indices,
                self.frame_range,
                self.viewport_range,
                self.position,
                original_item_spacing,
            ));

            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(ticks_rect)
                    .layout(Layout::top_down_justified(Align::Min)),
                |ui| {
                    Ticks::new(self.frame_range, self.viewport_range, self.position).ui(ui);
                },
            );

            response
        })
        .inner
    }
}
