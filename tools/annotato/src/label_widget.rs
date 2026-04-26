use std::{
    fs::{self, File},
    io::Write,
};

use crate::{
    annotation::AnnotationFormat,
    boundingbox::BoundingBox,
    classes::Class,
    leaderboard,
    paths::Paths,
    utils,
    widgets::{bounding_box_annotator::BoundingBoxAnnotator, class_selector::ClassSelector},
};
use color_eyre::eyre::{ContextCompat, Result};
use eframe::{
    egui::Id,
    epaint::{Color32, TextureHandle},
};

pub struct LabelWidget {
    current_paths: Option<Paths>,
    texture_handle: Option<TextureHandle>,
    image_size: Option<[f32; 2]>,
    selected_class: Class,
    bounding_boxes: Vec<BoundingBox>,
    editing_bounding_box: Option<BoundingBox>,
    disable_saving: bool,
    use_model_annotations: bool,
    unresolved_annotations: Vec<AnnotationFormat>,
}

impl Default for LabelWidget {
    fn default() -> Self {
        Self {
            current_paths: None,
            texture_handle: None,
            image_size: None,
            selected_class: Class::Robot,
            bounding_boxes: Vec::new(),
            editing_bounding_box: None,
            disable_saving: false,
            use_model_annotations: true,
            unresolved_annotations: Vec::new(),
        }
    }
}

impl LabelWidget {
    pub fn has_paths(&self, paths: &Paths) -> bool {
        self.current_paths
            .as_ref()
            .map(|current_paths| paths.image_path == current_paths.image_path)
            .unwrap_or(false)
    }

    pub fn ui(&mut self, ui: &mut eframe::egui::Ui) {
        if self.texture_handle.is_none() {
            if let Some(paths) = self.current_paths.as_ref() {
                let handle =
                    utils::load_image(ui, &paths.image_path).expect("failed to load image");
                let size = [handle.size()[0] as f32, handle.size()[1] as f32];
                self.image_size = Some(size);
                self.texture_handle = Some(handle);

                self.bounding_boxes = self
                    .unresolved_annotations
                    .drain(..)
                    .map(|annotation| BoundingBox::from_annotation(annotation, size))
                    .collect();
            } else {
                panic!("No image loaded");
            }
        }

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if let Some(paths) = &self.current_paths {
                    ui.label(paths.image_path.display().to_string());
                    if paths.label_present {
                        ui.colored_label(Color32::GREEN, "✔");
                    } else {
                        ui.colored_label(Color32::RED, "❌");
                    }
                }
                ui.add(ClassSelector::new(
                    "class-selector",
                    &mut self.selected_class,
                ));
                ui.checkbox(&mut self.disable_saving, "Disable Annotation Saving");
                ui.checkbox(&mut self.use_model_annotations, "AI-ssist");
            });
            if let (Some(texture_handle), Some(image_size)) =
                (self.texture_handle.clone(), self.image_size)
            {
                ui.add(BoundingBoxAnnotator::new(
                    Id::new(&texture_handle).with("image-plot"),
                    texture_handle,
                    image_size,
                    &mut self.bounding_boxes,
                    &mut self.editing_bounding_box,
                    &mut self.selected_class,
                ));
            }
        });
    }

    pub fn load_new_image_with_labels(
        &mut self,
        paths: Paths,
        model_annotations: Vec<AnnotationFormat>,
    ) -> Result<()> {
        self.bounding_boxes.clear();
        self.unresolved_annotations.clear();
        self.image_size = None;

        if paths.label_path.exists() {
            let existing_annotations = fs::read_to_string(&paths.label_path)?;
            self.unresolved_annotations = serde_json::from_str(&existing_annotations)?;
        } else if self.use_model_annotations {
            self.unresolved_annotations = model_annotations;
        }

        self.texture_handle = None;
        self.current_paths = Some(paths);

        Ok(())
    }

    pub fn save_annotation(&mut self) -> Result<()> {
        let paths = self
            .current_paths
            .as_ref()
            .wrap_err("no image loaded currently")?;

        if self.disable_saving {
            return Ok(());
        }

        if !paths.label_path.exists() {
            leaderboard::send_score_up()?;
        }

        let image_size = self.image_size.wrap_err("no image size available")?;
        let annotations: Vec<AnnotationFormat> = self
            .bounding_boxes
            .drain(..)
            .map(|bounding_box| bounding_box.to_annotation(image_size))
            .collect();

        let annotations = serde_json::to_string_pretty(&annotations)?;

        let mut file = File::create(&paths.label_path)?;
        file.write_all(annotations.as_bytes())?;

        Ok(())
    }
}
