use std::{collections::VecDeque, path::PathBuf};

use crate::label_widget::LabelWidget;
use color_eyre::{eyre::ContextCompat, Result};
use eframe::{
    egui::{self, CentralPanel, Key, ScrollArea, SidePanel},
    App, CreationContext,
};
use glob::glob;

pub struct AnnotatorApp {
    image_paths: VecDeque<PathBuf>,
    current_index: usize,
    label_widget: LabelWidget,
}

impl AnnotatorApp {
    fn convert_image_to_label_path(image_path: &PathBuf) -> Result<PathBuf> {
        let filename = image_path
            .file_name()
            .wrap_err("no filename")?
            .to_str()
            .unwrap();
        let filename_without_ext = filename
            .rsplit_once(".")
            .map(|(prefix, _suffix)| prefix)
            .unwrap();
        let mut label_path = image_path.clone();
        label_path.set_file_name(format!("{filename_without_ext}.json"));
        Ok(label_path)
    }

    pub fn try_new(_: &CreationContext) -> Result<Self> {
        let image_paths = glob("./images/*.png")?.collect::<Result<VecDeque<_>, _>>()?;

        Ok(AnnotatorApp {
            image_paths,
            current_index: 0,
            label_widget: LabelWidget::new(),
        })
    }

    fn update_image(&mut self) -> Result<()> {
        if let Some(image_path) = self.image_paths.get(self.current_index) {
            let label_path = Self::convert_image_to_label_path(&image_path)?;
            self.label_widget
                .load_new_image_with_labels(image_path.clone(), label_path)?;
        }

        Ok(())
    }

    pub fn set_index_to_unlabelled(&mut self) {
        todo!();
    }
}

impl App for AnnotatorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);

        SidePanel::left("image-path-list")
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.label("Image List");
                ui.separator();
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_width(200.0)
                    .max_height(0.8 * ui.available_height())
                    .show_rows(ui, 12.0, self.image_paths.len(), |ui, range| {
                        for filename in self.image_paths.range(range).filter_map(|path| {
                            path.file_name().map(|osstr| osstr.to_str()).flatten()
                        }) {
                            ui.label(filename);
                            ui.separator();
                        }
                    });
                ui.separator();
                ui.horizontal_centered(|ui| {
                    if ui.button("<").clicked() && self.current_index > 0 {
                        self.current_index -= 1;
                    }
                    if ui.button(">").clicked() || ui.input(|i| i.key_pressed(Key::ArrowRight)) {
                        if self.current_index < self.image_paths.len() - 1 {
                            self.current_index += 1;
                        }
                        self.update_image().expect("failed to update image");
                    }
                })
            });

        CentralPanel::default().show(ctx, |ui| {
            self.label_widget.ui(ui);
        });
    }
}
