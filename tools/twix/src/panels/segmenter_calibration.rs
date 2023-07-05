use color_eyre::eyre::{Context, Result};
use eframe::egui::{Response, Slider, Ui, Widget};
use log::{error, info};
use nalgebra::Vector3;
use serde_json::Value;
use std::{ops::RangeInclusive, sync::Arc};
use tokio::sync::mpsc;
use types::FieldColor;

use crate::{
    nao::Nao, panel::Panel, repository_parameters::RepositoryParameters, value_buffer::ValueBuffer,
};

use super::parameter::{add_save_button, subscribe};

struct ParameterSubscriptions<DeserializedValueType> {
    human_friendly_label: String,
    path: String,
    value_buffer: ValueBuffer,
    value: DeserializedValueType,
    update_notify_receiver: mpsc::Receiver<()>,
}

pub struct SegmenterCalibrationPanel {
    nao: Arc<Nao>,
    repository_parameters: Result<RepositoryParameters>,
    field_color_subscriptions: [ParameterSubscriptions<Option<FieldColor>>; 2],
    image_segmenter_subscriptions: [ParameterSubscriptions<Option<serde_json::Value>>; 2],
}

const FIELD_COLOUR_KEY_BASE: &str = "field_color_detection.vision_";
const IMAGE_SEGMENTER_KEY_BASE: &str = "image_segmenter.vision_";

impl Panel for SegmenterCalibrationPanel {
    const NAME: &'static str = "Segmenter Calibration";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let field_color_subscriptions = ["Top", "Bottom"].map(|name| {
            let path = FIELD_COLOUR_KEY_BASE.to_owned() + name.to_lowercase().as_str();

            let (update_notify_sender, update_notify_receiver) = mpsc::channel(1);
            let value_buffer = subscribe(nao.clone(), &path, update_notify_sender)
                .expect("ValudBuffer is None, subscription failed");

            info!("Subscribing to path {}", path);

            ParameterSubscriptions {
                human_friendly_label: name.to_string(),
                path,
                value_buffer,
                value: None,
                update_notify_receiver,
            }
        });

        let image_segmenter_subscriptions = ["Top", "Bottom"].map(|name| {
            let path = IMAGE_SEGMENTER_KEY_BASE.to_owned() + name.to_lowercase().as_str();

            let (update_notify_sender, update_notify_receiver) = mpsc::channel(1);
            let value_buffer = subscribe(nao.clone(), &path, update_notify_sender)
                .expect("ValudBuffer is None, subscription failed");

            info!("Subscribing to path {}", path);

            ParameterSubscriptions {
                human_friendly_label: name.to_string(),
                path,
                value_buffer,
                value: None,
                update_notify_receiver,
            }
        });

        Self {
            nao,
            repository_parameters: RepositoryParameters::try_new(),
            field_color_subscriptions,
            image_segmenter_subscriptions,
        }
    }
}

fn add_field_color_ui_components(
    ui: &mut Ui,
    nao: Arc<Nao>,
    repository_parameters: &RepositoryParameters,
    field_color_subscription: &mut ParameterSubscriptions<Option<FieldColor>>,
) {
    let buffer = &field_color_subscription.value_buffer;

    let mut field_color_option = &mut field_color_subscription.value;
    let label = &field_color_subscription.human_friendly_label;
    let subscription_path = &field_color_subscription.path;
    let update_notify_reciever = &mut field_color_subscription.update_notify_receiver;

    let green_chromacity_range = (0.0f32, 1.0f32);
    let green_luminance_range = (0u8, u8::MAX);

    ui.horizontal(|ui| {
        match buffer.parse_latest::<FieldColor>() {
            Ok(value) => {
                *field_color_option = Some(value);
            }
            Err(error) => {
                ui.label(format!("{error:#?}"));
            }
        }

        ui.label(format!("{label:#} Camera"));

        add_save_button(
            ui,
            subscription_path,
            || {
                serde_json::to_value(&field_color_option)
                    .wrap_err("Conveting FieldColor to serde_json::Value failed.")
            },
            nao.clone(),
            repository_parameters,
        );
    });

    ui.style_mut().spacing.slider_width = ui.available_size().x - 100.0;

    let mut changed = false;

    match &mut field_color_option {
        Some(field_color_value) => {
            ui.label(format!(
                "Green Chromacity [{}°, {}°]",
                green_chromacity_range.0, green_chromacity_range.1
            ));
            for (axis_value, axis_name) in [
                (
                    &mut field_color_value.lower_green_chromaticity_threshold,
                    "lower_green_chromaticity_threshold",
                ),
                (
                    &mut field_color_value.upper_green_chromaticity_threshold,
                    "upper_green_chromaticity_threshold",
                ),
            ] {
                let slider = Slider::new(
                    axis_value,
                    RangeInclusive::new(green_chromacity_range.0, green_chromacity_range.1),
                )
                .text(axis_name)
                .smart_aim(false);
                if ui.add(slider).changed() {
                    changed = true
                };
            }
            {
                ui.label(format!(
                    "Green Luminance Threshold [{}°, {}°]",
                    green_luminance_range.0, green_luminance_range.1
                ));
                let axis_value = &mut field_color_value.green_luminance_threshold;
                let axis_name = "green_luminance_threshold";

                let slider = Slider::new(
                    axis_value,
                    RangeInclusive::new(green_luminance_range.0, green_luminance_range.1),
                )
                .text(axis_name)
                .smart_aim(false);
                if ui.add(slider).changed() {
                    changed = true
                };
            }
        }
        _ => {
            ui.label("Extrinsic parameters not recieved.");
        }
    };
    if changed {
        if let Some(field_color_value) = field_color_option {
            match serde_json::value::to_value(field_color_value) {
                Ok(value) => {
                    nao.update_parameter_value(subscription_path, value);
                }
                Err(error) => error!("Failed to serialize parameter value: {error:#?}"),
            }
        }
    }
}

impl Widget for &mut SegmenterCalibrationPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let repository_parameters = match &self.repository_parameters {
            Ok(repository_parameters) => repository_parameters,
            Err(error) => return ui.label(format!("{error:#?}")),
        };
        ui.vertical(|ui| {
            for subscription in &mut self.field_color_subscriptions {
                add_field_color_ui_components(
                    ui,
                    self.nao.clone(),
                    repository_parameters,
                    subscription,
                );

                ui.separator();
            }
        })
        .response
    }
}
