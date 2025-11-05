use std::sync::Arc;

use coordinate_systems::Pixel;
use eframe::{
    egui::{Color32, Pos2, Rect, Response, Stroke, Ui, Vec2, Widget},
    emath::RectTransform,
};
use geometry::circle::Circle;
use linear_algebra::{point, vector};
use serde_json::{json, Value};
use types::{
    ball_detection::CandidateEvaluation,
    ycbcr422_image::{YCbCr422Image, SAMPLE_SIZE},
};

use crate::{
    nao::Nao,
    panel::Panel,
    twix_painter::{Orientation, TwixPainter},
    value_buffer::BufferHandle,
};

pub struct BallCandidatePanel {
    ball_radius_enlargement_factor: BufferHandle<f32>,
    ball_candidates: BufferHandle<Option<Vec<CandidateEvaluation>>>,
    image: BufferHandle<YCbCr422Image>,
}

impl Panel for BallCandidatePanel {
    const NAME: &'static str = "Ball Candidates";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let ball_radius_enlargement_factor =
            nao.subscribe_value("parameters.ball_detection.vision.ball_radius_enlargement_factor");
        let ball_candidates = nao.subscribe_value("Vision.additional_outputs.ball_candidates");
        let image = nao.subscribe_value("Vision.main_outputs.image");
        Self {
            ball_radius_enlargement_factor,
            ball_candidates,
            image,
        }
    }

    fn save(&self) -> Value {
        json!({
            "cycler": "Vision",
        })
    }
}

impl Widget for &mut BallCandidatePanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            if let Some((ball_radius_enlargement_factor, ball_candidates, image)) = self
                .ball_radius_enlargement_factor
                .get_last_value()
                .ok()
                .flatten()
                .and_then(|ball_radius_enlargement_factor| {
                    self.ball_candidates
                        .get_last_value()
                        .ok()
                        .flatten()
                        .flatten()
                        .map(|ball_candidates| (ball_radius_enlargement_factor, ball_candidates))
                })
                .and_then(|(ball_radius_enlargement_factor, ball_candidates)| {
                    self.image
                        .get_last_value()
                        .ok()
                        .flatten()
                        .map(|image| (ball_radius_enlargement_factor, ball_candidates, image))
                })
            {
                ui.horizontal_wrapped(|ui| {
                    for candidate in ball_candidates {
                        ui.add(CandidateSample {
                            ball_radius_enlargement_factor,
                            candidate,
                            image: image.clone(),
                        });
                    }
                });
            } else {
                ui.label("Some outputs are missing");
            }
        })
        .response
    }
}

impl BallCandidatePanel {}

struct CandidateSample {
    ball_radius_enlargement_factor: f32,
    candidate: CandidateEvaluation,
    image: YCbCr422Image,
}

impl Widget for CandidateSample {
    fn ui(self, ui: &mut Ui) -> Response {
        let enlarged_candidate = Circle {
            center: self.candidate.candidate_circle.center,
            radius: self.candidate.candidate_circle.radius * self.ball_radius_enlargement_factor,
        };

        let sample = self.image.sample_grayscale(enlarged_candidate);

        const SAMPLE_SIZE_F32: f32 = SAMPLE_SIZE as f32;
        const SCALING: f32 = 3.0;
        ui.allocate_ui(
            Vec2::new(SAMPLE_SIZE_F32 * SCALING, SAMPLE_SIZE_F32 * SCALING),
            |ui| {
                let (response, painter) = TwixPainter::<Pixel>::allocate(
                    ui,
                    vector![SAMPLE_SIZE_F32, SAMPLE_SIZE_F32],
                    point![0.0, 0.0],
                    Orientation::LeftHanded,
                );

                for (y, sample_row) in sample.iter().enumerate() {
                    let y = y as f32;
                    for (x, sample_value) in sample_row.iter().enumerate() {
                        let x = x as f32;
                        painter.rect_filled(
                            point![x, y],
                            point![x + 1.0, y + 1.0],
                            Color32::from_gray(*sample_value as u8),
                        );
                    }
                }

                if let Some(corrected_circle) = self.candidate.corrected_circle {
                    let candidate_circle = self.candidate.candidate_circle;
                    let transform = RectTransform::from_to(
                        Rect::from_center_size(
                            Pos2::new(candidate_circle.center.x(), candidate_circle.center.y()),
                            Vec2::new(
                                candidate_circle.radius * 2.0 * self.ball_radius_enlargement_factor,
                                candidate_circle.radius * 2.0 * self.ball_radius_enlargement_factor,
                            ),
                        ),
                        Rect::from_min_size(
                            Pos2::ZERO,
                            Vec2::new(SAMPLE_SIZE_F32, SAMPLE_SIZE_F32),
                        ),
                    );
                    let corrected_center_in_sample = transform.transform_pos(Pos2::new(
                        corrected_circle.center.x(),
                        corrected_circle.center.y(),
                    ));
                    let corrected_center_in_sample =
                        point![corrected_center_in_sample.x, corrected_center_in_sample.y];
                    let corrected_radius_in_sample = corrected_circle.radius
                        / (self.candidate.candidate_circle.radius
                            * self.ball_radius_enlargement_factor)
                        * (SAMPLE_SIZE_F32 / 2.0);
                    painter.circle_stroke(
                        corrected_center_in_sample,
                        corrected_radius_in_sample,
                        Stroke::new(0.5, Color32::GREEN),
                    );
                }

                response
            },
        )
        .response
    }
}
