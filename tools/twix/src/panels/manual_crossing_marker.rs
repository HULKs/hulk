use std::{
    collections::BTreeMap,
    fs::{read, write, File},
    sync::Arc,
    time::SystemTime,
};

use calibration::{lines::Lines, measurement, solve};
use color_eyre::eyre::eyre;
use communication::client::{Cycler, CyclerOutput, Output};
use eframe::{
    egui::{Key, Label, Modifiers, Response, ScrollArea, Sense, TextureOptions, Ui, Widget},
    epaint::{Color32, Rect, Stroke},
};
use egui_extras::RetainedImage;
use glob::glob;
use nalgebra::{point, vector, Point2, Similarity2};
use serde::{Deserialize, Serialize};
use serde_json::{from_reader, to_writer_pretty};
use types::{CameraMatrix, CameraPosition, Line};

use crate::{
    image_buffer::ImageBuffer,
    nao::Nao,
    panel::Panel,
    twix_painter::{CoordinateSystem, TwixPainter},
    value_buffer::ValueBuffer,
};

#[derive(Debug, Deserialize, Serialize)]
struct ImageWithMeasurement {
    image: Vec<u8>,
    measurement: Measurement,
}

#[derive(Debug, Deserialize, Serialize)]
struct Measurement {
    position: CameraPosition,
    camera_matrix: CameraMatrix,
    crossings: Crossings,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Crossings {
    border_line_0: Option<Point2<f32>>,
    border_line_1: Option<Point2<f32>>,
    goal_box_line_0: Option<Point2<f32>>,
    goal_box_line_1: Option<Point2<f32>>,
    connecting_line_0: Option<Point2<f32>>,
    connecting_line_1: Option<Point2<f32>>,
}

pub struct ManualCrossingMarker {
    _nao: Arc<Nao>,
    field_dimensions_buffer: ValueBuffer,
    image_buffer: ImageBuffer,
    camera_matrix_buffer: ValueBuffer,
    crossings: Crossings,
    measurements: BTreeMap<String, ImageWithMeasurement>,
    current_id: Option<String>,
}

const CAPTURE_POSITION: CameraPosition = CameraPosition::Bottom;

impl Panel for ManualCrossingMarker {
    const NAME: &'static str = "Manual Crossing Marker";

    fn new(nao: Arc<Nao>, _value: Option<&serde_json::Value>) -> Self {
        let field_dimensions_buffer = nao.subscribe_parameter("field_dimensions");
        let image_buffer = nao.subscribe_image(CyclerOutput {
            cycler: match CAPTURE_POSITION {
                CameraPosition::Top => Cycler::VisionTop,
                CameraPosition::Bottom => Cycler::VisionBottom,
            },
            output: Output::Main {
                path: "image.jpeg".to_string(),
            },
        });
        let camera_matrix_buffer = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::Control,
            output: Output::Main {
                path: format!(
                    "camera_matrices.{}",
                    match CAPTURE_POSITION {
                        CameraPosition::Top => "top",
                        CameraPosition::Bottom => "bottom",
                    }
                ),
            },
        });
        Self {
            _nao: nao,
            field_dimensions_buffer,
            image_buffer,
            camera_matrix_buffer,
            crossings: Default::default(),
            measurements: BTreeMap::new(),
            current_id: None,
        }
    }
}

