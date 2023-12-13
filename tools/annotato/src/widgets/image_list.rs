use std::collections::VecDeque;

use eframe::egui::{ProgressBar, ScrollArea, Widget};

use crate::{annotator_app::AnnotationPhase, paths::Paths};

use super::path_row::Row;
pub struct ImageList<'a> {
    paths: &'a VecDeque<Paths>,
    phase: &'a mut AnnotationPhase,
}

impl<'a> ImageList<'a> {
    pub fn new(paths: &'a VecDeque<Paths>, phase: &'a mut AnnotationPhase) -> Self {
        Self { paths, phase }
    }
}

impl<'a> Widget for ImageList<'a> {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        ui.vertical(|ui| {
            ui.label("Image List");
            let images_done = self
                .paths
                .iter()
                .filter(|paths| paths.label_present)
                .count();
            ui.add(
                ProgressBar::new(images_done as f32 / self.paths.len() as f32)
                    .show_percentage()
                    .text(format!("{}/{}", images_done, self.paths.len())),
            );
            ui.separator();

            ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(0.8 * ui.available_height())
                .show_rows(ui, 12.0, self.paths.len(), |ui, range| {
                    for (paths, index) in self.paths.range(range.clone()).zip(range) {
                        let row = ui.add(Row::new(paths).highligh(match self.phase {
                            AnnotationPhase::Labelling { current_index } => *current_index == index,
                            _ => false,
                        }));

                        if let AnnotationPhase::Labelling { current_index } = self.phase {
                            if *current_index == index {
                                row.scroll_to_me(None);
                            }
                        }

                        if row.clicked() {
                            *self.phase = AnnotationPhase::Labelling {
                                current_index: index,
                            };
                        }
                        ui.separator();
                    }
                });
            ui.separator();
        })
        .response
    }
}
