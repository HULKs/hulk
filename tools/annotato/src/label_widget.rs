use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use crate::{
    annotation::AnnotationFormat,
    boundingbox::BoundingBox,
    classes::Classes,
    paths::Paths,
    widgets::{bounding_box_annotator::BoundingBoxAnnotator, class_selector::ClassSelector},
};
use color_eyre::eyre::Result;
use eframe::{
    egui::Ui,
    epaint::{Color32, ColorImage, TextureHandle},
};

fn load_image_from_path(path: impl AsRef<Path>) -> Result<ColorImage, image::ImageError> {
    let image = image::io::Reader::open(path)?.decode()?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
}

pub struct LabelWidget {
    current_paths: Option<Paths>,
    texture_id: Option<TextureHandle>,
    selected_class: Classes,
    bounding_boxes: Vec<BoundingBox>,
    editing_bounding_box: Option<BoundingBox>,
    auto_save_on_next_image: bool,
    use_model_annotations: bool,
}

impl Default for LabelWidget {
    fn default() -> Self {
        Self {
            current_paths: None,
            texture_id: None,
            selected_class: Classes::Robot,
            bounding_boxes: Vec::new(),
            editing_bounding_box: None,
            auto_save_on_next_image: true,
            use_model_annotations: false,
        }
    }
}

impl LabelWidget {
    pub fn load_image(&mut self, ui: &Ui) -> Result<()> {
        if let (None, Some(paths)) = (&self.texture_id, &self.current_paths) {
            let image = load_image_from_path(&paths.image_path)?;

            let handle = ui.ctx().load_texture(
                paths.image_path.display().to_string(),
                image,
                Default::default(),
            );
            self.texture_id = Some(handle);
        }
        Ok(())
    }

    pub fn ui(&mut self, ui: &mut eframe::egui::Ui) {
        self.load_image(ui).expect("failed to load image");

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
                ui.checkbox(&mut self.auto_save_on_next_image, "Auto-Save");
                ui.checkbox(&mut self.use_model_annotations, "AI-ssist");
            });
            if let Some(texture_id) = self.texture_id.clone() {
                ui.add(BoundingBoxAnnotator::new(
                    "image-plot",
                    texture_id.clone(),
                    &mut self.bounding_boxes,
                    &mut self.editing_bounding_box,
                    self.selected_class,
                ));
            }
        });
    }

    pub fn load_new_image_with_labels(
        &mut self,
        paths: Paths,
        model_annotations: Vec<BoundingBox>,
    ) -> Result<()> {
        if let (true, Some(paths)) = (self.auto_save_on_next_image, &self.current_paths) {
            // export current bboxes
            let annotations: Vec<AnnotationFormat> = self
                .bounding_boxes
                .drain(..)
                .map(|bbox| bbox.into())
                .collect();
            let annotations = serde_json::to_string_pretty(&annotations)?;

            let mut file = File::create(&paths.label_path)?;
            file.write_all(annotations.as_bytes())?;
        }

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
}
