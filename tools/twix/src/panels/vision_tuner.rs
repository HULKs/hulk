use std::{f32::consts::PI, sync::Arc};

use color_eyre::{eyre::Context, Result};
use eframe::egui::{ComboBox, Response, Slider, Ui, Widget};
use log::error;
use nalgebra::{Isometry2, Rotation2, Translation2};
use serde_json::{to_value, Value};

use communication::client::Cycler;
use types::{field_color::FieldColorFunction, interpolated::Interpolated};

use crate::{
    nao::Nao, panel::Panel, repository_parameters::RepositoryParameters, value_buffer::ValueBuffer,
};

pub struct VisionTunerPanel {
    nao: Arc<Nao>,
    repository_parameters: Result<RepositoryParameters>,
    cycler: Cycler,
    position: Option<Position>,
    parameters: Parameters<ValueBuffer, ValueBuffer>,
}

impl Panel for VisionTunerPanel {
    const NAME: &'static str = "Vision Tuner";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let cycler = Cycler::VisionTop;
        let parameters = Parameters::from(&nao, cycler);

        Self {
            nao,
            repository_parameters: RepositoryParameters::try_new(),
            cycler,
            position: None,
            parameters,
        }
    }
}

impl Widget for &mut VisionTunerPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut parameters = match self.parameters.parse_latest() {
            Ok(parameters) => parameters,
            Err(error) => return ui.label(format!("{error:#?}")),
        };

        ui.style_mut().spacing.slider_width = ui.available_size().x - 250.0;
        ui.vertical(|ui| {
            add_selector_row(
                ui,
                &self.nao,
                &self.repository_parameters,
                &mut self.cycler,
                &mut self.position,
                &mut parameters.function,
                &mut self.parameters,
            );

            if let Some(position) = self.position {
                let value =
                    get_value_from_interpolated(position, &mut parameters.vertical_edge_threshold);
                if ui
                    .add(
                        Slider::new(value, 0.0..=255.0)
                            .text("vertical_edge_threshold")
                            .smart_aim(false),
                    )
                    .changed()
                {
                    self.parameters
                        .vertical_edge_threshold
                        .update_parameter_value(
                            to_value(parameters.vertical_edge_threshold).unwrap(),
                        );
                }

                match parameters.function {
                    FieldColorFunction::GreenChromaticity => {
                        let value = get_value_from_interpolated(
                            position,
                            &mut parameters.red_chromaticity_threshold,
                        );
                        if ui
                            .add(
                                Slider::new(value, 0.0..=1.0)
                                    .text("red_chromaticity_threshold")
                                    .smart_aim(false),
                            )
                            .changed()
                        {
                            self.parameters
                                .red_chromaticity_threshold
                                .update_parameter_value(
                                    to_value(parameters.red_chromaticity_threshold).unwrap(),
                                );
                        }

                        let value = get_value_from_interpolated(
                            position,
                            &mut parameters.blue_chromaticity_threshold,
                        );
                        if ui
                            .add(
                                Slider::new(value, 0.0..=1.0)
                                    .text("blue_chromaticity_threshold")
                                    .smart_aim(false),
                            )
                            .changed()
                        {
                            self.parameters
                                .blue_chromaticity_threshold
                                .update_parameter_value(
                                    to_value(parameters.blue_chromaticity_threshold).unwrap(),
                                );
                        }

                        let value = get_value_from_interpolated(
                            position,
                            &mut parameters.green_chromaticity_threshold,
                        );
                        if ui
                            .add(
                                Slider::new(value, 0.0..=1.0)
                                    .text("green_chromaticity_threshold")
                                    .smart_aim(false),
                            )
                            .changed()
                        {
                            self.parameters
                                .green_chromaticity_threshold
                                .update_parameter_value(
                                    to_value(parameters.green_chromaticity_threshold).unwrap(),
                                );
                        }

                        let value = get_value_from_interpolated(
                            position,
                            &mut parameters.green_luminance_threshold,
                        );
                        if ui
                            .add(
                                Slider::new(value, 0.0..=255.0)
                                    .text("green_luminance_threshold")
                                    .smart_aim(false),
                            )
                            .changed()
                        {
                            self.parameters
                                .green_luminance_threshold
                                .update_parameter_value(
                                    to_value(parameters.green_luminance_threshold).unwrap(),
                                );
                        }
                    }
                    FieldColorFunction::Hsv => {
                        let value = get_value_from_interpolated(
                            position,
                            &mut parameters.hue_low_threshold,
                        );
                        if ui
                            .add(
                                Slider::new(value, 0.0..=360.0)
                                    .text("hue_low_threshold")
                                    .smart_aim(false),
                            )
                            .changed()
                        {
                            self.parameters.hue_low_threshold.update_parameter_value(
                                to_value(parameters.hue_low_threshold).unwrap(),
                            );
                        }

                        let value = get_value_from_interpolated(
                            position,
                            &mut parameters.hue_high_threshold,
                        );
                        if ui
                            .add(
                                Slider::new(value, 0.0..=360.0)
                                    .text("hue_high_threshold")
                                    .smart_aim(false),
                            )
                            .changed()
                        {
                            self.parameters.hue_high_threshold.update_parameter_value(
                                to_value(parameters.hue_high_threshold).unwrap(),
                            );
                        }

                        let value = get_value_from_interpolated(
                            position,
                            &mut parameters.saturation_low_threshold,
                        );
                        if ui
                            .add(
                                Slider::new(value, 0.0..=255.0)
                                    .text("saturation_low_threshold")
                                    .smart_aim(false),
                            )
                            .changed()
                        {
                            self.parameters
                                .saturation_low_threshold
                                .update_parameter_value(
                                    to_value(parameters.saturation_low_threshold).unwrap(),
                                );
                        }

                        let value = get_value_from_interpolated(
                            position,
                            &mut parameters.saturation_high_threshold,
                        );
                        if ui
                            .add(
                                Slider::new(value, 0.0..=255.0)
                                    .text("saturation_high_threshold")
                                    .smart_aim(false),
                            )
                            .changed()
                        {
                            self.parameters
                                .saturation_high_threshold
                                .update_parameter_value(
                                    to_value(parameters.saturation_high_threshold).unwrap(),
                                );
                        }
                    }
                }

                let value =
                    get_value_from_interpolated(position, &mut parameters.luminance_threshold);
                if ui
                    .add(
                        Slider::new(value, 0.0..=255.0)
                            .text("luminance_threshold")
                            .smart_aim(false),
                    )
                    .changed()
                {
                    self.parameters
                        .luminance_threshold
                        .update_parameter_value(to_value(parameters.luminance_threshold).unwrap());
                }
            }
        })
        .response
    }
}

