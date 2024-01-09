use std::{
    fs::{self, File},
    io::Write,
};

use crate::{
    annotation::AnnotationFormat,
    boundingbox::BoundingBox,
    classes::Class,
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
    texture_id: Option<TextureHandle>,
    selected_class: Class,
    bounding_boxes: Vec<BoundingBox>,
    editing_bounding_box: Option<BoundingBox>,
    disable_saving: bool,
    use_model_annotations: bool,
}

impl Default for LabelWidget {
    fn default() -> Self {
        Self {
            current_paths: None,
            texture_id: None,
            selected_class: Class::Robot,
            bounding_boxes: Vec::new(),
            editing_bounding_box: None,
            disable_saving: false,
            use_model_annotations: true,
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
        if self.texture_id.is_none() {
            self.texture_id.get_or_insert_with(|| {
                if let Some(paths) = self.current_paths.as_ref() {
                    utils::load_image(ui, &paths.image_path).expect("failed to load image")
                } else {
                    panic!("No image loaded");
                }
            });
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
            if let Some(texture_id) = self.texture_id.clone() {
                ui.add(BoundingBoxAnnotator::new(
                    Id::new(&texture_id).with("image-plot"), // using the texture_id as hash to reset plot on new image
                    texture_id.clone(),
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
        model_annotations: Vec<BoundingBox>,
    ) -> Result<()> {
        self.bounding_boxes.clear();

        if paths.label_path.exists() {
            let existing_annotations = fs::read_to_string(&paths.label_path)?;
            let mut existing_annotations: Vec<AnnotationFormat> =
                serde_json::from_str(&existing_annotations)?;
            self.bounding_boxes = existing_annotations
                .drain(..)
                .map(|annotation| annotation.into())
                .collect();
        } else if self.use_model_annotations {
            self.bounding_boxes.extend(model_annotations);
        }

        self.texture_id = None;
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

        let annotations: Vec<AnnotationFormat> = self
            .bounding_boxes
            .drain(..)
            .map(|bbox| bbox.into())
            .collect();
        let annotations = serde_json::to_string_pretty(&annotations)?;

        let mut file = File::create(&paths.label_path)?;
        file.write_all(annotations.as_bytes())?;

        Ok(())
    }
}
