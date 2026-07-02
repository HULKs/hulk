use std::sync::{Arc, Mutex};

use coordinate_systems::{Camera, Field, Robot};
use eframe::{
    App, CreationContext, Frame,
    egui::{
        self, CentralPanel, Color32, ColorImage, Context, FontId, PointerButton, Pos2, Rect,
        RichText, Sense, SidePanel, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2,
        Widget, pos2, vec2,
    },
};
use egui_bevy::BevyWidget;
use field_mark_association::FieldMarkAssociations;
use linear_algebra::{Isometry3, Point2, Point3, point};
use projection::{camera_matrix::CameraMatrix, intrinsic::Intrinsic};
use ros_z::time::Time;
use tokio::runtime::Runtime;
use types::{
    field_dimensions::FieldDimensions,
    object_detection::{Object, RobocupObjectLabel},
};

use crate::{
    cli::Arguments,
    scene::{self, ViewerData},
    state::{AlignedViewerState, CameraFrame, PoseSource, SharedState, ViewerState},
    subscriptions,
};

use self::stabilizer::RenderSampleStabilizer;

mod header;
mod stabilizer;

pub(crate) struct RobotViewerApp {
    widget: BevyWidget,
    state: SharedState,
    namespace: String,
    router: String,
    pose_source: PoseSource,
    camera_texture: Option<TextureHandle>,
    camera_texture_sequence: u64,
    camera_zoom: f32,
    camera_pan: Vec2,
    show_projected_field_lines: bool,
    render_samples: RenderSampleStabilizer,
    _runtime: Arc<Runtime>,
}

const CAMERA_MIN_ZOOM: f32 = 1.0;
const CAMERA_MAX_ZOOM: f32 = 25.0;
const CAMERA_FOOTER_HEIGHT: f32 = 88.0;
const FIELD_LINE_SAMPLE_STEP: f32 = 0.05;
const PROJECTED_FIELD_LINE_STROKE: Stroke = Stroke {
    width: 2.0,
    color: Color32::from_rgba_premultiplied(80, 220, 255, 190),
};
const ASSOCIATION_RESIDUAL_STROKE: Stroke = Stroke {
    width: 2.0,
    color: Color32::from_rgba_premultiplied(255, 80, 200, 220),
};
const PROJECTED_ARC_INITIAL_SEGMENTS: usize = 16;
const PROJECTED_ARC_MAX_DEPTH: u8 = 10;
const PROJECTED_ARC_SCREEN_ERROR: f32 = 1.5;

impl RobotViewerApp {
    pub(crate) fn new(
        creation_context: &CreationContext,
        arguments: Arguments,
        runtime: Arc<Runtime>,
    ) -> Self {
        creation_context.egui_ctx.set_visuals(egui::Visuals::dark());

        let namespace = arguments.namespace();
        let router = arguments.router_display();
        let state = Arc::new(Mutex::new(ViewerState::default()));

        let mut widget = BevyWidget::new(
            creation_context
                .wgpu_render_state
                .clone()
                .expect("no wgpu render state found"),
        );
        scene::configure(&mut widget.bevy_app);
        widget.bevy_app.finish();
        widget.bevy_app.cleanup();

        subscriptions::spawn(
            &runtime,
            arguments,
            state.clone(),
            creation_context.egui_ctx.clone(),
        );

        Self {
            widget,
            state,
            namespace,
            router,
            pose_source: PoseSource::default(),
            camera_texture: None,
            camera_texture_sequence: 0,
            camera_zoom: 1.0,
            camera_pan: Vec2::ZERO,
            show_projected_field_lines: false,
            render_samples: RenderSampleStabilizer::default(),
            _runtime: runtime,
        }
    }
}

impl App for RobotViewerApp {
    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        let (status, mut aligned) = {
            let mut state = self
                .state
                .lock()
                .expect("viewer state lock should not be poisoned");
            (state.status_snapshot(), state.aligned_snapshot())
        };
        self.render_samples.stabilize(&mut aligned);

