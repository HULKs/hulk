use std::{
    collections::BTreeMap,
    f32::consts::FRAC_PI_4,
    fs::{read, write, File},
    io::Write,
    path::Path,
    sync::Arc,
    time::SystemTime,
};

use calibration::{corrections::Corrections, lines::Lines, measurement, problem::Metric, solve};
use color_eyre::eyre::eyre;
use communication::client::{Cycler, CyclerOutput, Output};
use eframe::{
    egui::{
        DragValue, Key, Label, Modifiers, Response, ScrollArea, Sense, TextureOptions, Ui, Widget,
    },
    epaint::{Color32, Rect, Stroke},
};
use egui_extras::RetainedImage;
use glob::glob;
use nalgebra::{point, vector, Point2, Rotation3, Similarity2, Vector2};
use serde::{Deserialize, Serialize};
use serde_json::{from_reader, to_writer_pretty};
use types::{
    field_marks_from_field_dimensions, CameraMatrix, CameraPosition, FieldDimensions, Line, Line2,
};

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
    image_buffer: ImageBuffer,
    camera_matrix_buffer: ValueBuffer,
    crossings: Crossings,
    measurements: BTreeMap<String, ImageWithMeasurement>,
    current_id: Option<String>,
    correction_in_robot_roll: f32,
    correction_in_robot_pitch: f32,
    correction_in_robot_yaw: f32,
    correction_in_camera_top_roll: f32,
    correction_in_camera_top_pitch: f32,
    correction_in_camera_top_yaw: f32,
    correction_in_camera_bottom_roll: f32,
    correction_in_camera_bottom_pitch: f32,
    correction_in_camera_bottom_yaw: f32,
}

const CAPTURE_POSITION: CameraPosition = CameraPosition::Bottom;

impl Panel for ManualCrossingMarker {
    const NAME: &'static str = "Manual Crossing Marker";

    fn new(nao: Arc<Nao>, _value: Option<&serde_json::Value>) -> Self {
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
            image_buffer,
            camera_matrix_buffer,
            crossings: Default::default(),
            measurements: BTreeMap::new(),
            current_id: None,
            correction_in_robot_roll: Default::default(),
            correction_in_robot_pitch: Default::default(),
            correction_in_robot_yaw: Default::default(),
            correction_in_camera_top_roll: Default::default(),
            correction_in_camera_top_pitch: Default::default(),
            correction_in_camera_top_yaw: Default::default(),
            correction_in_camera_bottom_roll: Default::default(),
            correction_in_camera_bottom_pitch: Default::default(),
            correction_in_camera_bottom_yaw: Default::default(),
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
                                            measurement.measurement.crossings.border_line_0?,
                                            measurement.measurement.crossings.border_line_1?,
                                        ),
                                        goal_box_line: Line(
                                            measurement.measurement.crossings.goal_box_line_0?,
                                            measurement.measurement.crossings.goal_box_line_1?,
                                        ),
                                        connecting_line: Line(
                                            measurement.measurement.crossings.connecting_line_0?,
                                            measurement.measurement.crossings.connecting_line_1?,
                                        ),
                                    },
                                })
                            })
                            .collect::<Vec<_>>();
                        let field_dimensions = FieldDimensions {
                            ball_radius: 0.05,
                            length: 9.0,
                            width: 6.0,
                            line_width: 0.05,
                            penalty_marker_size: 0.1,
                            goal_box_area_length: 0.6,
                            goal_box_area_width: 2.2,
                            penalty_area_length: 1.65,
                            penalty_area_width: 4.0,
                            penalty_marker_distance: 1.3,
                            center_circle_diameter: 1.5,
                            border_strip_width: 0.7,
                            goal_inner_width: 1.5,
                            goal_post_diameter: 0.1,
                            goal_depth: 0.5,
                        };
                        let (corrections, metrics) = solve(
                            Corrections {
                                correction_in_robot: Rotation3::from_euler_angles(
                                    self.correction_in_robot_roll,
                                    self.correction_in_robot_pitch,
                                    self.correction_in_robot_yaw,
                                ),
                                correction_in_camera_top: Rotation3::from_euler_angles(
                                    self.correction_in_camera_top_roll,
                                    self.correction_in_camera_top_pitch,
                                    self.correction_in_camera_top_yaw,
                                ),
                                correction_in_camera_bottom: Rotation3::from_euler_angles(
                                    self.correction_in_camera_bottom_roll,
                                    self.correction_in_camera_bottom_pitch,
                                    self.correction_in_camera_bottom_yaw,
                                ),
                            },
                            measurements.clone(),
                            field_dimensions,
                        );
                        println!("corrections: {corrections:?}");
                        // println!("metrics: {metrics:?}");
                        render_lines_in_field(
                            "solved.html",
                            &measurements,
                            &corrections,
                            &metrics,
                        );
                    }
                    if ui.button("Reset Rotations").clicked() {
                        self.correction_in_robot_roll = 0.0;
                        self.correction_in_robot_pitch = 0.0;
                        self.correction_in_robot_yaw = 0.0;
                        self.correction_in_camera_top_roll = 0.0;
                        self.correction_in_camera_top_pitch = 0.0;
                        self.correction_in_camera_top_yaw = 0.0;
                        self.correction_in_camera_bottom_roll = 0.0;
                        self.correction_in_camera_bottom_pitch = 0.0;
                        self.correction_in_camera_bottom_yaw = 0.0;
                    }
                    ui.add(DragValue::new(&mut self.correction_in_robot_roll).speed(0.1));
                    ui.add(DragValue::new(&mut self.correction_in_robot_pitch).speed(0.1));
                    ui.add(DragValue::new(&mut self.correction_in_robot_yaw).speed(0.1));
                    ui.add(DragValue::new(&mut self.correction_in_camera_top_roll).speed(0.1));
                    ui.add(DragValue::new(&mut self.correction_in_camera_top_pitch).speed(0.1));
                    ui.add(DragValue::new(&mut self.correction_in_camera_top_yaw).speed(0.1));
                    ui.add(DragValue::new(&mut self.correction_in_camera_bottom_roll).speed(0.1));
                    ui.add(DragValue::new(&mut self.correction_in_camera_bottom_pitch).speed(0.1));
                    ui.add(DragValue::new(&mut self.correction_in_camera_bottom_yaw).speed(0.1));
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

