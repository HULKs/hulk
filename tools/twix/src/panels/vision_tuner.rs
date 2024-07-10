use std::{f32::consts::PI, fmt::Display, sync::Arc};

use color_eyre::{eyre::eyre, Result};
use communication::messages::TextOrBinary;
use eframe::egui::{ComboBox, Response, Slider, Ui, Widget};
use log::error;
use nalgebra::{Isometry2, Rotation2, Translation2};
use serde_json::{to_value, Value};

use types::{field_color::FieldColorFunction, interpolated::Interpolated};

use crate::{nao::Nao, panel::Panel, value_buffer::BufferHandle};

use super::image::cycler_selector::{VisionCycler, VisionCyclerSelector};

trait SelectPerspective {
    fn as_seen_from(&self, perspective: Perspective) -> f32;
}

impl SelectPerspective for Interpolated {
    fn as_seen_from(&self, perspective: Perspective) -> f32 {
        match perspective {
            Perspective::FirstHalfOwnHalfTowardsOwnGoal => {
                self.first_half_own_half_towards_own_goal
            }
            Perspective::FirstHalfOwnHalfAwayOwnGoal => self.first_half_own_half_away_own_goal,
            Perspective::FirstHalfOpponentHalfTowardsOwnGoal => {
                self.first_half_opponent_half_towards_own_goal
            }
            Perspective::FirstHalfOpponentHalfAwayOwnGoal => {
                self.first_half_opponent_half_away_own_goal
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::enum_variant_names)]
enum Perspective {
    FirstHalfOwnHalfTowardsOwnGoal,
    FirstHalfOwnHalfAwayOwnGoal,
    FirstHalfOpponentHalfTowardsOwnGoal,
    FirstHalfOpponentHalfAwayOwnGoal,
}

impl Display for Perspective {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Perspective::FirstHalfOwnHalfTowardsOwnGoal => {
                write!(f, "first_half_own_half_towards_own_goal")
            }
            Perspective::FirstHalfOwnHalfAwayOwnGoal => {
                write!(f, "first_half_own_half_away_own_goal")
            }
            Perspective::FirstHalfOpponentHalfTowardsOwnGoal => {
                write!(f, "first_half_opponent_half_towards_own_goal")
            }
            Perspective::FirstHalfOpponentHalfAwayOwnGoal => {
                write!(f, "first_half_opponent_half_away_own_goal")
            }
        }
    }
}

struct Parameters<T> {
    vertical_edge: T,
    red_chromaticity: T,
    green_chromaticity: T,
    blue_chromaticity: T,
    green_luminance: T,
    hue_low: T,
    hue_high: T,
    saturation_low: T,
    saturation_high: T,
}

impl Parameters<BufferHandle<Interpolated>> {
    fn as_seen_from(&self, perspective: Perspective) -> Result<Parameters<f32>> {
        let vertical_edge = self
            .vertical_edge
            .get_last_value()?
            .ok_or_else(|| eyre!("failed to get vertical edge threshold from buffer"))?
            .as_seen_from(perspective);
        let red_chromaticity = self
            .red_chromaticity
            .get_last_value()?
            .ok_or_else(|| eyre!("failed to get red chromaticity threshold from buffer"))?
            .as_seen_from(perspective);
        let green_chromaticity = self
            .green_chromaticity
            .get_last_value()?
            .ok_or_else(|| eyre!("failed to get green chromaticity threshold from buffer"))?
            .as_seen_from(perspective);
        let blue_chromaticity = self
            .blue_chromaticity
            .get_last_value()?
            .ok_or_else(|| eyre!("failed to get blue chromaticity threshold from buffer"))?
            .as_seen_from(perspective);
        let green_luminance = self
            .green_luminance
            .get_last_value()?
            .ok_or_else(|| eyre!("failed to get green luminance threshold from buffer"))?
            .as_seen_from(perspective);
        let hue_low = self
            .hue_low
            .get_last_value()?
            .ok_or_else(|| eyre!("failed to get hue low threshold from buffer"))?
            .as_seen_from(perspective);
        let hue_high = self
            .hue_high
            .get_last_value()?
            .ok_or_else(|| eyre!("failed to get hue high threshold from buffer"))?
            .as_seen_from(perspective);
        let saturation_low = self
            .saturation_low
            .get_last_value()?
            .ok_or_else(|| eyre!("failed to get saturation low threshold from buffer"))?
            .as_seen_from(perspective);
        let saturation_high = self
            .saturation_high
            .get_last_value()?
            .ok_or_else(|| eyre!("failed to get saturation high threshold from buffer"))?
            .as_seen_from(perspective);

        let parameters = Parameters {
            vertical_edge,
            red_chromaticity,
            green_chromaticity,
            blue_chromaticity,
            green_luminance,
            hue_low,
            hue_high,
            saturation_low,
            saturation_high,
        };
        Ok(parameters)
    }
}