        self.update_camera_texture(
            context,
            aligned
                .camera_frame
                .as_ref()
                .map(|camera_frame| camera_frame.inner.as_ref()),
        );
        self.header(context, &status);
        self.camera_panel(context, &aligned);
        self.widget
            .bevy_app
            .world_mut()
            .insert_resource(ViewerData::from_aligned_state(aligned, self.pose_source));
        self.viewport(context);
    }
}

impl RobotViewerApp {
    fn update_camera_texture(&mut self, context: &Context, frame: Option<&CameraFrame>) {
        let Some(frame) = frame else {
            return;
        };
        if self.camera_texture_sequence == frame.sequence {
            return;
        }

        let image = ColorImage::from_rgba_unmultiplied(
            [frame.width as usize, frame.height as usize],
            &frame.rgba,
        );
        if let Some(texture) = &mut self.camera_texture {
            texture.set(image, TextureOptions::LINEAR);
        } else {
            self.camera_texture =
                Some(context.load_texture("robot_viewer_camera", image, TextureOptions::LINEAR));
        }
        self.camera_texture_sequence = frame.sequence;
    }

    fn camera_panel(&mut self, context: &Context, state: &AlignedViewerState) {
        SidePanel::right("camera_panel")
            .resizable(true)
            .default_width(440.0)
            .width_range(300.0..=900.0)
            .show(context, |ui| {
                ui.vertical(|ui| {
                    ui.heading("Camera");
                    ui.label(
                        RichText::new(subscriptions::CAMERA_IMAGE_TOPIC)
                            .monospace()
                            .color(Color32::GRAY),
                    );
                    ui.add_space(8.0);
                    self.camera_image(ui, state);
                });
            });
    }