struct Parameters<FunctionType, InterpolatedType> {
    function: FunctionType,
    luminance_threshold: InterpolatedType,
    vertical_edge_threshold: InterpolatedType,
    red_chromaticity_threshold: InterpolatedType,
    blue_chromaticity_threshold: InterpolatedType,
    green_chromaticity_threshold: InterpolatedType,
    green_luminance_threshold: InterpolatedType,
    hue_low_threshold: InterpolatedType,
    hue_high_threshold: InterpolatedType,
    saturation_low_threshold: InterpolatedType,
    saturation_high_threshold: InterpolatedType,
}

impl Parameters<ValueBuffer, ValueBuffer> {
    fn from(nao: &Nao, cycler: Cycler) -> Self {
        let function = nao.subscribe_parameter("field_color_detection.function");
        let luminance_threshold = nao.subscribe_parameter(get_luminance_threshold_path(cycler));
        let vertical_edge_threshold =
            nao.subscribe_parameter(get_vertical_edge_threshold_path(cycler));
        let red_chromaticity_threshold =
            nao.subscribe_parameter(get_red_chromaticity_threshold_path(cycler));
        let blue_chromaticity_threshold =
            nao.subscribe_parameter(get_blue_chromaticity_threshold_path(cycler));
        let green_chromaticity_threshold =
            nao.subscribe_parameter(get_green_chromaticity_threshold_path(cycler));
        let green_luminance_threshold =
            nao.subscribe_parameter(get_green_luminance_threshold_path(cycler));
        let hue_low_threshold = nao.subscribe_parameter(get_hue_low_threshold_path(cycler));
        let hue_high_threshold = nao.subscribe_parameter(get_hue_high_threshold_path(cycler));
        let saturation_low_threshold =
            nao.subscribe_parameter(get_saturation_low_threshold_path(cycler));
        let saturation_high_threshold =
            nao.subscribe_parameter(get_saturation_high_threshold_path(cycler));

        Self {
            function,
            luminance_threshold,
            vertical_edge_threshold,
            red_chromaticity_threshold,
            blue_chromaticity_threshold,
            green_chromaticity_threshold,
            green_luminance_threshold,
            hue_low_threshold,
            hue_high_threshold,
            saturation_low_threshold,
            saturation_high_threshold,
        }
    }