pub struct VisionTunerPanel {
    nao: Arc<Nao>,
    cycler: VisionCycler,
    perspective: Option<Perspective>,
    field_color_function: BufferHandle<FieldColorFunction>,
    parameters: Parameters<BufferHandle<Interpolated>>,
}

impl Panel for VisionTunerPanel {
    const NAME: &'static str = "Vision Tuner";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let cycler = VisionCycler::Top;
        let perspective = None;

        let field_color_function = nao.subscribe_value("parameters.field_color_detection.function");
        let parameters = resubscribe(&nao, cycler);

        Self {
            nao,
            cycler,
            perspective,
            field_color_function,
            parameters,
        }
    }
}

impl Widget for &mut VisionTunerPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.style_mut().spacing.slider_width = ui.available_size().x - 250.0;
        let layout= ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let mut cycler_selector = VisionCyclerSelector::new(&mut self.cycler);
                if cycler_selector.ui(ui).changed() {
                    self.parameters = resubscribe(&self.nao, self.cycler);
                }
                let mut perspective_selector = PerspectiveSelector::new(&mut self.perspective);
                if perspective_selector.ui(ui).changed() {
                    let injected_ground_to_field = match self.perspective {
                        None => None,
                        Some(position) => {
                            let injected_ground_to_field_translation = match position {
                                Perspective::FirstHalfOwnHalfTowardsOwnGoal
                                | Perspective::FirstHalfOwnHalfAwayOwnGoal => {
                                    Translation2::new(-3.0, 0.0)
                                }
                                Perspective::FirstHalfOpponentHalfTowardsOwnGoal
                                | Perspective::FirstHalfOpponentHalfAwayOwnGoal => {
                                    Translation2::new(3.0, 0.0)
                                }
                            };
                            let injected_ground_to_field_rotation = match position {
                                Perspective::FirstHalfOwnHalfTowardsOwnGoal
                                | Perspective::FirstHalfOpponentHalfTowardsOwnGoal => {
                                    Rotation2::new(PI)
                                }
                                Perspective::FirstHalfOwnHalfAwayOwnGoal
                                | Perspective::FirstHalfOpponentHalfAwayOwnGoal => {
                                    Rotation2::new(0.0)
                                }
                            };
                            Some(Isometry2::from_parts(
                                injected_ground_to_field_translation,
                                injected_ground_to_field_rotation.into(),
                            ))
                        }
                    };
                    let value = to_value(injected_ground_to_field).unwrap();
                    self.nao.write(
                        "parameters.injected_ground_to_field_of_home_after_coin_toss_before_second_half",
                        TextOrBinary::Text(value),
                    );
                }
            });
            ui.separator();
            if let Some(mut function) = self.field_color_function.get_last_value()? {
                let mut function_selector = FieldColorFunctionSelector::new(&mut function);
                if function_selector.ui(ui).changed() {
                    self.nao.write(
                        "parameters.field_color_detection.function",
                        TextOrBinary::Text(to_value(function).unwrap()),
                    );
                }
            }
            if let Some(perspective) = self.perspective{
                let cycler=self.cycler.as_snake_case_path();
                let mut parameters = self.parameters.as_seen_from(perspective)?;
                let slider = ui.add(Slider::new(&mut parameters.vertical_edge, 0.0..=255.0).text("vertical_edge_threshold"));
                if slider.changed(){
                    self.nao.write(
                        format!("parameters.image_segmenter.{cycler}.vertical_edge_threshold.{perspective}"),
                        TextOrBinary::Text(to_value(parameters.vertical_edge).unwrap()),
                    );
                }
                let slider = ui.add(Slider::new(&mut parameters.red_chromaticity, 0.0..=1.0).text("red_chromaticity"));
                if slider.changed(){
                    self.nao.write(
                        format!("parameters.field_color_detection.{cycler}.red_chromaticity_threshold.{perspective}"),
                        TextOrBinary::Text(to_value(parameters.red_chromaticity).unwrap()),
                    );
                }
                let slider = ui.add(Slider::new(&mut parameters.green_chromaticity, 0.0..=1.0).text("green_chromaticity"));
                if slider.changed(){
                    self.nao.write(
                        format!("parameters.field_color_detection.{cycler}.green_chromaticity_threshold.{perspective}"),
                        TextOrBinary::Text(to_value(parameters.green_chromaticity).unwrap()),
                    );
                }
                let slider = ui.add(Slider::new(&mut parameters.blue_chromaticity, 0.0..=1.0).text("blue_chromaticity"));
                if slider.changed(){
                    self.nao.write(
                        format!("parameters.field_color_detection.{cycler}.blue_chromaticity_threshold.{perspective}"),
                        TextOrBinary::Text(to_value(parameters.blue_chromaticity).unwrap()),
                    );
                }
                let slider = ui.add(Slider::new(&mut parameters.green_luminance, 0.0..=255.0).text("green_luminance"));
                if slider.changed(){
                    self.nao.write(
                        format!("parameters.field_color_detection.{cycler}.green_luminance_threshold.{perspective}"),
                        TextOrBinary::Text(to_value(parameters.green_luminance).unwrap()),
                    );
                }
                let slider = ui.add(Slider::new(&mut parameters.hue_low, 0.0..=360.0).text("hue_low"));
                if slider.changed(){
                    self.nao.write(
                        format!("parameters.field_color_detection.{cycler}.hue_low_threshold.{perspective}"),
                        TextOrBinary::Text(to_value(parameters.hue_low).unwrap()),
                    );
                }
                let slider = ui.add(Slider::new(&mut parameters.hue_high, 0.0..=360.0).text("hue_high"));
                if slider.changed(){
                    self.nao.write(
                        format!("parameters.field_color_detection.{cycler}.hue_high_threshold.{perspective}"),
                        TextOrBinary::Text(to_value(parameters.hue_high).unwrap()),
                    );
                }
                let slider = ui.add(Slider::new(&mut parameters.saturation_low, 0.0..=255.0).text("saturation_low"));
                if slider.changed(){
                    self.nao.write(
                        format!("parameters.field_color_detection.{cycler}.saturation_low_threshold.{perspective}"),
                        TextOrBinary::Text(to_value(parameters.saturation_low).unwrap()),
                    );
                }
                let slider = ui.add(Slider::new(&mut parameters.saturation_high, 0.0..=255.0).text("saturation_high"));
                if slider.changed(){
                    self.nao.write(
                        format!("parameters.field_color_detection.{cycler}.saturation_high_threshold.{perspective}"),
                        TextOrBinary::Text(to_value(parameters.saturation_high).unwrap()),
                    );
                }

            }
            Ok::<(), color_eyre::Report>(())
        });
        if let Err(error) = layout.inner {
            error!("failed to render vision tuner panel: {error}");
        }
        layout.response
    }
}