    fn camera_image(&mut self, ui: &mut Ui, state: &AlignedViewerState) {
        let Some(frame) = state
            .camera_frame
            .as_ref()
            .map(|camera_frame| camera_frame.inner.as_ref())
        else {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("waiting for camera image").color(Color32::GRAY));
            });
            return;
        };
        let Some(texture_id) = self.camera_texture.as_ref().map(TextureHandle::id) else {
            return;
        };

        let image_size = vec2(frame.width as f32, frame.height as f32);
        let available = ui.available_size().max(vec2(1.0, 1.0));
        let viewport_size = vec2(available.x, (available.y - CAMERA_FOOTER_HEIGHT).max(1.0));

        let (viewport_rect, response) =
            ui.allocate_exact_size(viewport_size, Sense::click_and_drag());
        self.update_camera_view(ui, &response, viewport_rect, image_size);

        let image_rect = self.camera_image_rect(viewport_rect, image_size);
        ui.painter_at(viewport_rect).image(
            texture_id,
            image_rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );

        if self.show_projected_field_lines {
            draw_projected_field_lines(ui, viewport_rect, image_rect, image_size, state);
        }
        let detected_objects = state
            .detected_objects
            .as_ref()
            .map_or([].as_slice(), |objects| objects.inner.as_ref().as_slice());
        draw_detected_objects(ui, viewport_rect, image_rect, image_size, detected_objects);
        if let Some(associations) = state
            .field_mark_associations
            .as_ref()
            .map(|associations| associations.inner.as_ref())
        {
            draw_field_mark_associations(ui, viewport_rect, image_rect, image_size, associations);
        }

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("{}x{}", frame.width, frame.height));
            if let Some(time) = state.anchor_time {
                ui.separator();
                ui.label(format!("aligned {:.3}s", time.as_nanos() as f64 / 1.0e9));
            }
            if let (Some(anchor_time), Some(camera_matrix)) =
                (state.anchor_time, state.camera_matrix.as_ref())
            {
                ui.separator();
                ui.label(format!(
                    "matrix {:+.0}ms",
                    time_delta_ms(camera_matrix.time, anchor_time)
                ));
            }
            ui.separator();
            match &state.detected_objects {
                Some(objects) => ui.label(format!("{} detections", objects.inner.len())),
                None => ui.label(RichText::new("detections unavailable").color(Color32::GRAY)),
            };
            if let Some(associations) = &state.field_mark_associations {
                ui.separator();
                ui.label(format!(
                    "{} associations",
                    associations.inner.associations.len()
                ));
            }
            if let Some(intrinsics) = state.latest_calibrated_intrinsics {
                ui.separator();
                ui.label(format!(
                    "calibrated fx/fy {:.1}/{:.1} cx/cy {:.1}/{:.1}",
                    intrinsics.focals.x,
                    intrinsics.focals.y,
                    intrinsics.optical_center.x(),
                    intrinsics.optical_center.y(),
                ));
            }
            ui.separator();
            ui.label(format!("zoom {:.1}x", self.camera_zoom));
            ui.separator();
            ui.checkbox(&mut self.show_projected_field_lines, "project field lines");
        });
    }

    fn update_camera_view(
        &mut self,
        ui: &Ui,
        response: &egui::Response,
        viewport_rect: Rect,
        image_size: Vec2,
    ) {
        if response.double_clicked_by(PointerButton::Primary)
            || response.double_clicked_by(PointerButton::Secondary)
        {
            self.camera_zoom = 1.0;
            self.camera_pan = Vec2::ZERO;
            return;
        }

        if response.dragged() {
            self.camera_pan += ui.input(|input| input.pointer.delta());
        }

        let Some(pointer) = response.hover_pos() else {
            return;
        };
        let scroll_y = ui.input(|input| input.smooth_scroll_delta.y);
        if scroll_y.abs() <= f32::EPSILON {
            return;
        }

        let old_zoom = self.camera_zoom;
        let zoom_factor = 1.01_f32.powf(scroll_y);
        let new_zoom = (old_zoom * zoom_factor).clamp(CAMERA_MIN_ZOOM, CAMERA_MAX_ZOOM);
        if (new_zoom - old_zoom).abs() <= f32::EPSILON {
            return;
        }

        let fit_scale = fitted_image_scale(viewport_rect.size(), image_size);
        let old_rect = camera_image_rect(viewport_rect, image_size, old_zoom, self.camera_pan);
        let old_scale = fit_scale * old_zoom;
        let image_pixel_under_pointer = (pointer - old_rect.min) / old_scale.max(f32::EPSILON);

        let new_scale = fit_scale * new_zoom;
        let new_size = image_size * new_scale;
        let new_min = pointer
            - vec2(
                image_pixel_under_pointer.x * new_scale,
                image_pixel_under_pointer.y * new_scale,
            );
        let new_center = new_min + new_size * 0.5;

        self.camera_zoom = new_zoom;
        self.camera_pan = new_center - viewport_rect.center();
    }

    fn camera_image_rect(&self, viewport_rect: Rect, image_size: Vec2) -> Rect {
        camera_image_rect(viewport_rect, image_size, self.camera_zoom, self.camera_pan)
    }

    fn viewport(&mut self, context: &Context) {
        CentralPanel::default()
            .frame(egui::Frame::central_panel(&context.style()).fill(Color32::from_rgb(16, 18, 22)))
            .show(context, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("3D Field");
                        ui.label(RichText::new("pan/zoom with mouse").color(Color32::GRAY));
                    });
                    ui.add_space(6.0);
                    self.widget.ui(ui);
                });
            });
    }
}

fn camera_image_rect(viewport_rect: Rect, image_size: Vec2, zoom: f32, pan: Vec2) -> Rect {
    let scale = fitted_image_scale(viewport_rect.size(), image_size) * zoom;
    let size = image_size * scale;
    Rect::from_center_size(viewport_rect.center() + pan, size)
}

fn fitted_image_scale(viewport_size: Vec2, image_size: Vec2) -> f32 {
    (viewport_size.x / image_size.x.max(1.0))
        .min(viewport_size.y / image_size.y.max(1.0))
        .max(0.05)
}

fn time_delta_ms(sample_time: Time, anchor_time: Time) -> f64 {
    (sample_time.as_nanos() as i128 - anchor_time.as_nanos() as i128) as f64 / 1.0e6
}

