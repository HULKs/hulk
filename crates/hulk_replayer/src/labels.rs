use eframe::egui::{
    pos2, vec2, Align, Layout, Rect, Response, RichText, Sense, TextStyle, Ui, Widget,
};
use framework::ScanState;

use crate::{execution::Replayer, ticks::ticks_height, ReplayerHardwareInterface};

pub struct Labels {
    labels: Vec<LabelContent>,
}

impl Labels {
    pub fn new(replayer: &Replayer<ReplayerHardwareInterface>) -> Self {
        let labels = replayer
            .get_recording_indices()
            .into_iter()
            .map(|(name, index)| LabelContent {
                name,
                number_of_frames: index.number_of_frames(),
                scan_state: index.scan_state(),
            })
            .collect();

        Self { labels }
    }
}

impl Widget for Labels {
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
            if row_height
                >= (2.0 * ui.style().text_styles.get(&TextStyle::Body).unwrap().size)
                    + ui.spacing().item_spacing.y
            {
                child_ui.label(format!("{} frames", label_content.number_of_frames));
            }
            if row_height
                >= (3.0 * ui.style().text_styles.get(&TextStyle::Body).unwrap().size)
                    + ui.spacing().item_spacing.y
            {
                if let ScanState::Loading { progress } = label_content.scan_state {
                    child_ui.label(format!("{:.2} %", progress * 100.0));
                }
            }
            maximum_width = maximum_width.max(child_ui.min_size().x);
        }

        ui.allocate_rect(
            Rect::from_min_size(left_top, vec2(maximum_width, height)),
            Sense::hover(),
        )
    }
}

struct LabelContent {
    name: String,
    number_of_frames: usize,
    scan_state: ScanState,
}