impl Widget for &mut ManualCrossingMarker {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.horizontal_top(|ui| {
            self.show_image(ui);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("Capture").clicked() {
                        let id = format!(
                            "{}",
                            SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                        );
                        let image_path = format!("measurements/{id}.jpg");
                        let json_path = format!("measurements/{id}.json");
                        let image_data = self.image_buffer.get_latest().unwrap();
                        let image_raw = bincode::deserialize::<Vec<u8>>(&image_data).unwrap();
                        let camera_matrix = self.camera_matrix_buffer.parse_latest().unwrap();
                        let measurement = Measurement {
                            position: CAPTURE_POSITION,
                            camera_matrix,
                            crossings: Default::default(),
                        };
                        println!("Clicked! {image_path} {json_path} {measurement:?}");
                        write(image_path, &image_raw).unwrap();
                        let json_file = File::create(json_path).unwrap();
                        to_writer_pretty(json_file, &measurement).unwrap();
                    }
                    if ui.button("Load").clicked() {
                        self.measurements = glob("measurements/*.json")
                            .unwrap()
                            .map(|entry| {
                                let json_path = entry.unwrap();
                                let image_path = json_path.with_extension("jpg");
                                let id =
                                    json_path.file_stem().unwrap().to_str().unwrap().to_string();
                                let json_file = File::open(json_path).unwrap();
                                let image = read(image_path).unwrap();
                                let measurement = from_reader(json_file).unwrap();
                                (id, ImageWithMeasurement { image, measurement })
                            })
                            .collect();
                    }
                    if let Some(current_id) = &self.current_id {
                        if ui.button("Store").clicked() {
                            let image_with_measurement =
                                self.measurements.get_mut(current_id).unwrap();
                            image_with_measurement.measurement.crossings = self.crossings.clone();
                            let json_path = format!("measurements/{}.json", current_id);
                            let json_file = File::create(json_path).unwrap();
                            to_writer_pretty(json_file, &image_with_measurement.measurement)
                                .unwrap();
                        }
                    }
                    if ui.button("Optimize").clicked() {
                        let measurements = self
                            .measurements
                            .values()
                            .filter_map(|measurement| {
                                Some(measurement::Measurement {
                                    position: measurement.measurement.position,
                                    matrix: measurement.measurement.camera_matrix.clone(),
                                    lines: Lines {
                                        border_line: Line(
                                            self.crossings.border_line_0?,
                                            self.crossings.border_line_1?,
                                        ),
                                        goal_box_line: Line(
                                            self.crossings.goal_box_line_0?,
                                            self.crossings.goal_box_line_1?,
                                        ),
                                        connecting_line: Line(
                                            self.crossings.connecting_line_0?,
                                            self.crossings.connecting_line_1?,
                                        ),
                                    },
                                })
                            })
                            .collect::<Vec<_>>();
                        let field_dimensions = self.field_dimensions_buffer.parse_latest().unwrap();
                        solve(Default::default(), &measurements, field_dimensions);
                    }
                });
                self.show_list(ui);
            });
        })
        .response
    }
}

impl ManualCrossingMarker {
    fn show_list(&mut self, ui: &mut Ui) {
        let scroll_area = ScrollArea::vertical().auto_shrink([false; 2]);
        scroll_area.show(ui, |ui| {
            ui.vertical(|ui| {
                for (id, image_with_measurement) in self.measurements.iter() {
                    ui.push_id(id, |ui| {
                        if ui
                            .add(
                                Label::new(format!(
                                    "{id} ({:?}) ({}) {}",
                                    image_with_measurement.measurement.position,
                                    if image_with_measurement
                                        .measurement
                                        .crossings
                                        .border_line_0
                                        .is_some()
                                        || image_with_measurement
                                            .measurement
                                            .crossings
                                            .border_line_1
                                            .is_some()
                                        || image_with_measurement
                                            .measurement
                                            .crossings
                                            .goal_box_line_0
                                            .is_some()
                                        || image_with_measurement
                                            .measurement
                                            .crossings
                                            .goal_box_line_1
                                            .is_some()
                                        || image_with_measurement
                                            .measurement
                                            .crossings
                                            .connecting_line_0
                                            .is_some()
                                        || image_with_measurement
                                            .measurement
                                            .crossings
                                            .connecting_line_1
                                            .is_some()
                                    {
                                        "labeled"
                                    } else {
                                        ""
                                    },
                                    if self.current_id.is_some()
                                        && self.current_id.as_ref().unwrap() == id
                                    {
                                        " (selected)"
                                    } else {
                                        ""
                                    }
                                ))
                                .sense(Sense::click()),
                            )
                            .clicked()
                        {
                            println!("Clicked {id}");
                            self.current_id = Some(id.clone());
                            self.crossings = self
                                .measurements
                                .get(id)
                                .unwrap()
                                .measurement
                                .crossings
                                .clone();
                        }
                    });
                }
            });
        });
    }