fn draw_detected_objects(
    ui: &mut Ui,
    clip_rect: Rect,
    image_rect: Rect,
    image_size: Vec2,
    detected_objects: &[Object<RobocupObjectLabel>],
) {
    let scale = vec2(
        image_rect.width() / image_size.x.max(1.0),
        image_rect.height() / image_size.y.max(1.0),
    );
    let painter = ui.painter_at(clip_rect);

    for object in detected_objects {
        let color = object_label_color(object.label);
        let min = image_rect.min
            + vec2(
                object.bounding_box.area.min.x() * scale.x,
                object.bounding_box.area.min.y() * scale.y,
            );
        let max = image_rect.min
            + vec2(
                object.bounding_box.area.max.x() * scale.x,
                object.bounding_box.area.max.y() * scale.y,
            );
        let rect = Rect::from_min_max(min, max).intersect(clip_rect);

        painter.rect_stroke(
            rect,
            egui::CornerRadius::same(4),
            Stroke::new(2.0, color),
            StrokeKind::Outside,
        );

        let label: String = object.label.into();
        let text = format!("{label} {:.0}%", object.bounding_box.confidence * 100.0);
        let text_position = pos2(rect.min.x + 5.0, rect.min.y + 5.0);
        let galley = painter.layout_no_wrap(text, FontId::proportional(13.0), Color32::WHITE);
        let label_rect = Rect::from_min_size(
            text_position - vec2(3.0, 2.0),
            galley.size() + vec2(6.0, 4.0),
        );
        painter.rect_filled(
            label_rect,
            egui::CornerRadius::same(3),
            color.gamma_multiply(0.85),
        );
        painter.galley(text_position, galley, Color32::WHITE);
    }
}

fn draw_field_mark_associations(
    ui: &mut Ui,
    clip_rect: Rect,
    image_rect: Rect,
    image_size: Vec2,
    associations: &FieldMarkAssociations,
) {
    let scale = vec2(
        image_rect.width() / image_size.x.max(1.0),
        image_rect.height() / image_size.y.max(1.0),
    );
    let painter = ui.painter_at(clip_rect);
    for (index, association) in associations.associations.iter().enumerate() {
        let position = image_rect.min
            + vec2(
                association.detection.x() * scale.x,
                association.detection.y() * scale.y,
            );
        if !clip_rect.contains(position) {
            continue;
        }
        painter.circle_stroke(
            position,
            7.0,
            Stroke::new(2.5, Color32::from_rgb(255, 80, 200)),
        );
        painter.text(
            position + vec2(8.0, -8.0),
            egui::Align2::LEFT_TOP,
            index.to_string(),
            FontId::proportional(12.0),
            Color32::from_rgb(255, 180, 235),
        );
    }
}

fn draw_projected_field_lines(
    ui: &mut Ui,
    clip_rect: Rect,
    image_rect: Rect,
    image_size: Vec2,
    state: &AlignedViewerState,
) {
    let (Some(field_to_robot), Some(camera_matrix)) = (
        state.latest_localization,
        state
            .camera_matrix
            .as_ref()
            .map(|sample| sample.inner.as_ref()),
    ) else {
        return;
    };
    let dimensions = state.field_dimensions.unwrap_or(FieldDimensions::SPL_2025);
    let intrinsics = state
        .latest_calibrated_intrinsics
        .unwrap_or(camera_matrix.intrinsics);
    let robot_to_camera = robot_to_camera(camera_matrix);
    let field_to_camera = robot_to_camera * field_to_robot;
    let painter = ui.painter_at(clip_rect);

    draw_projected_field_markings(
        &painter,
        image_rect,
        image_size,
        &field_to_camera,
        intrinsics,
        dimensions,
    );
    if let Some(associations) = state
        .field_mark_associations
        .as_ref()
        .map(|associations| associations.inner.as_ref())
    {
        draw_projected_field_mark_association_residuals(
            &painter,
            image_rect,
            image_size,
            &field_to_camera,
            intrinsics,
            associations,
        );
    }
}

fn robot_to_camera(camera_matrix: &CameraMatrix) -> Isometry3<Robot, Camera> {
    camera_matrix.head_to_camera * camera_matrix.robot_to_head
}

