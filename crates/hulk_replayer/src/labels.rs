use std::collections::BTreeMap;

use eframe::egui::{
    pos2, vec2, Align, Layout, Rect, Response, RichText, Sense, TextStyle, Ui, Widget,
};

use framework::Timing;

use crate::ticks::ticks_height;

pub struct Labels<'state> {
    labels: Vec<LabelContent<'state>>,
}

impl<'state> Labels<'state> {
    pub fn new(indices: &'state BTreeMap<String, Vec<Timing>>) -> Self {
        let labels = indices
            .iter()
            .map(|(name, timings)| LabelContent {
                name,
                number_of_frames: timings.len(),
            })
            .collect();

        Self { labels }
    }
}

impl<'state> Widget for Labels<'state> {
    fn ui(self, ui: &mut Ui) -> Response {
        let spacing = ui.spacing().item_spacing.y;
        let total_spacing = spacing * (self.labels.len() - 1) as f32;
        let row_height =
            (ui.available_height() - total_spacing - ticks_height(ui)) / self.labels.len() as f32;
        let height =
            row_height * self.labels.len() as f32 + spacing * (self.labels.len() - 1) as f32;
        let left_top = ui.cursor().min + vec2(0.0, ticks_height(ui));

        let mut maximum_width = 0.0_f32;
        for (index, label_content) in self.labels.into_iter().enumerate() {
            let left_top = left_top + vec2(0.0, (row_height + spacing) * index as f32);
            let child_rect = Rect::from_min_max(
                left_top,
                pos2(ui.max_rect().right(), left_top.y + row_height),
            );
            let mut child_ui = ui.child_ui(child_rect, Layout::top_down(Align::Min));
            child_ui.set_height(row_height);
            child_ui.label(RichText::new(label_content.name).strong());
            let text_height = ui.style().text_styles.get(&TextStyle::Body).unwrap().size;
            if child_ui.available_height() >= text_height {
                child_ui.label(format!("{} frames", label_content.number_of_frames));
            }
            maximum_width = maximum_width.max(child_ui.min_size().x);
        }

        ui.allocate_rect(
            Rect::from_min_size(left_top, vec2(maximum_width, height)),
            Sense::hover(),
        )
    }
}

struct LabelContent<'state> {
    name: &'state str,
    number_of_frames: usize,
}
