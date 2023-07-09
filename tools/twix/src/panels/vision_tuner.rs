use communication::client::Cycler;
use eframe::egui::{ComboBox, Response, Slider, Ui, Widget};
use nalgebra::{Isometry2, Rotation2, Translation2};
use serde_json::{to_value, Value};
use std::{
    f32::consts::PI,
    fmt::{self, Display, Formatter},
    sync::Arc,
};
use types::interpolated::Interpolated;

use crate::{nao::Nao, panel::Panel, value_buffer::ValueBuffer};

pub struct VisionTunerPanel {
    nao: Arc<Nao>,
    cycler: Cycler,
    position: Position,
    buffers: Buffers,
}

impl Panel for VisionTunerPanel {
    const NAME: &'static str = "Vision Tuner";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let cycler = Cycler::VisionTop;
        let buffers = Buffers::from(&nao, cycler);

        Self {
            nao,
            cycler,
            position: Position::FirstHalfOwnHalfTowardsOwnGoal,
            buffers,
        }
    }
}

impl Widget for &mut VisionTunerPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut vertical_edge_threshold = match self
            .buffers
            .vertical_edge_threshold_buffer
            .parse_latest::<Interpolated>()
        {
            Ok(vertical_edge_threshold) => vertical_edge_threshold,
            Err(error) => {
                return ui.label(format!("{error:#?}"));
            }
        };
        let mut red_chromaticity_threshold = match self
            .buffers
            .red_chromaticity_threshold_buffer
            .parse_latest::<Interpolated>()
        {
            Ok(red_chromaticity_threshold) => red_chromaticity_threshold,
            Err(error) => {
                return ui.label(format!("{error:#?}"));
            }
        };
        let mut blue_chromaticity_threshold = match self
            .buffers
            .blue_chromaticity_threshold_buffer
            .parse_latest::<Interpolated>()
        {
            Ok(blue_chromaticity_threshold) => blue_chromaticity_threshold,
            Err(error) => {
                return ui.label(format!("{error:#?}"));
            }
        };
        let mut lower_green_chromaticity_threshold = match self
            .buffers
            .lower_green_chromaticity_threshold_buffer
            .parse_latest::<Interpolated>()
        {
            Ok(lower_green_chromaticity_threshold) => lower_green_chromaticity_threshold,
            Err(error) => {
                return ui.label(format!("{error:#?}"));
            }
        };
        let mut upper_green_chromaticity_threshold = match self
            .buffers
            .upper_green_chromaticity_threshold_buffer
            .parse_latest::<Interpolated>()
        {
            Ok(upper_green_chromaticity_threshold) => upper_green_chromaticity_threshold,
            Err(error) => {
                return ui.label(format!("{error:#?}"));
            }
        };
        let mut green_luminance_threshold = match self
            .buffers
            .green_luminance_threshold_buffer
            .parse_latest::<Interpolated>()
        {
            Ok(green_luminance_threshold) => green_luminance_threshold,
            Err(error) => {
                return ui.label(format!("{error:#?}"));
            }
        };

        ui.style_mut().spacing.slider_width = ui.available_size().x - 250.0;
        ui.vertical(|ui| {
            add_selector_row(
                ui,
                &self.nao,
                &mut self.cycler,
                &mut self.position,
                &mut self.buffers,
            );

            let value = get_value_from_interpolated(self.position, &mut vertical_edge_threshold);
            if ui
                .add(
                    Slider::new(value, 0.0..=255.0)
                        .text("vertical_edge_threshold")
                        .smart_aim(false),
                )
                .changed()
            {
                self.nao.update_parameter_value(
                    get_vertical_edge_threshold_path(self.cycler),
                    to_value(vertical_edge_threshold).unwrap(),
                );
            }

            let value = get_value_from_interpolated(self.position, &mut red_chromaticity_threshold);
            if ui
                .add(
                    Slider::new(value, 0.0..=1.0)
                        .text("red_chromaticity_threshold")
                        .smart_aim(false),
                )
                .changed()
            {
                self.nao.update_parameter_value(
                    get_red_chromaticity_threshold_path(self.cycler),
                    to_value(red_chromaticity_threshold).unwrap(),
                );
            }

            let value =
                get_value_from_interpolated(self.position, &mut blue_chromaticity_threshold);
            if ui
                .add(
                    Slider::new(value, 0.0..=1.0)
                        .text("blue_chromaticity_threshold")
                        .smart_aim(false),
                )
                .changed()
            {
                self.nao.update_parameter_value(
                    get_blue_chromaticity_threshold_path(self.cycler),
                    to_value(blue_chromaticity_threshold).unwrap(),
                );
            }
            let value =
                get_value_from_interpolated(self.position, &mut lower_green_chromaticity_threshold);
            if ui
                .add(
                    Slider::new(value, 0.0..=1.0)
                        .text("lower_green_chromaticity_threshold")
                        .smart_aim(false),
                )
                .changed()
            {
                self.nao.update_parameter_value(
                    get_lower_green_chromaticity_threshold_path(self.cycler),
                    to_value(lower_green_chromaticity_threshold).unwrap(),
                );
            }
            let value =
                get_value_from_interpolated(self.position, &mut upper_green_chromaticity_threshold);
            if ui
                .add(
                    Slider::new(value, 0.0..=1.0)
                        .text("upper_green_chromaticity_threshold")
                        .smart_aim(false),
                )
                .changed()
            {
                self.nao.update_parameter_value(
                    get_upper_green_chromaticity_threshold_path(self.cycler),
                    to_value(upper_green_chromaticity_threshold).unwrap(),
                );
            }
            let value = get_value_from_interpolated(self.position, &mut green_luminance_threshold);
            if ui
                .add(
                    Slider::new(value, 0.0..=255.0)
                        .text("green_luminance_threshold")
                        .smart_aim(false),
                )
                .changed()
            {
                self.nao.update_parameter_value(
                    get_green_luminance_threshold_path(self.cycler),
                    to_value(green_luminance_threshold).unwrap(),
                );
            }
        })
        .response
    }
}