    fn parse_latest(&self) -> Result<Parameters<FieldColorFunction, Interpolated>> {
        Ok(Parameters {
            function: self
                .function
                .parse_latest()
                .wrap_err("failed to parse latest function")?,
            luminance_threshold: self
                .luminance_threshold
                .parse_latest()
                .wrap_err("failed to parse latest luminance_threshold")?,
            vertical_edge_threshold: self
                .vertical_edge_threshold
                .parse_latest()
                .wrap_err("failed to parse latest vertical_edge_threshold")?,
            red_chromaticity_threshold: self
                .red_chromaticity_threshold
                .parse_latest()
                .wrap_err("failed to parse latest red_chromaticity_threshold")?,
            blue_chromaticity_threshold: self
                .blue_chromaticity_threshold
                .parse_latest()
                .wrap_err("failed to parse latest blue_chromaticity_threshold")?,
            green_chromaticity_threshold: self
                .green_chromaticity_threshold
                .parse_latest()
                .wrap_err("failed to parse latest green_chromaticity_threshold")?,
            green_luminance_threshold: self
                .green_luminance_threshold
                .parse_latest()
                .wrap_err("failed to parse latest green_luminance_threshold")?,
            hue_low_threshold: self
                .hue_low_threshold
                .parse_latest()
                .wrap_err("failed to parse latest hue_low_threshold")?,
            hue_high_threshold: self
                .hue_high_threshold
                .parse_latest()
                .wrap_err("failed to parse latest hue_high_threshold")?,
            saturation_low_threshold: self
                .saturation_low_threshold
                .parse_latest()
                .wrap_err("failed to parse latest saturation_low_threshold")?,
            saturation_high_threshold: self
                .saturation_high_threshold
                .parse_latest()
                .wrap_err("failed to parse latest saturation_high_threshold")?,
        })
    }
}