fn resubscribe(nao: &Nao, cycler: VisionCycler) -> Parameters<BufferHandle<Interpolated>> {
    let camera = cycler.as_snake_case_path();
    let vertical_edge = nao.subscribe_value(format!(
        "parameters.image_segmenter.{camera}.vertical_edge_threshold"
    ));
    let red_chromaticity = nao.subscribe_value(format!(
        "parameters.field_color_detection.{camera}.red_chromaticity_threshold"
    ));
    let green_chromaticity = nao.subscribe_value(format!(
        "parameters.field_color_detection.{camera}.green_chromaticity_threshold"
    ));
    let blue_chromaticity = nao.subscribe_value(format!(
        "parameters.field_color_detection.{camera}.blue_chromaticity_threshold"
    ));
    let green_luminance = nao.subscribe_value(format!(
        "parameters.field_color_detection.{camera}.green_luminance_threshold"
    ));
    let hue_low = nao.subscribe_value(format!(
        "parameters.field_color_detection.{camera}.hue_low_threshold"
    ));
    let hue_high = nao.subscribe_value(format!(
        "parameters.field_color_detection.{camera}.hue_high_threshold"
    ));
    let saturation_low = nao.subscribe_value(format!(
        "parameters.field_color_detection.{camera}.saturation_low_threshold"
    ));
    let saturation_high = nao.subscribe_value(format!(
        "parameters.field_color_detection.{camera}.saturation_high_threshold"
    ));

    Parameters {
        vertical_edge,
        red_chromaticity,
        green_chromaticity,
        blue_chromaticity,
        green_luminance,
        hue_low,
        hue_high,
        saturation_low,
        saturation_high,
    }
}

