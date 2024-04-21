use eframe::egui::{
    pos2, vec2, Align, Layout, Rect, Response, RichText, Sense, TextStyle, Ui, Widget,
};

use crate::{execution::Replayer, ticks::ticks_height, ReplayerHardwareInterface};

pub struct Labels<'state> {
    replayer: &'state Replayer<ReplayerHardwareInterface>,
}

impl<'state> Labels<'state> {
    pub fn new(replayer: &'state Replayer<ReplayerHardwareInterface>) -> Self {
        Self { replayer }
    }

    fn generate_label_contents(&self) -> Vec<LabelContent> {
        self.replayer
            .get_recording_indices()
            .into_iter()
            .map(|(name, index)| LabelContent {
                name,
                number_of_frames: index.number_of_frames(),
            })
            .collect()
    }
}

impl<'state> Widget for Labels<'state> {
    fn ui(self, ui: &mut Ui) -> Response {
        let label_contents = self.generate_label_contents();
        let spacing = ui.spacing().item_spacing.y;
        let total_spacing = spacing * (label_contents.len() - 1) as f32;
        let row_height = (ui.available_height() - total_spacing - ticks_height(ui))
            / label_contents.len() as f32;
        let height =
            row_height * label_contents.len() as f32 + spacing * (label_contents.len() - 1) as f32;
        let left_top = ui.cursor().min + vec2(0.0, ticks_height(ui));

        let mut maximum_width = 0.0_f32;
        for (index, label_content) in label_contents.into_iter().enumerate() {
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
}