impl Parameters<FieldColorFunction, Interpolated> {
    fn write_to(
        &self,
        repository_parameters: &RepositoryParameters,
        address: &str,
        cycler: Cycler,
    ) -> Result<()> {
        repository_parameters.write(
            address,
            "field_color_detection.function".to_string(),
            to_value(self.function).wrap_err("failed to serialize function")?,
        );
        repository_parameters.write(
            address,
            get_luminance_threshold_path(cycler).to_string(),
            to_value(self.luminance_threshold)
                .wrap_err("failed to serialize luminance_threshold")?,
        );
        repository_parameters.write(
            address,
            get_vertical_edge_threshold_path(cycler).to_string(),
            to_value(self.vertical_edge_threshold)
                .wrap_err("failed to serialize vertical_edge_threshold")?,
        );
        repository_parameters.write(
            address,
            get_red_chromaticity_threshold_path(cycler).to_string(),
            to_value(self.red_chromaticity_threshold)
                .wrap_err("failed to serialize red_chromaticity_threshold")?,
        );
        repository_parameters.write(
            address,
            get_blue_chromaticity_threshold_path(cycler).to_string(),
            to_value(self.blue_chromaticity_threshold)
                .wrap_err("failed to serialize blue_chromaticity_threshold")?,
        );
        repository_parameters.write(
            address,
            get_green_chromaticity_threshold_path(cycler).to_string(),
            to_value(self.green_chromaticity_threshold)
                .wrap_err("failed to serialize green_chromaticity_threshold")?,
        );
        repository_parameters.write(
            address,
            get_green_luminance_threshold_path(cycler).to_string(),
            to_value(self.green_luminance_threshold)
                .wrap_err("failed to serialize green_luminance_threshold")?,
        );
        repository_parameters.write(
            address,
            get_hue_low_threshold_path(cycler).to_string(),
            to_value(self.hue_low_threshold).wrap_err("failed to serialize hue_low_threshold")?,
        );
        repository_parameters.write(
            address,
            get_hue_high_threshold_path(cycler).to_string(),
            to_value(self.hue_high_threshold).wrap_err("failed to serialize hue_high_threshold")?,
        );
        repository_parameters.write(
            address,
            get_saturation_low_threshold_path(cycler).to_string(),
            to_value(self.saturation_low_threshold)
                .wrap_err("failed to serialize saturation_low_threshold")?,
        );
        repository_parameters.write(
            address,
            get_saturation_high_threshold_path(cycler).to_string(),
            to_value(self.saturation_high_threshold)
                .wrap_err("failed to serialize saturation_high_threshold")?,
        );
        Ok(())
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

fn add_selector_row(
    ui: &mut Ui,
    nao: &Nao,
    repository_parameters: &Result<RepositoryParameters>,
    cycler: &mut Cycler,
    position: &mut Option<Position>,
    function: &mut FieldColorFunction,
    parameters: &mut Parameters<ValueBuffer, ValueBuffer>,
) -> Response {
    ui.horizontal(|ui| {
        add_vision_cycler_selector(ui, nao, cycler, parameters);
        let response = add_position_selector(ui, position);
        if response.changed() {
            let injected_ground_to_field = match position {
                None => None,
                Some(position) => {
                    let injected_ground_to_field_translation = match position {
                        Position::FirstHalfOwnHalfTowardsOwnGoal
                        | Position::FirstHalfOwnHalfAwayOwnGoal => Translation2::new(-3.0, 0.0),
                        Position::FirstHalfOpponentHalfTowardsOwnGoal
                        | Position::FirstHalfOpponentHalfAwayOwnGoal => Translation2::new(3.0, 0.0),
                    };
                    let injected_ground_to_field_rotation = match position {
                        Position::FirstHalfOwnHalfTowardsOwnGoal
                        | Position::FirstHalfOpponentHalfTowardsOwnGoal => Rotation2::new(PI),
                        Position::FirstHalfOwnHalfAwayOwnGoal
                        | Position::FirstHalfOpponentHalfAwayOwnGoal => Rotation2::new(0.0),
                    };
                    Some(Isometry2::from_parts(
                        injected_ground_to_field_translation,
                        injected_ground_to_field_rotation.into(),
                    ))
                }
            };
            let value = to_value(injected_ground_to_field).unwrap();
            nao.update_parameter_value(
                "injected_ground_to_field_of_home_after_coin_toss_before_second_half",
                value,
            );
        }

        let response = add_function_selector(ui, function);
        if response.changed() {
            let value = to_value(function).unwrap();
            nao.update_parameter_value("field_color_detection.function", value);
        }

        match repository_parameters {
            Ok(repository_parameters) => {
                if ui
                    .button("Save all interpolated parameters of this cycler to disk")
                    .clicked()
                {
                    if let Some(address) = nao.get_address() {
                        if let Err(error) = parameters.parse_latest().and_then(|parameters| {
                            parameters.write_to(repository_parameters, &address, *cycler)
                        }) {
                            error!("Failed to parse parameters: {error:#?}");
                        }
                    }
                }
            }
            Err(error) => {
                ui.label(format!("{error:?}"));
            }
        }
    })
    .response
}

fn add_vision_cycler_selector(
    ui: &mut Ui,
    nao: &Nao,
    cycler: &mut Cycler,
    parameters: &mut Parameters<ValueBuffer, ValueBuffer>,
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
        *parameters = Parameters::from(nao, *cycler);
    }
    response
}

fn add_position_selector(ui: &mut Ui, position: &mut Option<Position>) -> Response {
    let mut position_selection_changed = false;
    let mut combo_box = ComboBox::from_label("Position")
        .selected_text(match position {
            None => "No Injection",
            Some(Position::FirstHalfOwnHalfTowardsOwnGoal) => "Own Half Towards Own Goal",
            Some(Position::FirstHalfOwnHalfAwayOwnGoal) => "Own Half Away Own Goal",
            Some(Position::FirstHalfOpponentHalfTowardsOwnGoal) => "Opponent Half Towards Own Goal",
            Some(Position::FirstHalfOpponentHalfAwayOwnGoal) => "Opponent Half Away Own Goal",
        })
        .show_ui(ui, |ui| {
            if ui
                .selectable_value(position, None, "No Injection")
                .clicked()
            {
                position_selection_changed = true;
            }
            if ui
                .selectable_value(
                    position,
                    Some(Position::FirstHalfOwnHalfTowardsOwnGoal),
                    "Own Half Towards Own Goal",
                )
                .clicked()
            {
                position_selection_changed = true;
            }
            if ui
                .selectable_value(
                    position,
                    Some(Position::FirstHalfOwnHalfAwayOwnGoal),
                    "Own Half Away Own Goal",
                )
                .clicked()
            {
                position_selection_changed = true;
            }
            if ui
                .selectable_value(
                    position,
                    Some(Position::FirstHalfOpponentHalfTowardsOwnGoal),
                    "Opponent Half Towards Own Goal",
                )
                .clicked()
            {
                position_selection_changed = true;
            }
            if ui
                .selectable_value(
                    position,
                    Some(Position::FirstHalfOpponentHalfAwayOwnGoal),
                    "Opponent Half Away Own Goal",
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

fn add_function_selector(ui: &mut Ui, function: &mut FieldColorFunction) -> Response {
    let mut function_selection_changed = false;
    let mut combo_box = ComboBox::from_label("Function")
        .selected_text(format!("{:?}", function))
        .show_ui(ui, |ui| {
            if ui
                .selectable_value(
                    function,
                    FieldColorFunction::GreenChromaticity,
                    "Green Chromaticity",
                )
                .clicked()
            {
                function_selection_changed = true;
            }
            if ui
                .selectable_value(function, FieldColorFunction::Hsv, "HSV")
                .clicked()
            {
                function_selection_changed = true;
            }
        });
    if function_selection_changed {
        combo_box.response.mark_changed()
    }
    combo_box.response
}

fn get_luminance_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.luminance_threshold",
        Cycler::VisionBottom => "field_color_detection.vision_bottom.luminance_threshold",
        _ => panic!("not implemented"),
    }
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

fn get_green_chromaticity_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.green_chromaticity_threshold",
        Cycler::VisionBottom => "field_color_detection.vision_bottom.green_chromaticity_threshold",
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

fn get_hue_low_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.hue_low_threshold",
        Cycler::VisionBottom => "field_color_detection.vision_bottom.hue_low_threshold",
        _ => panic!("not implemented"),
    }
}

fn get_hue_high_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.hue_high_threshold",
        Cycler::VisionBottom => "field_color_detection.vision_bottom.hue_high_threshold",
        _ => panic!("not implemented"),
    }
}

fn get_saturation_low_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.saturation_low_threshold",
        Cycler::VisionBottom => "field_color_detection.vision_bottom.saturation_low_threshold",
        _ => panic!("not implemented"),
    }
}

fn get_saturation_high_threshold_path(cycler: Cycler) -> &'static str {
    match cycler {
        Cycler::VisionTop => "field_color_detection.vision_top.saturation_high_threshold",
        Cycler::VisionBottom => "field_color_detection.vision_bottom.saturation_high_threshold",
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
