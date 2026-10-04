use eframe::egui::{Color32, RichText, Sense, Stroke, TextStyle, Ui, vec2};
use ros_z_debug::TopicObservationStatus;

use crate::backend::RobotBackend;

pub const TEAM_BALL_COLOR: Color32 = Color32::from_rgb(180, 100, 255);

pub fn ball_filter_status(
    backend: &RobotBackend,
    count: Option<usize>,
    status: TopicObservationStatus,
) -> String {
    let error = match &status {
        TopicObservationStatus::Observing { cache } => cache.message().map(str::to_owned),
        TopicObservationStatus::Retrying { error, .. } => Some(error.clone()),
        _ => None,
    };
    if let Some(error) = error {
        return format!("Ball Filter: {error}");
    }
    match count {
        Some(0) => "Ball Filter: no candidates in the received data".to_owned(),
        Some(count) => format!("Ball Filter: {count} candidates"),
        None => {
            let TopicObservationStatus::Observing { cache } = status else {
                return "Ball Filter: connecting to candidate data".to_owned();
            };
            let (Some(topic), Some(expected)) = (cache.resolved_topic(), cache.type_info()) else {
                return "Ball Filter: waiting for candidate data".to_owned();
            };
            let graph = backend.graph().lock();
            let mut publishers = graph.publishers_on(topic).peekable();
            if publishers.peek().is_none() {
                return format!("Ball Filter: no publisher discovered for {topic}");
            }
            if !publishers.any(|publisher| publisher.type_info == *expected) {
                return format!(
                    "Ball Filter: incompatible message format on {topic}; robot and Twix builds must match"
                );
            }
            format!("Ball Filter: publisher found on {topic}, waiting for samples")
        }
    }
}

#[derive(Default)]
pub struct BallLegend {
    pub percepts: bool,
    pub position: bool,
    pub filter: bool,
}

impl BallLegend {
    pub fn show(&self, ui: &mut Ui) {
        if !(self.percepts || self.position || self.filter) {
            return;
        }
        ui.horizontal_wrapped(|ui| {
            if self.percepts {
                legend_entry(ui, &[Color32::GREEN], "Ball Percepts");
            }
            if self.position {
                legend_entry(ui, &[Color32::BLUE], "Ball Filter");
                legend_entry(ui, &[TEAM_BALL_COLOR], "Team ball");
            }
            if self.filter {
                legend_entry(ui, &[Color32::GRAY], "Ball Filter Candidates");
            }
        });
    }
}

fn legend_entry(ui: &mut Ui, colors: &[Color32], label: &str) {
    ui.horizontal(|ui| {
        for color in colors {
            let (rect, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
            ui.painter()
                .circle(rect.center(), 5.0, *color, Stroke::new(1.0, Color32::BLACK));
        }
        let font_size = TextStyle::Small.resolve(ui.style()).size + 1.0;
        ui.label(RichText::new(label).size(font_size));
    });
}

// None means an incomplete snapshot; Some(None) means a complete snapshot with
// no selected ball. Never classify a newer state using an older selection.
pub fn selected_candidate(
    filter: &ros_z_debug::SampleRecord<ball_filter::BallFilter>,
    selected: &ros_z_debug::SampleRecord<Option<ball_filter::BallHypothesis>>,
) -> Option<Option<usize>> {
    if filter.source_time != selected.source_time {
        return None;
    }
    match &selected.value {
        None => Some(None),
        Some(selected) => filter
            .value
            .hypotheses
            .iter()
            .position(|hypothesis| {
                selected.last_seen == hypothesis.last_seen
                    && selected.validity == hypothesis.validity
                    && selected.position().position == hypothesis.position().position
            })
            .map(Some),
    }
}

pub fn detection_for_percept<'a>(
    percept: &types::ball_detection::BallPercept,
    detections: &'a [types::object_detection::Object<
        types::object_detection::RobocupObjectLabel,
    >],
) -> Option<&'a types::object_detection::Object<types::object_detection::RobocupObjectLabel>> {
    detections.iter().find(|detection| {
        let area = detection.bounding_box.area;
        detection.label == types::object_detection::RobocupObjectLabel::Ball
            && area.center() == percept.image_location.center
            && (area.max.x() - area.min.x()).min(area.max.y() - area.min.y()) / 2.0
                == percept.image_location.radius
    })
}
