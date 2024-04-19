use std::{f32::consts::PI, sync::Arc};

use color_eyre::{eyre::Context, Result};
use eframe::egui::{ComboBox, Response, Slider, Ui, Widget};
use log::error;
use nalgebra::{Isometry2, Rotation2, Translation2};
use serde_json::{to_value, Value};

use communication::client::Cycler;
use types::interpolated::Interpolated;

use crate::{
    nao::Nao, panel::Panel, repository_parameters::RepositoryParameters, value_buffer::ValueBuffer,
};

pub struct VisionTunerPanel {
    nao: Arc<Nao>,
    repository_parameters: Result<RepositoryParameters>,
    cycler: Cycler,
    position: Option<Position>,
    parameters: Parameters<ValueBuffer>,
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
        })
        .response
    }
}

struct Parameters<T> {
    vertical_edge_threshold: T,
    red_chromaticity_threshold: T,
    blue_chromaticity_threshold: T,
    green_chromaticity_threshold: T,
    green_luminance_threshold: T,
}

impl Parameters<ValueBuffer> {
    fn from(nao: &Nao, cycler: Cycler) -> Self {
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

        Self {
            vertical_edge_threshold,
            red_chromaticity_threshold,
            blue_chromaticity_threshold,
            green_chromaticity_threshold,
            green_luminance_threshold,
        }
    }

    fn parse_latest(&self) -> Result<Parameters<Interpolated>> {
        Ok(Parameters {
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
        })
    }
}

impl Parameters<Interpolated> {
    fn write_to(
        &self,
        repository_parameters: &RepositoryParameters,
        address: &str,
        cycler: Cycler,
    ) -> Result<()> {
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
    parameters: &mut Parameters<ValueBuffer>,
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
    parameters: &mut Parameters<ValueBuffer>,
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