fn render_lines_in_field(
    file_name: impl AsRef<Path>,
    measurements: &[measurement::Measurement],
    corrections: &Corrections,
    metrics: &[Metric],
) {
    let mut file = File::create(&file_name).unwrap();
    writeln!(
        file,
        "<!DOCTYPE html><html><head><title>{:?}</title></head><body>",
        file_name.as_ref(),
    )
    .unwrap();
    for measurement in measurements {
        draw_projected_lines(&mut file, measurement, corrections);
    }
    writeln!(file, "</body></html>").unwrap();
    // write!(
    //     file,
    //     "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" style=\"background-color: green;\">",
    //     -(field_dimensions.length) / 2.0 - field_dimensions.border_strip_width,
    //     -(field_dimensions.width) / 2.0 - field_dimensions.border_strip_width,
    //     field_dimensions.length + 2.0 * field_dimensions.border_strip_width,
    //     field_dimensions.width + 2.0 * field_dimensions.border_strip_width,
    // )?;
    // write!(file, "<g transform=\"scale(1, -1)\">")?;
    // for field_mark in field_marks.iter() {
    //     match field_mark {
    //         FieldMark::Line { line, direction: _ } => {
    //             draw_line(&mut file, *line, "white", field_dimensions.line_width)?;
    //         }
    //         FieldMark::Circle { center, radius } => {
    //             draw_circle(
    //                 &mut file,
    //                 *center,
    //                 *radius,
    //                 "white",
    //                 field_dimensions.line_width,
    //                 "none",
    //             )?;
    //         }
    //     };
    // }
    // for (index, line) in lines.iter().enumerate() {
    //     draw_line(&mut file, *line, "red", field_dimensions.line_width / 2.0)?;
    //     draw_text(
    //         &mut file,
    //         line.center(),
    //         format!("{index}"),
    //         "black",
    //         0.2,
    //         0.0,
    //     )?;
    // }
    // write!(file, "</g>")?;
    // write!(file, "</svg>")?;
}

