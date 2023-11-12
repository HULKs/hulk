use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use crate::{annotation::AnnotationFormat, boundingbox::BoundingBox, classes::Classes, yolo::Yolo};
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

struct Paths {
    pub image_path: PathBuf,
    pub label_path: PathBuf,
}

pub struct LabelWidget {
    current_paths: Option<Paths>,
    texture_id: Option<TextureHandle>,
    selected_class: Classes,
    yolo_model: Yolo,
    bounding_boxes: Vec<BoundingBox>,
    box_in_drawing: Option<BoundingBox>,
}

impl LabelWidget {
    pub fn new() -> Self {
        Self {
            current_paths: None,
            texture_id: None,
            selected_class: Classes::Robot,
            yolo_model: Yolo::try_from_onnx("best.onnx".into()),
            bounding_boxes: Vec::new(),
            box_in_drawing: None,
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
            self.current_paths.as_ref().map(|paths| {
                ui.label(paths.image_path.display().to_string());
            });
            ComboBox::from_id_source("class-selector")
                .selected_text(format!("{:?}", self.selected_class))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.selected_class, Classes::Robot, "Robot");
                    ui.selectable_value(&mut self.selected_class, Classes::Ball, "Ball");
                    ui.selectable_value(&mut self.selected_class, Classes::GoalPost, "Goal Post");
                    ui.selectable_value(
                        &mut self.selected_class,
                        Classes::PenaltySpot,
                        "Penalty Spot",
                    );
                });
            Plot::new("image-plot")
                .view_aspect(640. / 480.)
                .show_axes([false, false])
                .show_grid([false, false])
                .set_margin_fraction(Vec2::ZERO)
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
                if let Some(clicked_bbox_index) = self
                    .bounding_boxes
                    .iter()
                    .enumerate()
                    .filter(|(_, bbox)| bbox.contains(mouse_position))
                    .min_by(|(_, bbox1), (_, bbox2)| {
                        bbox1.rect().area().total_cmp(&bbox2.rect().area())
                    })
                    .map(|(idx, _)| idx)
                {
                    self.bounding_boxes.remove(clicked_bbox_index);
                }
            }
        }
    }

    pub fn load_new_image_with_labels(
        &mut self,
        image_path: PathBuf,
        label_path: PathBuf,
    ) -> Result<()> {
        if let Some(paths) = &self.current_paths {
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

        if label_path.exists() {
            let existing_annotations = fs::read_to_string(&label_path)?;
            let mut existing_annotations: Vec<AnnotationFormat> =
                serde_json::from_str(&existing_annotations)?;
            self.bounding_boxes = existing_annotations
                .drain(..)
                .map(|annotation| annotation.into())
                .collect();
        }

        self.texture_id = None;
        self.current_paths = Some(Paths {
            image_path,
            label_path,
        });

        Ok(())
    }
}
