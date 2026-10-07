use std::sync::Arc;

use bevy::{camera_controller::pan_orbit_camera::prelude::PanOrbitCamera, prelude::*};
use coordinate_systems::{Field, Robot};
use eframe::egui::{ComboBox, Ui};
use egui_bevy::BevyWidget;
use kinematics::robot_kinematics::RobotKinematics;
use linear_algebra::Isometry3;
use projection::camera_matrix::CameraMatrix;
use ros_z::{
    qos::{QosDurability, QosProfile},
    time::Time,
};
use ros_z_debug::{ObservationPolicy, SampleRecord};
use ros2::sensor_msgs::image::Image as RosImage;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use types::{
    field_dimensions::FieldDimensions, time_wrapper::TimeWrapper,
    visual_localization::VisualLocalizationFrame as FieldMarkAssociations,
    visual_odometry::VisualOdometer,
};

use crate::{
    panel::{Panel, PanelCreationContext, PanelUiContext},
    repaint::ObservationContext,
};
use observation::Observation;
use transforms::*;

mod associations;
mod camera;
mod field;
#[cfg(test)]
mod gpu_test;
mod observation;
mod robot;
mod transforms;

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
enum PoseSource {
    #[default]
    Localization,
    VisualOdometer,
}

#[derive(Clone, Resource, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    field: bool,
    robot: bool,
    camera: bool,
    associations: bool,
    pose_source: PoseSource,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            field: true,
            robot: true,
            camera: true,
            associations: false,
            pose_source: PoseSource::Localization,
        }
    }
}

#[derive(Default, Resource)]
struct ViewerData {
    pose_source: PoseSource,
    field_dimensions: Option<Arc<SampleRecord<FieldDimensions>>>,
    localization: Option<Isometry3<Field, Robot>>,
    visual_odometer: Option<nalgebra::Isometry3<f32>>,
    robot_kinematics: Option<Arc<SampleRecord<TimeWrapper<RobotKinematics>>>>,
    camera_matrix: Option<CameraMatrix>,
    camera_frame: Option<Arc<SampleRecord<RosImage>>>,
    field_mark_associations: Option<Arc<SampleRecord<TimeWrapper<FieldMarkAssociations>>>>,
}

struct Observations {
    replay_revision: u64,
    namespace: String,
    frame_id: Option<(u64, u64)>,
    last_anchor: Option<Time>,
    dimensions: Observation<FieldDimensions>,
    localization: Observation<types::localization::LocalizationEstimate>,
    localization_status: Observation<types::localization::LocalizationStatus>,
    odometer: Observation<VisualOdometer>,
    kinematics: Observation<TimeWrapper<RobotKinematics>>,
    matrix: Observation<TimeWrapper<CameraMatrix>>,
    images: Option<Observation<RosImage>>,
    associations: Option<associations::Observations>,
}

impl Observations {
    fn new(context: &impl ObservationContext) -> color_eyre::Result<Self> {
        Ok(Self {
            replay_revision: 0,
            namespace: String::new(),
            frame_id: None,
            last_anchor: None,
            dimensions: Observation::new(
                context,
                "field_dimensions",
                1,
                ObservationPolicy::default().with_subscriber_qos(QosProfile {
                    durability: QosDurability::TransientLocal,
                    ..Default::default()
                }),
            )?,
            localization: Observation::new(
                context,
                "localization/estimate",
                1,
                Default::default(),
            )?,
            odometer: Observation::new(
                context,
                "visual_odometry/current_left_camera_to_visual_odometer",
                64,
                Default::default(),
            )?,
            localization_status: Observation::new(
                context,
                "localization/status",
                1,
                ObservationPolicy::default().with_subscriber_qos(QosProfile {
                    durability: QosDurability::TransientLocal,
                    ..Default::default()
                }),
            )?,
            kinematics: Observation::new(context, "robot_kinematics", 1024, Default::default())?,
            matrix: Observation::new(context, "camera_matrix", 1024, Default::default())?,
            images: None,
            associations: None,
        })
    }

    fn update_layers(
        &mut self,
        context: &impl ObservationContext,
        settings: &Settings,
    ) -> color_eyre::Result<()> {
        if !settings.camera {
            self.images = None;
        }
        if !settings.associations {
            self.associations = None;
        }
        if settings.camera && self.images.is_none() {
            self.images = Some(Observation::new(
                context,
                "inputs/left_image",
                12,
                Default::default(),
            )?);
        }
        if settings.associations && self.associations.is_none() {
            self.associations = Some(associations::Observations::new(context)?);
        }
        Ok(())
    }

