use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use crate::{
    annotation::AnnotationFormat, boundingbox::BoundingBox, classes::Classes, paths::Paths,
};
use color_eyre::eyre::Result;
use eframe::{
    egui::{ComboBox, Key, PointerButton, RichText, Ui},
    epaint::{Color32, ColorImage, TextureHandle, Vec2},
};
use egui_plot::{Plot, PlotImage, PlotPoint, PlotUi, Polygon, Text};

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
    box_in_drawing: Option<BoundingBox>,
    auto_save_on_next_image: bool,
    model_boxes: Vec<BoundingBox>,
    use_model_annotations: bool,
}

impl LabelWidget {
    pub fn new() -> Self {
        Self {
            current_paths: None,
            texture_id: None,
            selected_class: Classes::Robot,
            bounding_boxes: Vec::new(),
            box_in_drawing: None,
            auto_save_on_next_image: true,
            use_model_annotations: false,
            model_boxes: Vec::new(),
        }
    }

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

        let b_pressed = ui.input(|i| i.key_pressed(Key::B));
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                self.current_paths.as_ref().map(|paths| {
                    ui.label(paths.image_path.display().to_string());
                    if paths.label_present {
                        ui.colored_label(Color32::GREEN, "✔");
                    } else {
                        ui.colored_label(Color32::RED, "❌");
                    }
                });
                ComboBox::from_id_source("class-selector")
                    .selected_text(format!("{:?}", self.selected_class))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.selected_class, Classes::Robot, "Robot");
                        ui.selectable_value(&mut self.selected_class, Classes::Ball, "Ball");
                        ui.selectable_value(
                            &mut self.selected_class,
                            Classes::GoalPost,
                            "Goal Post",
                        );
                        ui.selectable_value(
                            &mut self.selected_class,
                            Classes::PenaltySpot,
                            "Penalty Spot",
                        );
                    });
                ui.checkbox(&mut self.auto_save_on_next_image, "Auto-Save");
                ui.checkbox(&mut self.use_model_annotations, "AI-ssist");
            });
            Plot::new("image-plot")
                .view_aspect(1.)
                .show_axes([false, false])
                .show_grid([false, false])
                .set_margin_fraction(Vec2::splat(0.1))
                .allow_scroll(false)
                .allow_boxed_zoom(false)
                .show(ui, |ui| {
                    self.texture_id.as_ref().map(|texture_handle| {
                        ui.image(PlotImage::new(
                            texture_handle,
                            PlotPoint::new(320., 240.),
                            Vec2::new(640., 480.),
                        ));
                        self.bounding_boxes
                            .iter()
                            .chain(self.box_in_drawing.iter())
                            .chain(
                                self.model_boxes
                                    .iter()
                                    .filter(|_| self.use_model_annotations),
                            )
                            .for_each(|bbox| {
                                let polygon: Polygon = bbox.into();
                                ui.polygon(polygon.fill_color(bbox.class.color()));
                                ui.text(Text::new(
                                    bbox.top_left(),
                                    RichText::new(format!("{:?}", bbox.class))
                                        .color(Color32::BLACK)
                                        .size(10.),
                                ))
                            });
                    });
                    self.handle_bounding_box_input(ui, b_pressed);
                });
        });

        if let Some(class) =
            ui.input(|i| i.keys_down.iter().find_map(|key| Classes::from_key(*key)))
        {
            self.selected_class = class;
        }
    }

    fn handle_bounding_box_input(&mut self, ui: &PlotUi, b_pressed: bool) {
        if let (true, Some(mouse_position)) = (b_pressed, ui.response().hover_pos()) {
            // insert current drawing bbox into list or create new one
            if let Some(mut bbox) = self.box_in_drawing.take() {
                bbox.clip_to_image();
                self.bounding_boxes.push(bbox);
            } else {
                let mouse_position = ui.plot_from_screen(mouse_position);
                self.box_in_drawing = Some(BoundingBox::new(
                    mouse_position,
                    mouse_position,
                    self.selected_class,
                ));
            }
        }

        if let (Some(bbox), Some(mouse_position)) =
            (self.box_in_drawing.as_mut(), ui.response().hover_pos())
        {
            let mouse_position = ui.plot_from_screen(mouse_position);
            bbox.set_opposing_corner(mouse_position);
        }

        if ui.response().clicked_by(PointerButton::Secondary) {
            // delete bbox when right-clicking
            if self.box_in_drawing.is_some() {
                self.box_in_drawing.take();
            } else if let Some(mouse_position) = ui.response().hover_pos() {
                let mouse_position = ui.plot_from_screen(mouse_position);
                if !Self::delete_box_from(mouse_position, &mut self.model_boxes) {
                    Self::delete_box_from(mouse_position, &mut self.bounding_boxes);
                }
            }
        }
    }

    fn delete_box_from(mouse_position: PlotPoint, bbox_list: &mut Vec<BoundingBox>) -> bool {
        if let Some(clicked_bbox_index) = bbox_list
            .iter()
            .enumerate()
            .filter(|(_, bbox)| bbox.contains(mouse_position))
            .min_by(|(_, bbox1), (_, bbox2)| bbox1.rect().area().total_cmp(&bbox2.rect().area()))
            .map(|(idx, _)| idx)
        {
            bbox_list.remove(clicked_bbox_index);
            return true;
        }
        false
    }

    pub fn load_new_image_with_labels(
        &mut self,
        paths: Paths,
        annotations: Vec<BoundingBox>,
    ) -> Result<()> {
        if let (true, Some(paths)) = (self.auto_save_on_next_image, &self.current_paths) {
            // export current bboxes
            let mut annotations: Vec<AnnotationFormat> = self
                .bounding_boxes
                .drain(..)
                .map(|bbox| bbox.into())
                .collect();
            if self.use_model_annotations {
                annotations.extend(self.model_boxes.drain(..).map(|bbox| bbox.into()));
            }
            let annotations = serde_json::to_string_pretty(&annotations)?;

            let mut file = File::create(&paths.label_path)?;
            file.write_all(annotations.as_bytes())?;
        }

        self.bounding_boxes.clear();
        self.model_boxes.clear();

        if paths.label_path.exists() {
            let existing_annotations = fs::read_to_string(&paths.label_path)?;
            let mut existing_annotations: Vec<AnnotationFormat> =
                serde_json::from_str(&existing_annotations)?;
            self.bounding_boxes = existing_annotations
                .drain(..)
                .map(|annotation| annotation.into())
                .collect();
        } else {
            self.model_boxes = annotations;
        }

        self.texture_id = None;
        self.current_paths = Some(paths);

        Ok(())
    }
}