fn draw_projected_field_markings(
    painter: &egui::Painter,
    image_rect: Rect,
    image_size: Vec2,
    field_to_camera: &Isometry3<Field, Camera>,
    intrinsics: Intrinsic,
    dimensions: FieldDimensions,
) {
    let half_length = dimensions.length / 2.0;
    let half_width = dimensions.width / 2.0;

    draw_projected_rect(
        painter,
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        -half_length,
        -half_width,
        half_length,
        half_width,
    );
    draw_projected_segment(
        painter,
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        [0.0, -half_width],
        [0.0, half_width],
    );
    draw_projected_arc(
        painter,
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        [0.0, 0.0],
        dimensions.center_circle_diameter / 2.0,
        0.0,
        std::f32::consts::TAU,
    );

    for sign in [-1.0, 1.0] {
        draw_projected_goal_area(
            painter,
            image_rect,
            image_size,
            field_to_camera,
            intrinsics,
            dimensions,
            sign,
            dimensions.goal_box_area_length,
            dimensions.goal_box_area_width,
        );
        draw_projected_goal_area(
            painter,
            image_rect,
            image_size,
            field_to_camera,
            intrinsics,
            dimensions,
            sign,
            dimensions.penalty_area_length,
            dimensions.penalty_area_width,
        );

        let penalty_x = sign * (half_length - dimensions.penalty_marker_distance);
        draw_projected_marker_cross(
            painter,
            image_rect,
            image_size,
            field_to_camera,
            intrinsics,
            [penalty_x, 0.0],
            dimensions.penalty_marker_size,
        );

        if dimensions.corner_arc_radius > 0.0 {
            draw_projected_corner_arcs(
                painter,
                image_rect,
                image_size,
                field_to_camera,
                intrinsics,
                dimensions,
                sign,
            );
        }
    }
}

fn draw_projected_field_mark_association_residuals(
    painter: &egui::Painter,
    image_rect: Rect,
    image_size: Vec2,
    field_to_camera: &Isometry3<Field, Camera>,
    intrinsics: Intrinsic,
    associations: &FieldMarkAssociations,
) {
    let scale = vec2(
        image_rect.width() / image_size.x.max(1.0),
        image_rect.height() / image_size.y.max(1.0),
    );

    for association in &associations.associations {
        let detection_position = image_rect.min
            + vec2(
                association.detection.x() * scale.x,
                association.detection.y() * scale.y,
            );
        let Some(projected_position) = project_field_point3_to_image(
            image_rect,
            image_size,
            field_to_camera,
            intrinsics,
            association.field_point,
        ) else {
            continue;
        };

        painter.line_segment(
            [detection_position, projected_position],
            ASSOCIATION_RESIDUAL_STROKE,
        );
        painter.circle_filled(projected_position, 3.5, ASSOCIATION_RESIDUAL_STROKE.color);
    }
}

fn draw_projected_rect(
    painter: &egui::Painter,
    image_rect: Rect,
    image_size: Vec2,
    field_to_camera: &Isometry3<Field, Camera>,
    intrinsics: Intrinsic,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
) {
    draw_projected_segment(
        painter,
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        [min_x, min_y],
        [max_x, min_y],
    );
    draw_projected_segment(
        painter,
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        [max_x, min_y],
        [max_x, max_y],
    );
    draw_projected_segment(
        painter,
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        [max_x, max_y],
        [min_x, max_y],
    );
    draw_projected_segment(
        painter,
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        [min_x, max_y],
        [min_x, min_y],
    );
}

fn draw_projected_goal_area(
    painter: &egui::Painter,
    image_rect: Rect,
    image_size: Vec2,
    field_to_camera: &Isometry3<Field, Camera>,
    intrinsics: Intrinsic,
    dimensions: FieldDimensions,
    sign: f32,
    length: f32,
    width: f32,
) {
    let goal_line_x = sign * dimensions.length / 2.0;
    let inner_x = goal_line_x - sign * length;
    let half_width = width / 2.0;

    draw_projected_segment(
        painter,
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        [goal_line_x, -half_width],
        [inner_x, -half_width],
    );
    draw_projected_segment(
        painter,
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        [inner_x, -half_width],
        [inner_x, half_width],
    );
    draw_projected_segment(
        painter,
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        [inner_x, half_width],
        [goal_line_x, half_width],
    );
}