    fn snapshot(&mut self, namespace: &str, settings: &Settings) -> ViewerData {
        let frame_id = self
            .localization_status
            .latest(namespace)
            .map(|status| (status.value.epoch, status.value.generation));
        if self.namespace != namespace
            || self.frame_id != frame_id
            || (!settings.camera && !settings.associations)
        {
            self.namespace = namespace.to_owned();
            self.frame_id = frame_id;
            self.last_anchor = None;
        }
        let images = self
            .images
            .as_ref()
            .map(|images| images.all(namespace))
            .unwrap_or_default();
        let associations = self
            .associations
            .as_ref()
            .map(|associations| associations.current_frames(namespace, frame_id))
            .unwrap_or_default();
        let anchor = select_anchor(
            images.iter().map(|record| image_time(record)),
            associations.iter().map(|record| record.value.time),
            self.last_anchor,
        );
        self.last_anchor = anchor;
        let association = associations
            .iter()
            .find(|record| Some(record.value.time) == anchor);
        let camera_frame = images
            .iter()
            .find(|image| Some(image_time(image)) == anchor)
            .cloned();
        let camera_matrix = self
            .matrix
            .aligned(namespace, anchor)
            .map(|record| record.value.inner.clone());
        let odometer = match anchor {
            Some(time) => observation::nearest(
                self.odometer.all(namespace),
                time,
                std::time::Duration::from_millis(100),
                |record| record.value.time,
            ),
            None => self
                .odometer
                .all(namespace)
                .into_iter()
                .max_by_key(|record| record.value.time),
        };
        ViewerData {
            pose_source: settings.pose_source,
            field_dimensions: self.dimensions.latest(namespace),
            // ponytail: latest solve pose; use estimate history for image-aligned rendering.
            localization: self
                .localization
                .latest(namespace)
                .filter(|record| frame_id == Some((record.value.epoch, record.value.generation)))
                .and_then(|record| record.value.robot_to_field)
                .map(|field| Isometry3::wrap(field.pose.inner.cast::<f32>().inverse())),
            visual_odometer: odometer
                .map(|record| record.value.current_left_camera_to_visual_odometer),
            robot_kinematics: self.kinematics.aligned(namespace, anchor),
            camera_matrix,
            camera_frame,
            field_mark_associations: association.cloned(),
        }
    }
}

fn select_anchor(
    images: impl Iterator<Item = Time> + Clone,
    associations: impl Iterator<Item = Time>,
    last: Option<Time>,
) -> Option<Time> {
    let latest_image = images.clone().filter(|time| Some(*time) >= last).max();
    let association = associations
        .filter(|time| Some(*time) >= last)
        .filter(|time| {
            latest_image.is_none_or(|latest| {
                *time <= latest
                    && latest.abs_diff(*time) <= std::time::Duration::from_secs(1)
                    && images.clone().any(|image| image == *time)
                    // An already displayed association must not hold back newer images.
                    && (Some(*time) > last || Some(latest) == last)
            })
        })
        .max();
    association.or(latest_image).or(last)
}

fn image_time(record: &SampleRecord<RosImage>) -> Time {
    record.value.header.stamp.into()
}

pub struct Map3DPanel {
    settings: Settings,
    widget: Option<BevyWidget>,
    observations: Result<Observations, String>,
}

impl Panel for Map3DPanel {
    const STORAGE_ID: &'static str = "map_3d";
    const DISPLAY_NAME: &'static str = "3D Map";
    const ICON: &'static str = egui_material_icons::icons::ICON_MAP.codepoint;

    fn new(context: PanelCreationContext<'_>) -> Self {
        let settings: Settings = context
            .value
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or_default();
        let observations = Observations::new(&context).map_err(|error| error.to_string());
        let widget = context.render_state.clone().map(|render_state| {
            let mut widget = BevyWidget::new(render_state);
            widget
                .bevy_app
                .insert_resource(ViewerData::default())
                .insert_resource(settings.clone())
                .insert_resource(GlobalAmbientLight {
                    color: Color::WHITE,
                    brightness: 600.0,
                    ..default()
                })
                .add_systems(
                    Startup,
                    (
                        field::setup,
                        robot::setup,
                        camera::setup,
                        associations::setup,
                    ),
                )
                .add_systems(
                    Update,
                    (
                        position_camera_once,
                        field::visibility,
                        field::update_field_plane,
                        field::update_field_markings
                            .run_if(|settings: Res<Settings>| settings.field),
                        robot::update,
                        (camera::update_camera_image, camera::update_camera_viewport).chain(),
                        associations::update,
                    ),
                );
            widget.bevy_app.finish();
            widget.bevy_app.cleanup();
            widget
        });
        Self {
            settings,
            widget,
            observations,
        }
    }