struct Buffers {
    vertical_edge_threshold_buffer: ValueBuffer,
    red_chromaticity_threshold_buffer: ValueBuffer,
    blue_chromaticity_threshold_buffer: ValueBuffer,
    lower_green_chromaticity_threshold_buffer: ValueBuffer,
    upper_green_chromaticity_threshold_buffer: ValueBuffer,
    green_luminance_threshold_buffer: ValueBuffer,
}

impl Buffers {
    fn from(nao: &Nao, cycler: Cycler) -> Self {
        let vertical_edge_threshold_buffer =
            nao.subscribe_parameter(get_vertical_edge_threshold_path(cycler));
        let red_chromaticity_threshold_buffer =
            nao.subscribe_parameter(get_red_chromaticity_threshold_path(cycler));
        let blue_chromaticity_threshold_buffer =
            nao.subscribe_parameter(get_blue_chromaticity_threshold_path(cycler));
        let lower_green_chromaticity_threshold_buffer =
            nao.subscribe_parameter(get_lower_green_chromaticity_threshold_path(cycler));
        let upper_green_chromaticity_threshold_buffer =
            nao.subscribe_parameter(get_upper_green_chromaticity_threshold_path(cycler));
        let green_luminance_threshold_buffer =
            nao.subscribe_parameter(get_green_luminance_threshold_path(cycler));

        Self {
            vertical_edge_threshold_buffer,
            red_chromaticity_threshold_buffer,
            blue_chromaticity_threshold_buffer,
            lower_green_chromaticity_threshold_buffer,
            upper_green_chromaticity_threshold_buffer,
            green_luminance_threshold_buffer,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::enum_variant_names)]
enum Position {
    FirstHalfOwnHalfTowardsOwnGoal,
    FirstHalfOwnHalfAwayOwnGoal,
    FirstHalfOpponentHalfTowardsOwnGoal,
    FirstHalfOpponentHalfAwayOwnGoal,
}

impl Display for Position {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Position::FirstHalfOwnHalfTowardsOwnGoal => {
                formatter.write_str("Own Half Towards ;) Own Goal")
            }
            Position::FirstHalfOwnHalfAwayOwnGoal => {
                formatter.write_str("Own Half Away ;) Own Goal")
            }
            Position::FirstHalfOpponentHalfTowardsOwnGoal => {
                formatter.write_str("Opponent Half Towards ;) Own Goal")
            }
            Position::FirstHalfOpponentHalfAwayOwnGoal => {
                formatter.write_str("Opponent Half Away ;) Own Goal")
            }
        }
    }
}

fn add_selector_row(
    ui: &mut Ui,
    nao: &Nao,
    cycler: &mut Cycler,
    position: &mut Position,
    buffers: &mut Buffers,
) -> Response {
    ui.horizontal(|ui| {
        add_vision_cycler_selector(ui, nao, cycler, buffers);
        let response = add_position_selector(ui, position);
        if response.changed() {
            let injected_robot_to_field_translation = match position {
                Position::FirstHalfOwnHalfTowardsOwnGoal
                | Position::FirstHalfOwnHalfAwayOwnGoal => Translation2::new(-3.0, 0.0),
                Position::FirstHalfOpponentHalfTowardsOwnGoal
                | Position::FirstHalfOpponentHalfAwayOwnGoal => Translation2::new(3.0, 0.0),
            };
            let injected_robot_to_field_rotation = match position {
                Position::FirstHalfOwnHalfTowardsOwnGoal
                | Position::FirstHalfOpponentHalfTowardsOwnGoal => Rotation2::new(0.0),
                Position::FirstHalfOwnHalfAwayOwnGoal
                | Position::FirstHalfOpponentHalfAwayOwnGoal => Rotation2::new(PI),
            };
            let injected_robot_to_field = Isometry2::from_parts(
                injected_robot_to_field_translation,
                injected_robot_to_field_rotation.into(),
            );
            let value = to_value(injected_robot_to_field).unwrap();
            println!("update");
            nao.update_parameter_value(
                "injected_robot_to_field_of_home_after_coin_toss_before_second_half",
                value,
            );
        }
    })
    .response
}