    fn show_image(&mut self, ui: &mut Ui) {
        let Some(current_id) = &self.current_id else {
            ui.label("Select an image on the left");
            return;
        };
        let image = RetainedImage::from_image_bytes(
            "image",
            &self.measurements.get(current_id).unwrap().image,
        )
        .map_err(|error| eyre!("{error}"))
        .unwrap()
        .with_options(TextureOptions::NEAREST);
        let image_size = image.size_vec2();
        let width_scale = ui.available_width() / image_size.x;
        let height_scale = ui.available_height() / image_size.y;
        let scale = width_scale.min(height_scale);
        let image_response = image.show_scaled(ui, scale);
        let displayed_image_size = image_size * scale;
        let image_rect = Rect::from_min_size(image_response.rect.left_top(), displayed_image_size);
        let painter = TwixPainter::paint_at(ui, image_rect).with_camera(
            vector![640.0, 480.0],
            Similarity2::identity(),
            CoordinateSystem::LeftHand,
        );

        if let Some(position) = self.crossings.border_line_0 {
            self.show_cross(&painter, position);
        }
        if let Some(position) = self.crossings.border_line_1 {
            self.show_cross(&painter, position);
        }
        if let Some(position) = self.crossings.goal_box_line_0 {
            self.show_cross(&painter, position);
        }
        if let Some(position) = self.crossings.goal_box_line_1 {
            self.show_cross(&painter, position);
        }
        if let Some(position) = self.crossings.connecting_line_0 {
            self.show_cross(&painter, position);
        }
        if let Some(position) = self.crossings.connecting_line_1 {
            self.show_cross(&painter, position);
        }

        if let (Some(start), Some(end)) =
            (self.crossings.border_line_0, self.crossings.border_line_1)
        {
            self.show_line_segment(&painter, start, end, Color32::BROWN);
        }
        if let (Some(start), Some(end)) = (
            self.crossings.goal_box_line_0,
            self.crossings.goal_box_line_1,
        ) {
            self.show_line_segment(&painter, start, end, Color32::BLACK);
        }
        if let (Some(start), Some(end)) = (
            self.crossings.connecting_line_0,
            self.crossings.connecting_line_1,
        ) {
            self.show_line_segment(&painter, start, end, Color32::GRAY);
        }

        if let Some(position) = image_response.hover_pos() {
            let position = painter.transform_pixel_to_world(position);
            painter.line_segment(
                point![position.x, 0.0],
                point![position.x, 480.0],
                Stroke::new(0.5, Color32::WHITE),
            );
            painter.line_segment(
                point![0.0, position.y],
                point![640.0, position.y],
                Stroke::new(0.5, Color32::WHITE),
            );
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Q)) {
                self.crossings.border_line_0 = match self.crossings.border_line_0 {
                    Some(_) => None,
                    None => Some(position),
                };
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::W)) {
                self.crossings.border_line_1 = match self.crossings.border_line_1 {
                    Some(_) => None,
                    None => Some(position),
                };
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::E)) {
                self.crossings.goal_box_line_0 = match self.crossings.goal_box_line_0 {
                    Some(_) => None,
                    None => Some(position),
                };
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::R)) {
                self.crossings.goal_box_line_1 = match self.crossings.goal_box_line_1 {
                    Some(_) => None,
                    None => Some(position),
                };
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::T)) {
                self.crossings.connecting_line_0 = match self.crossings.connecting_line_0 {
                    Some(_) => None,
                    None => Some(position),
                };
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Z)) {
                self.crossings.connecting_line_1 = match self.crossings.connecting_line_1 {
                    Some(_) => None,
                    None => Some(position),
                };
            }
        }
    }

    fn show_cross(&self, painter: &TwixPainter, position: Point2<f32>) {
        painter.line_segment(
            point![position.x, position.y - 3.0],
            point![position.x, position.y + 3.0],
            Stroke::new(0.5, Color32::RED),
        );
        painter.line_segment(
            point![position.x - 3.0, position.y],
            point![position.x + 3.0, position.y],
            Stroke::new(0.5, Color32::RED),
        );
    }

    fn show_line_segment(
        &self,
        painter: &TwixPainter,
        start: Point2<f32>,
        end: Point2<f32>,
        color: Color32,
    ) {
        painter.line_segment(
            point![start.x, start.y],
            point![end.x, end.y],
            Stroke::new(0.5, color),
        );
    }
}