    fn save(&self) -> Value {
        serde_json::to_value(&self.settings).expect("settings serialize")
    }

    fn ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
        ui.horizontal(|ui| {
            ui.menu_button("Layers", |ui| {
                ui.checkbox(&mut self.settings.field, "Field");
                ui.checkbox(&mut self.settings.robot, "Robot");
                ui.checkbox(&mut self.settings.camera, "Camera");
                ui.checkbox(&mut self.settings.associations, "Associations")
                    .on_hover_text(
                        "Field-frame associations require the localization pose source.",
                    );
            });
            ComboBox::from_id_salt(ui.id().with("pose_source"))
                .selected_text(match self.settings.pose_source {
                    PoseSource::Localization => "Localization",
                    PoseSource::VisualOdometer => "Visual odometry",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.pose_source,
                        PoseSource::Localization,
                        "Localization",
                    );
                    ui.selectable_value(
                        &mut self.settings.pose_source,
                        PoseSource::VisualOdometer,
                        "Visual odometry",
                    );
                });
        });
        let Some(widget) = &mut self.widget else {
            ui.label("3D rendering requires the WGPU renderer.");
            return;
        };
        let data = match &mut self.observations {
            Ok(observations) => {
                if let Err(error) = observations.update_layers(&context, &self.settings) {
                    ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                }
                if observations.replay_revision != context.backend.replay_revision() {
                    observations.replay_revision = context.backend.replay_revision();
                    observations.last_anchor = None;
                }
                observations.snapshot(&context.backend.namespace(), &self.settings)
            }
            Err(error) => {
                ui.colored_label(ui.visuals().error_fg_color, error);
                ViewerData::default()
            }
        };
        widget.bevy_app.world_mut().insert_resource(data);
        widget
            .bevy_app
            .world_mut()
            .insert_resource(self.settings.clone());
        if ui.available_width() >= 1.0 && ui.available_height() >= 1.0 {
            ui.add(widget);
        }
    }
}

fn position_camera_once(
    mut positioned: Local<bool>,
    mut cameras: Query<(&mut Transform, &mut PanOrbitCamera), With<Camera3d>>,
) {
    if *positioned {
        return;
    }
    for (mut transform, mut camera) in &mut cameras {
        *transform = Transform::from_xyz(4.0, 6.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y);
        camera.last_anchor_depth = -(transform.translation.length() as f64);
        *positioned = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchors_only_advance() {
        let select = |images: &[i64], associations: &[i64], last: Option<i64>| {
            select_anchor(
                images.iter().copied().map(Time::from_nanos),
                associations.iter().copied().map(Time::from_nanos),
                last.map(Time::from_nanos),
            )
        };
        assert_eq!(select(&[10, 20], &[10], None), Some(Time::from_nanos(10)));
        assert_eq!(
            select(&[10, 20], &[10], Some(10)),
            Some(Time::from_nanos(20))
        );
        assert_eq!(
            select(&[10, 20], &[10], Some(20)),
            Some(Time::from_nanos(20))
        );
        assert_eq!(select(&[20], &[20], Some(20)), Some(Time::from_nanos(20)));
        assert_eq!(
            select(&[15, 5, 10], &[10], Some(20)),
            Some(Time::from_nanos(20))
        );
        assert_eq!(
            select(&[], &[10, 30, 20], Some(20)),
            Some(Time::from_nanos(30))
        );
        assert_eq!(select(&[], &[10], Some(20)), Some(Time::from_nanos(20)));
        assert_eq!(select(&[], &[], Some(20)), Some(Time::from_nanos(20)));
        assert_eq!(select(&[5], &[5], None), Some(Time::from_nanos(5)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn expensive_observations_follow_layer_preferences() {
        let context = PanelCreationContext {
            backend: Arc::new(
                crate::backend::RobotBackend::new(
                    tokio::runtime::Handle::current(),
                    None,
                    "/".to_string(),
                )
                .await
                .unwrap(),
            ),
            value: None,
            egui_context: eframe::egui::Context::default(),
            render_state: None,
        };
        let mut observations = Observations::new(&context).unwrap();
        let mut settings = Settings {
            camera: false,
            associations: false,
            ..default()
        };
        observations.update_layers(&context, &settings).unwrap();
        assert!(observations.images.is_none() && observations.associations.is_none());
        settings.camera = true;
        settings.associations = true;
        observations.update_layers(&context, &settings).unwrap();
        assert!(observations.images.is_some() && observations.associations.is_some());
        settings.camera = false;
        settings.associations = false;
        observations.update_layers(&context, &settings).unwrap();
        assert!(observations.images.is_none() && observations.associations.is_none());
    }
}
