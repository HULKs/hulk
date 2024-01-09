use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

use crate::{
    ai_assistant::ModelAnnotations, label_widget::LabelWidget, paths::Paths,
    widgets::image_list::ImageList,
};
use color_eyre::{eyre::Context as C, Result};
use eframe::{
    egui::{CentralPanel, Context, Key, RichText, SidePanel, TextStyle},
    App, CreationContext,
};
use glob::glob;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotationPhase {
    Started,
    Labelling { current_index: usize },
    Finished,
}

pub struct AnnotatorApp {
    phase: AnnotationPhase,
    paths: VecDeque<Paths>,
    label_widget: LabelWidget,
    model_annotations: ModelAnnotations,
}

impl AnnotatorApp {
    fn convert_image_to_label_path(image_path: &Path) -> PathBuf {
        image_path.with_extension("json")
    }

    pub fn try_new(
        _: &CreationContext,
        image_folder: impl AsRef<Path>,
        annotation_json_path: impl AsRef<Path>,
        skip_introduction: bool,
    ) -> Result<Self> {
        let image_paths = glob(
            &image_folder
                .as_ref()
                .to_path_buf()
                .join("*.png")
                .display()
                .to_string(),
        )?
        .collect::<Result<VecDeque<_>, _>>()?;

        let model_annotations = ModelAnnotations::try_new(annotation_json_path)?;

        let paths = image_paths
            .into_iter()
            .map(|image_path| {
                let label_path = Self::convert_image_to_label_path(&image_path);
                Ok(Paths::new(image_path, label_path))
            })
            .collect::<Result<VecDeque<_>>>()
            .expect("failed to build paths");

        let phase = if skip_introduction {
            AnnotationPhase::Labelling { current_index: 0 }
        } else {
            AnnotationPhase::Started
        };

        Ok(AnnotatorApp {
            phase,
            paths,
            label_widget: LabelWidget::default(),
            model_annotations,
        })
    }

    fn next(&mut self) -> Result<()> {
        self.phase = match self.phase {
            AnnotationPhase::Started => AnnotationPhase::Labelling { current_index: 0 },
            AnnotationPhase::Labelling { current_index } => {
                self.label_widget
                    .save_annotation()
                    .wrap_err("failed to go to next image")?;

                let new_index = current_index + 1;
                if new_index < self.paths.len() {
                    AnnotationPhase::Labelling {
                        current_index: new_index,
                    }
                } else {
                    AnnotationPhase::Finished
                }
            }
            AnnotationPhase::Finished => AnnotationPhase::Finished,
        };

        Ok(())
    }

    fn previous(&mut self) -> Result<()> {
        self.phase = match self.phase {
            AnnotationPhase::Started => AnnotationPhase::Started,
            AnnotationPhase::Labelling { current_index } => {
                self.label_widget
                    .save_annotation()
                    .wrap_err("failed to go to previous image")?;

                if current_index > 0 {
                    let new_index = current_index - 1;
                    AnnotationPhase::Labelling {
                        current_index: new_index,
                    }
                } else {
                    AnnotationPhase::Started
                }
            }
            AnnotationPhase::Finished => {
                let new_index = self.paths.len() - 1;
                AnnotationPhase::Labelling {
                    current_index: new_index,
                }
            }
        };

        Ok(())
    }

    fn load_image(&mut self) -> Result<()> {
        let index = match self.phase {
            AnnotationPhase::Started => 0,
            AnnotationPhase::Labelling { current_index } => current_index,
            AnnotationPhase::Finished => self.paths.len() - 1,
        };

        if let Some(paths) = self.paths.get_mut(index) {
            if self.label_widget.has_paths(paths) {
                return Ok(());
            }
            // get file name of the path as a string
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
                .unwrap_or_default();

            self.label_widget
                .load_new_image_with_labels(paths.clone(), annotations)?;
        }

        self.paths.iter_mut().for_each(|paths| {
            paths.check_existence();
        });

        Ok(())
    }

    fn show_phase_started(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(200.0);
                ui.label(RichText::new("Annotato-rs").size(32.0).strong());
                let number_unlabelled_images = self
                    .paths
                    .iter()
                    .filter(|paths| !paths.label_present)
                    .count();
                ui.label(format!(
                    "You are about to label {number_unlabelled_images} images."
                ));
                ui.add_space(100.0);

                ui.label(RichText::new("Labelling Instructions").text_style(TextStyle::Heading));
                ui.add_space(50.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(
                        r#"
                    • select a class with a number key
                    • start drawing a box with 'b' key
                    • end drawing a box with 'b' key
                    • delete a box by hovering and rightclicking
                    • move in the image with left click dragging
                    • zoom in the image ctrl + mousewheel
                    • proceed to the next image with 'n' key
                    "#,
                    ));
                });

                ui.add_space(50.0);
                if ui.button("Start Labelling").clicked() {
                    self.next().expect("failed to start labelling")
                }
            });
        });
    }

    fn show_phase_labelling(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            self.load_image().expect("failed to update image");
            self.label_widget.ui(ui);
        });
    }

    fn show_phase_finished(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label("You finished the data chunk, take the next and go on :)")
            })
        });
    }
}

impl App for AnnotatorApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);

        let width = ctx.screen_rect().x_range().span();
        SidePanel::left("image-path-list")
            .default_width(0.3 * width)
            .show(ctx, |ui| {
                let mut current_phase = self.phase.clone();
                ui.add(ImageList::new(&self.paths, &mut current_phase));

                if current_phase != self.phase {
                    if let AnnotationPhase::Labelling { .. } = self.phase {
                        self.label_widget
                            .save_annotation()
                            .expect("failed to save annotation");
                    }
                    self.phase = current_phase;
                }

                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .button("<")
                            .on_hover_text("Previous image (p, <)")
                            .clicked()
                            || ui.input(|i| {
                                i.key_pressed(Key::ArrowLeft)
                                    || i.key_pressed(Key::P)
                                    || (i.key_pressed(Key::Space) && i.modifiers.shift)
                            })
                        {
                            self.previous().expect("failed to load previous image");
                        }
                        if ui.button(">").on_hover_text("Next image (n, >)").clicked()
                            || ui.input(|i| {
                                i.key_pressed(Key::ArrowRight)
                                    || i.key_pressed(Key::N)
                                    || (i.key_pressed(Key::Space) && !i.modifiers.shift)
                            })
                        {
                            self.next().expect("failed to load next image");
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
                                self.phase = AnnotationPhase::Labelling {
                                    current_index: unlabelled_index,
                                };
                            } else {
                                // no more unlabelled images
                                self.phase = AnnotationPhase::Finished;
                            }
                        }
                    })
                })
            });

        match self.phase {
            AnnotationPhase::Started => self.show_phase_started(ctx),
            AnnotationPhase::Labelling { .. } => self.show_phase_labelling(ctx),
            AnnotationPhase::Finished => self.show_phase_finished(ctx),
        }
    }
}