fn draw_projected_marker_cross(
    painter: &egui::Painter,
    image_rect: Rect,
    image_size: Vec2,
    field_to_camera: &Isometry3<Field, Camera>,
    intrinsics: Intrinsic,
    center: [f32; 2],
    size: f32,
) {
    let half_size = size / 2.0;
    draw_projected_segment(
        painter,
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        [center[0] - half_size, center[1]],
        [center[0] + half_size, center[1]],
    );
    draw_projected_segment(
        painter,
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        [center[0], center[1] - half_size],
        [center[0], center[1] + half_size],
    );
}

fn draw_projected_corner_arcs(
    painter: &egui::Painter,
    image_rect: Rect,
    image_size: Vec2,
    field_to_camera: &Isometry3<Field, Camera>,
    intrinsics: Intrinsic,
    dimensions: FieldDimensions,
    sign: f32,
) {
    let half_length = dimensions.length / 2.0;
    let half_width = dimensions.width / 2.0;
    for side in [-1.0, 1.0] {
        let center = [sign * half_length, side * half_width];
        let start = if sign > 0.0 { 0.5 } else { 0.0 };
        let start = std::f32::consts::PI * (start + if side > 0.0 { 0.0 } else { 1.0 });
        draw_projected_arc(
            painter,
            image_rect,
            image_size,
            field_to_camera,
            intrinsics,
            center,
            dimensions.corner_arc_radius,
            start,
            start + std::f32::consts::FRAC_PI_2,
        );
    }
}

fn draw_projected_segment(
    painter: &egui::Painter,
    image_rect: Rect,
    image_size: Vec2,
    field_to_camera: &Isometry3<Field, Camera>,
    intrinsics: Intrinsic,
    start: [f32; 2],
    end: [f32; 2],
) {
    let delta = [end[0] - start[0], end[1] - start[1]];
    let distance = delta[0].hypot(delta[1]);
    let samples = (distance / FIELD_LINE_SAMPLE_STEP).ceil().max(1.0) as usize;
    let mut previous = None;

    for index in 0..=samples {
        let t = index as f32 / samples as f32;
        let point = [start[0] + delta[0] * t, start[1] + delta[1] * t];
        draw_projected_point_step(
            painter,
            image_rect,
            image_size,
            field_to_camera,
            intrinsics,
            point,
            &mut previous,
        );
    }
}

fn draw_projected_arc(
    painter: &egui::Painter,
    image_rect: Rect,
    image_size: Vec2,
    field_to_camera: &Isometry3<Field, Camera>,
    intrinsics: Intrinsic,
    center: [f32; 2],
    radius: f32,
    start: f32,
    end: f32,
) {
    if radius <= 0.0 {
        return;
    }

    let samples = ((radius * (end - start).abs()) / FIELD_LINE_SAMPLE_STEP)
        .ceil()
        .clamp(8.0, 160.0) as usize;
    let samples = samples.max(PROJECTED_ARC_INITIAL_SEGMENTS);

    for index in 0..samples {
        let start_angle = start + (end - start) * index as f32 / samples as f32;
        let end_angle = start + (end - start) * (index + 1) as f32 / samples as f32;
        draw_projected_arc_segment(
            painter,
            image_rect,
            image_size,
            field_to_camera,
            intrinsics,
            center,
            radius,
            start_angle,
            end_angle,
            0,
        );
    }
}

fn draw_projected_arc_segment(
    painter: &egui::Painter,
    image_rect: Rect,
    image_size: Vec2,
    field_to_camera: &Isometry3<Field, Camera>,
    intrinsics: Intrinsic,
    center: [f32; 2],
    radius: f32,
    start: f32,
    end: f32,
    depth: u8,
) {
    let middle = 0.5 * (start + end);
    let start_position = project_arc_point_to_image(
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        center,
        radius,
        start,
    );
    let middle_position = project_arc_point_to_image(
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        center,
        radius,
        middle,
    );
    let end_position = project_arc_point_to_image(
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        center,
        radius,
        end,
    );

    if let (Some(start_position), Some(middle_position), Some(end_position)) =
        (start_position, middle_position, end_position)
    {
        let error = distance_to_segment(middle_position, start_position, end_position);
        if error <= PROJECTED_ARC_SCREEN_ERROR || depth >= PROJECTED_ARC_MAX_DEPTH {
            painter.line_segment([start_position, end_position], PROJECTED_FIELD_LINE_STROKE);
            return;
        }
    } else if depth >= PROJECTED_ARC_MAX_DEPTH {
        return;
    }

    draw_projected_arc_segment(
        painter,
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        center,
        radius,
        start,
        middle,
        depth + 1,
    );
    draw_projected_arc_segment(
        painter,
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        center,
        radius,
        middle,
        end,
        depth + 1,
    );
}