#[derive(Debug)]
pub struct PerspectiveSelector<'a> {
    perspective: &'a mut Option<Perspective>,
}

impl<'a> PerspectiveSelector<'a> {
    fn new(perspective: &'a mut Option<Perspective>) -> Self {
        Self { perspective }
    }
}

impl<'a> Widget for &mut PerspectiveSelector<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut selection_changed = false;
        let mut combo_box = ComboBox::from_label("Perspective")
            .selected_text(match self.perspective {
                None => "No Injection",
                Some(Perspective::FirstHalfOwnHalfTowardsOwnGoal) => "Own Half Towards Own Goal",
                Some(Perspective::FirstHalfOwnHalfAwayOwnGoal) => "Own Half Away Own Goal",
                Some(Perspective::FirstHalfOpponentHalfTowardsOwnGoal) => {
                    "Opponent Half Towards Own Goal"
                }
                Some(Perspective::FirstHalfOpponentHalfAwayOwnGoal) => {
                    "Opponent Half Away Own Goal"
                }
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(self.perspective, None, "No Injection")
                    .clicked()
                {
                    selection_changed = true;
                }
                if ui
                    .selectable_value(
                        self.perspective,
                        Some(Perspective::FirstHalfOwnHalfTowardsOwnGoal),
                        "Own Half Towards Own Goal",
                    )
                    .clicked()
                {
                    selection_changed = true;
                }
                if ui
                    .selectable_value(
                        self.perspective,
                        Some(Perspective::FirstHalfOwnHalfAwayOwnGoal),
                        "Own Half Away Own Goal",
                    )
                    .clicked()
                {
                    selection_changed = true;
                }
                if ui
                    .selectable_value(
                        self.perspective,
                        Some(Perspective::FirstHalfOpponentHalfTowardsOwnGoal),
                        "Opponent Half Towards Own Goal",
                    )
                    .clicked()
                {
                    selection_changed = true;
                }
                if ui
                    .selectable_value(
                        self.perspective,
                        Some(Perspective::FirstHalfOpponentHalfAwayOwnGoal),
                        "Opponent Half Away Own Goal",
                    )
                    .clicked()
                {
                    selection_changed = true;
                }
            });
        if selection_changed {
            combo_box.response.mark_changed()
        }
        combo_box.response
    }
}

#[derive(Debug)]
pub struct FieldColorFunctionSelector<'a> {
    function: &'a mut FieldColorFunction,
}

impl<'a> FieldColorFunctionSelector<'a> {
    pub fn new(function: &'a mut FieldColorFunction) -> Self {
        Self { function }
    }
}

impl<'a> Widget for &mut FieldColorFunctionSelector<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut selection_changed = false;
        let mut combo_box = ComboBox::from_label("Field Color Function")
            .selected_text(match self.function {
                FieldColorFunction::GreenChromaticity => "GreenChromaticity",
                FieldColorFunction::Hsv => "Hsv",
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(
                        self.function,
                        FieldColorFunction::GreenChromaticity,
                        "GreenChromaticity",
                    )
                    .clicked()
                {
                    selection_changed = true;
                }
                if ui
                    .selectable_value(self.function, FieldColorFunction::Hsv, "Hsv")
                    .clicked()
                {
                    selection_changed = true;
                }
            });
        if selection_changed {
            combo_box.response.mark_changed()
        }
        combo_box.response
    }
}
