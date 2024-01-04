use eframe::{
    egui::{Response, RichText, Sense, TextStyle, Ui, Widget, WidgetText},
    epaint::{Color32, Vec2},
};

use crate::paths::Paths;

pub struct Row<'a> {
    paths: &'a Paths,
    highlight: bool,
}

impl<'a> Row<'a> {
    pub fn new(paths: &'a Paths) -> Self {
        Self {
            paths,
            highlight: false,
        }
    }

    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }
}

impl<'a> Widget for Row<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let filename = self.paths.image_path.display().to_string();
        let is_labelled = self.paths.label_present;

        let text: WidgetText = RichText::new(filename).monospace().into();
        let check_mark: WidgetText = if is_labelled {
            RichText::new("✔").color(Color32::GREEN)
        } else {
            RichText::new("❌").color(Color32::RED)
        }
        .into();
        let text = text.into_galley(ui, Some(false), ui.available_width(), TextStyle::Button);
        let check_mark =
            check_mark.into_galley(ui, Some(false), ui.available_width(), TextStyle::Button);

        let desired_size = Vec2::new(
            text.size().x + 40.0 + check_mark.size().x + 20.0,
            text.size().y + 2.0 * 4.0,
        );

        let (rect, response) = ui.allocate_at_least(desired_size, Sense::click());
        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);
            let text_height = Vec2::new(0., text.size().y);
            let check_mark_offset = Vec2::new(text.size().x + 40.0, 0.0);

            if response.hovered || self.highlight {
                ui.painter().rect_filled(rect, 2.0, visuals.bg_fill);
            }

            text.paint_with_visuals(
                ui.painter(),
                rect.left_center() - 0.5 * text_height,
                visuals,
            );
            check_mark.paint_with_visuals(
                ui.painter(),
                rect.left_center() - 0.5 * text_height + check_mark_offset,
                visuals,
            );
        }

        response
    }
}