fn project_arc_point_to_image(
    image_rect: Rect,
    image_size: Vec2,
    field_to_camera: &Isometry3<Field, Camera>,
    intrinsics: Intrinsic,
    center: [f32; 2],
    radius: f32,
    angle: f32,
) -> Option<Pos2> {
    project_field_point_to_image(
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        [
            center[0] + radius * angle.cos(),
            center[1] + radius * angle.sin(),
        ],
    )
}

fn distance_to_segment(point: Pos2, start: Pos2, end: Pos2) -> f32 {
    let segment = end - start;
    let length_squared = segment.x * segment.x + segment.y * segment.y;
    if length_squared <= f32::EPSILON {
        return point.distance(start);
    }
    let offset = point - start;
    let t = ((offset.x * segment.x + offset.y * segment.y) / length_squared).clamp(0.0, 1.0);
    point.distance(start + segment * t)
}

fn draw_projected_point_step(
    painter: &egui::Painter,
    image_rect: Rect,
    image_size: Vec2,
    field_to_camera: &Isometry3<Field, Camera>,
    intrinsics: Intrinsic,
    point: [f32; 2],
    previous: &mut Option<Pos2>,
) {
    match project_field_point_to_image(image_rect, image_size, field_to_camera, intrinsics, point) {
        Some(position) => {
            if let Some(previous) = previous {
                painter.line_segment([*previous, position], PROJECTED_FIELD_LINE_STROKE);
            }
            *previous = Some(position);
        }
        None => *previous = None,
    }
}

fn project_field_point_to_image(
    image_rect: Rect,
    image_size: Vec2,
    field_to_camera: &Isometry3<Field, Camera>,
    intrinsics: Intrinsic,
    field_point: [f32; 2],
) -> Option<Pos2> {
    let point: Point2<Field> = point![<Field>, field_point[0], field_point[1]];
    project_field_point3_to_image(
        image_rect,
        image_size,
        field_to_camera,
        intrinsics,
        point.extend(0.0),
    )
}

fn project_field_point3_to_image(
    image_rect: Rect,
    image_size: Vec2,
    field_to_camera: &Isometry3<Field, Camera>,
    intrinsics: Intrinsic,
    field_point: Point3<Field>,
) -> Option<Pos2> {
    let camera_point = field_to_camera * field_point;
    if !camera_point.inner.iter().all(|value| value.is_finite()) || camera_point.z() <= 1.0e-4 {
        return None;
    }

    let pixel = intrinsics.project(camera_point.coords());
    if !pixel.inner.iter().all(|value| value.is_finite()) {
        return None;
    }

    let scale = vec2(
        image_rect.width() / image_size.x.max(1.0),
        image_rect.height() / image_size.y.max(1.0),
    );
    Some(image_rect.min + vec2(pixel.x() * scale.x, pixel.y() * scale.y))
}

fn object_label_color(label: RobocupObjectLabel) -> Color32 {
    match label {
        RobocupObjectLabel::Ball => Color32::from_rgb(255, 145, 64),
        RobocupObjectLabel::GoalPost => Color32::from_rgb(245, 245, 245),
        RobocupObjectLabel::Robot => Color32::from_rgb(82, 170, 255),
        RobocupObjectLabel::PenaltySpot => Color32::from_rgb(255, 230, 96),
        RobocupObjectLabel::LSpot | RobocupObjectLabel::TSpot | RobocupObjectLabel::XSpot => {
            Color32::from_rgb(120, 255, 170)
        }
    }
}
