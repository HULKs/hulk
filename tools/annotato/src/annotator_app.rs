use std::{collections::VecDeque, path::{PathBuf, Path}};

use crate::{ai_assistant::ModelAnnotations, label_widget::LabelWidget, paths::Paths, Args};
use color_eyre::{eyre::ContextCompat, Result};
use eframe::{
    egui::{
        CentralPanel, Context, Key, Layout, ProgressBar, RichText, ScrollArea, SidePanel, TextStyle,
    },
    emath::Align,
    epaint::Color32,
    App, CreationContext,
};
use glob::glob;

enum AnnotationPhase {
    Started,
    Labelling,
    Finished,
}

pub struct AnnotatorApp {
    phase: AnnotationPhase,
    paths: VecDeque<Paths>,
    current_index: usize,
    label_widget: LabelWidget,
    model_annotations: ModelAnnotations,
}

impl AnnotatorApp {
    fn convert_image_to_label_path(image_path: &Path) -> Result<PathBuf> {
        let filename = image_path
            .file_name()
            .wrap_err("no filename")?
            .to_str()
            .unwrap();
        let filename_without_ext = filename
            .rsplit_once('.')
            .map(|(prefix, _suffix)| prefix)
            .unwrap();
        let mut label_path = image_path.to_path_buf();
        label_path.set_file_name(format!("{filename_without_ext}.json"));
        Ok(label_path)
    }

    pub fn try_new(_: &CreationContext, arguments: Args) -> Result<Self> {
        let image_paths = glob(&arguments.image_folder.join("*.png").display().to_string())?
            .collect::<Result<VecDeque<_>, _>>()?;

        let model_annotations = ModelAnnotations::try_new(&arguments.annotation_json)?;

        let paths = image_paths
            .into_iter()
            .map(|image_path| {
                let label_path = Self::convert_image_to_label_path(&image_path)?;
                Ok(Paths::new(image_path, label_path))
            })
            .collect::<Result<VecDeque<_>>>()
            .expect("failed to build paths");

        let phase = if arguments.skip_introduction {
            AnnotationPhase::Labelling
        } else {
            AnnotationPhase::Started
        };

        let mut this = AnnotatorApp {
            phase,
            paths,
            current_index: 0,
            label_widget: LabelWidget::default(),
            model_annotations,
        };
        this.update_image().expect("failed to load image");

        Ok(this)
    }

    fn update_image(&mut self) -> Result<()> {
        if let Some(paths) = self.paths.get_mut(self.current_index) {
            let annotations = self
                .model_annotations
                .for_image(
                    &paths
                        .image_path
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                )
                .unwrap_or(vec![]);

            self.label_widget
                .load_new_image_with_labels(paths.clone(), annotations)?;
        }

        self.paths.iter_mut().for_each(|paths| {
            paths.check_existence();
        });

        if self.paths.iter().all(|paths| paths.label_present) {
            self.phase = AnnotationPhase::Finished;
        }

        Ok(())
    }

    pub fn set_index_to_unlabelled(&mut self) {
        todo!();
    }

    fn show_phase_started(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(200.0);
                ui.label(RichText::new("Annotato-rs").size(32.0).strong());
                let number_unlabelled_images = self
                    .paths
                    .iter()
                    .filter(|paths| paths.label_present)
                    .count();
                ui.label(format!(
                    "You are about to label {number_unlabelled_images} images."
                ));
                ui.add_space(100.0);

                ui.label(RichText::new("Labelling Instructions").text_style(TextStyle::Heading));
                ui.add_space(50.0);
                ui.vertical_centered(|ui| {
                    ui.label("• select a class with a number key");
                    ui.label("• start drawing a box with 'b' key");
                    ui.label("• end drawing a box with 'b' key");
                    ui.label("• delete a box by hovering and rightclicking");
                    ui.label("• move in the image with left click dragging");
                    ui.label("• zoom in the image ctrl + mousewheel");
                    ui.label("• proceed to the next image with 'n' key");
                });

                ui.add_space(50.0);
                if ui.button("Start Labelling").clicked() {
                    self.phase = AnnotationPhase::Labelling;
                }
            });
        });
    }

    fn show_phase_labelling(&mut self, ctx: &Context) {
        SidePanel::left("image-path-list")
            .default_width(200.0)
            .show(ctx, |ui| {
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
                        for (filename, is_labelled) in self.paths.range(range).filter_map(|path| {
                            path.image_path
                                .file_name()
                                .and_then(|osstr| osstr.to_str())
                                .map(|filename| (filename, path.label_present))
                        }) {
                            ui.horizontal(|ui| {
                                ui.label(filename);
                                ui.add_space(40.0);
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.add_space(20.0);
                                    if is_labelled {
                                        ui.colored_label(Color32::GREEN, "✔");
                                    } else {
                                        ui.colored_label(Color32::RED, "❌");
                                    }
                                });
                            });
                            ui.separator();
                        }
                    });
                ui.separator();
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        if ui.button("<").on_hover_text("Previous image").clicked()
                            && self.current_index > 0
                        {
                            self.current_index -= 1;
                            self.update_image().expect("failed to update image");
                        }
                        if ui.button(">").on_hover_text("Next image (n, →)").clicked()
                            || ui.input(|i| i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::N))
                        {
                            if self.current_index < self.paths.len() - 1 {
                                self.current_index += 1;
                            }
                            self.update_image().expect("failed to update image");
                        }
                        if ui
                            .button(">>")
                            .on_hover_text("Go to the first unlabelled image")
                            .clicked()
                        {
                            if let Some((unlabelled_index, _)) = self
                                .paths
                                .iter()
                                .enumerate()
                                .find(|(_, paths)| !paths.label_present)
                            {
                                self.current_index = unlabelled_index;
                                self.update_image().expect("failed to update image");
                            } else {
                                // no more unlabelled images
                                self.phase = AnnotationPhase::Finished;
                            }
                        }
                    })
                })
            });

        CentralPanel::default().show(ctx, |ui| {
            self.label_widget.ui(ui);
        });
    }

    fn show_phase_finished(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.label("You finished the data chunk, take the next and go on :)");
        });
    }
}

impl App for AnnotatorApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);

        match self.phase {
            AnnotationPhase::Started => self.show_phase_started(ctx),
            AnnotationPhase::Labelling => self.show_phase_labelling(ctx),
            AnnotationPhase::Finished => self.show_phase_finished(ctx),
        }
    }
}