fn draw_projected_lines(
    file: &mut impl Write,
    measurement: &measurement::Measurement,
    corrections: &Corrections,
) {
    let projected_lines = measurement.lines.to_projected(&measurement.matrix).unwrap();
    let corrected_projected_lines = {
        let corrected = measurement.matrix.to_corrected(
            corrections.correction_in_robot,
            match measurement.position {
                CameraPosition::Top => corrections.correction_in_camera_top,
                CameraPosition::Bottom => corrections.correction_in_camera_bottom,
            },
        );

        let projected_lines = measurement.lines.to_projected(&corrected).unwrap();

        projected_lines
    };
    let minimum_x = projected_lines
        .border_line
        .0
        .x
        .min(projected_lines.border_line.1.x)
        .min(projected_lines.goal_box_line.0.x)
        .min(projected_lines.goal_box_line.1.x)
        .min(projected_lines.connecting_line.0.x)
        .min(projected_lines.connecting_line.1.x)
        .min(corrected_projected_lines.border_line.0.x)
        .min(corrected_projected_lines.border_line.1.x)
        .min(corrected_projected_lines.goal_box_line.0.x)
        .min(corrected_projected_lines.goal_box_line.1.x)
        .min(corrected_projected_lines.connecting_line.0.x)
        .min(corrected_projected_lines.connecting_line.1.x);
    let maximum_x = projected_lines
        .border_line
        .0
        .x
        .max(projected_lines.border_line.1.x)
        .max(projected_lines.goal_box_line.0.x)
        .max(projected_lines.goal_box_line.1.x)
        .max(projected_lines.connecting_line.0.x)
        .max(projected_lines.connecting_line.1.x)
        .max(corrected_projected_lines.border_line.0.x)
        .max(corrected_projected_lines.border_line.1.x)
        .max(corrected_projected_lines.goal_box_line.0.x)
        .max(corrected_projected_lines.goal_box_line.1.x)
        .max(corrected_projected_lines.connecting_line.0.x)
        .max(corrected_projected_lines.connecting_line.1.x);
    let minimum_y = projected_lines
        .border_line
        .0
        .y
        .min(projected_lines.border_line.1.y)
        .min(projected_lines.goal_box_line.0.y)
        .min(projected_lines.goal_box_line.1.y)
        .min(projected_lines.connecting_line.0.y)
        .min(projected_lines.connecting_line.1.y)
        .min(corrected_projected_lines.border_line.0.y)
        .min(corrected_projected_lines.border_line.1.y)
        .min(corrected_projected_lines.goal_box_line.0.y)
        .min(corrected_projected_lines.goal_box_line.1.y)
        .min(corrected_projected_lines.connecting_line.0.y)
        .min(corrected_projected_lines.connecting_line.1.y);
    let maximum_y = projected_lines
        .border_line
        .0
        .y
        .max(projected_lines.border_line.1.y)
        .max(projected_lines.goal_box_line.0.y)
        .max(projected_lines.goal_box_line.1.y)
        .max(projected_lines.connecting_line.0.y)
        .max(projected_lines.connecting_line.1.y)
        .max(corrected_projected_lines.border_line.0.y)
        .max(corrected_projected_lines.border_line.1.y)
        .max(corrected_projected_lines.goal_box_line.0.y)
        .max(corrected_projected_lines.goal_box_line.1.y)
        .max(corrected_projected_lines.connecting_line.0.y)
        .max(corrected_projected_lines.connecting_line.1.y);
    write!(
        file,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" style=\"border: 1px solid #888888; width: 200px;\">",
        minimum_x - (maximum_x - minimum_x) * 0.05,
        minimum_y - (maximum_y - minimum_y) * 0.05,
        maximum_x - minimum_x + (maximum_x - minimum_x) * 0.1,
        maximum_y - minimum_y + (maximum_y - minimum_y) * 0.1,
    ).unwrap();
    draw_line(file, projected_lines.border_line, "red", 0.05);
    draw_line(file, projected_lines.goal_box_line, "red", 0.05);
    draw_line(file, projected_lines.connecting_line, "red", 0.05);
    draw_line(file, corrected_projected_lines.border_line, "green", 0.05);
    draw_line(file, corrected_projected_lines.goal_box_line, "green", 0.05);
    draw_line(
        file,
        corrected_projected_lines.connecting_line,
        "green",
        0.05,
    );
    write!(file, "</svg>").unwrap();
}

fn draw_rect(
    file: &mut impl Write,
    point_upper_left: Point2<f32>,
    size: Vector2<f32>,
    stroke_color: &str,
    stroke_width: f32,
) {
    write!(
        file,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" stroke=\"{}\" stroke-width=\"{}\" fill=\"none\" />",
        point_upper_left.x,
        point_upper_left.y,
        size.x,
        size.y,
        stroke_color,
        stroke_width,
    ).unwrap();
}

fn draw_line(file: &mut impl Write, line: Line2, stroke_color: &str, stroke_width: f32) {
    write!(
        file,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\" fill=\"none\" />",
        line.0.x,
        line.0.y,
        line.1.x,
        line.1.y,
        stroke_color,
        stroke_width,
    ).unwrap();
}

fn draw_circle(
    file: &mut impl Write,
    center: Point2<f32>,
    radius: f32,
    stroke_color: &str,
    stroke_width: f32,
    fill_color: &str,
) {
    write!(
        file,
        "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" stroke=\"{}\" stroke-width=\"{}\" fill=\"{}\" />",
        center.x, center.y, radius, stroke_color, stroke_width, fill_color,
    )
    .unwrap();
}

fn draw_text(
    file: &mut impl Write,
    center: Point2<f32>,
    text: String,
    fill_color: &str,
    font_size: f32,
    rotation_angle: f32,
) {
    write!(
        file,
        "<text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"{}\" transform=\"rotate({}, {}, {})\" text-anchor=\"middle\">{}</text>",
        center.x, center.y, fill_color, font_size, rotation_angle.to_degrees(), center.x, center.y, text,
    ).unwrap();
}