fn add_vision_cycler_selector(
    ui: &mut Ui,
    nao: &Nao,
    cycler: &mut Cycler,
    buffers: &mut Buffers,
) -> Response {
    let mut changed = false;
    let response = ComboBox::from_label("Cycler")
        .selected_text(format!("{:?}", cycler))
        .show_ui(ui, |ui| {
            if ui
                .selectable_value(cycler, Cycler::VisionTop, "VisionTop")
                .clicked()
            {
                *cycler = Cycler::VisionTop;
                changed = true;
            };
            if ui
                .selectable_value(cycler, Cycler::VisionBottom, "VisionBottom")
                .clicked()
            {
                *cycler = Cycler::VisionBottom;
                changed = true;
            };
        })
        .response;
    if changed {
        *buffers = Buffers::from(nao, *cycler);
    }
    response
}

fn add_position_selector(ui: &mut Ui, position: &mut Position) -> Response {
    let mut position_selection_changed = false;
    let mut combo_box = ComboBox::from_label("Position")
        .selected_text(format!("{}", position))
        .show_ui(ui, |ui| {
            if ui
                .selectable_value(
                    position,
                    Position::FirstHalfOwnHalfTowardsOwnGoal,
                    format!("{}", Position::FirstHalfOwnHalfTowardsOwnGoal),
                )
                .clicked()
            {
                position_selection_changed = true;
            }
            if ui
                .selectable_value(
                    position,
                    Position::FirstHalfOwnHalfAwayOwnGoal,
                    format!("{}", Position::FirstHalfOwnHalfAwayOwnGoal),
                )
                .clicked()
            {
                position_selection_changed = true;
            }
            if ui
                .selectable_value(
                    position,
                    Position::FirstHalfOpponentHalfTowardsOwnGoal,
                    format!("{}", Position::FirstHalfOpponentHalfTowardsOwnGoal),
                )
                .clicked()
            {
                position_selection_changed = true;
            }
            if ui
                .selectable_value(
                    position,
                    Position::FirstHalfOpponentHalfAwayOwnGoal,
                    format!("{}", Position::FirstHalfOpponentHalfAwayOwnGoal),
                )
                .clicked()
            {
                position_selection_changed = true;
            }
        });
    if position_selection_changed {
        combo_box.response.mark_changed()
    }
    combo_box.response
}

fn get_vertical_edge_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "image_segmenter.vision_top.vertical_edge_threshold",
        Cycler::VisionBottom => "image_segmenter.vision_bottom.vertical_edge_threshold",
        _ => panic!("not implemented"),
    }
}

fn get_red_chromaticity_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.red_chromaticity_threshold",
        Cycler::VisionBottom => "field_color_detection.vision_bottom.red_chromaticity_threshold",
        _ => panic!("not implemented"),
    }
}

fn get_blue_chromaticity_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.blue_chromaticity_threshold",
        Cycler::VisionBottom => "field_color_detection.vision_bottom.blue_chromaticity_threshold",
        _ => panic!("not implemented"),
    }
}

fn get_lower_green_chromaticity_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.lower_green_chromaticity_threshold",
        Cycler::VisionBottom => {
            "field_color_detection.vision_bottom.lower_green_chromaticity_threshold"
        }
        _ => panic!("not implemented"),
    }
}

fn get_upper_green_chromaticity_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.upper_green_chromaticity_threshold",
        Cycler::VisionBottom => {
            "field_color_detection.vision_bottom.upper_green_chromaticity_threshold"
        }
        _ => panic!("not implemented"),
    }
}

fn get_green_luminance_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.green_luminance_threshold",
        Cycler::VisionBottom => "field_color_detection.vision_bottom.green_luminance_threshold",
        _ => panic!("not implemented"),
    }
}

fn get_value_from_interpolated(position: Position, interpolated: &mut Interpolated) -> &mut f32 {
    match position {
        Position::FirstHalfOwnHalfTowardsOwnGoal => {
            &mut interpolated.first_half_own_half_towards_own_goal
        }
        Position::FirstHalfOwnHalfAwayOwnGoal => {
            &mut interpolated.first_half_own_half_away_own_goal
        }
        Position::FirstHalfOpponentHalfTowardsOwnGoal => {
            &mut interpolated.first_half_opponent_half_towards_own_goal
        }
        Position::FirstHalfOpponentHalfAwayOwnGoal => {
            &mut interpolated.first_half_opponent_half_away_own_goal
        }
    }
}
