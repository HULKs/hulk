use std::ops::Range;

use eframe::egui::{Color32, Rect, Shape, Stroke, TextStyle, Ui, pos2, text::LayoutJob};
use egui_plot::{PlotBounds, PlotGeometry, PlotItem, PlotItemBase, PlotPoint, PlotTransform};
use ros_z::time::Time;

use super::history::{StateInterval, seconds_from};

const PALETTE: [Color32; 8] = [
    Color32::from_rgb(78, 121, 167),
    Color32::from_rgb(242, 142, 43),
    Color32::from_rgb(89, 161, 79),
    Color32::from_rgb(225, 87, 89),
    Color32::from_rgb(176, 122, 161),
    Color32::from_rgb(237, 201, 72),
    Color32::from_rgb(118, 183, 178),
    Color32::from_rgb(156, 117, 95),
];
// Faint fills keep the plotted lines in the foreground.
const FILL_OPACITY: f32 = 0.07;
const BOUNDARY_OPACITY: f32 = 0.25;
const LABEL_PADDING: f32 = 4.0;
/// Spans narrower than this many pixels are merged into one neutral run, so
/// fast-changing states draw a bounded number of shapes.
const MIN_SPAN_WIDTH: f32 = 1.0;

/// Background color of a state, stable across sessions for a given schema.
pub(super) fn state_color(index: u32) -> Color32 {
    PALETTE[index as usize % PALETTE.len()]
}

/// One displayed state run, in seconds relative to the plot's time origin.
pub(super) struct StateSpan<'a> {
    pub x: Range<f64>,
    pub name: &'a str,
    pub index: u32,
}

/// Clip state runs to the displayed window. Open runs extend to its end.
pub(super) fn visible_spans<'a>(
    states: &'a [StateInterval],
    start: Time,
    end: Time,
) -> impl Iterator<Item = StateSpan<'a>> {
    states.iter().filter_map(move |state| {
        let state_end = state.end.unwrap_or(end).min(end);
        let state_start = state.start.max(start);
        (state_start < state_end).then(|| StateSpan {
            x: seconds_from(state_start, end)..seconds_from(state_end, end),
            name: &state.name,
            index: state.index,
        })
    })
}

/// Vertical share of the plot frame used by one of `count` stacked lanes.
pub(super) fn lane_rect(frame: Rect, lane: usize, count: usize) -> Rect {
    let height = frame.height() / count.max(1) as f32;
    let top = frame.top() + lane as f32 * height;
    Rect::from_x_y_ranges(frame.x_range(), top..=top + height)
}

/// Labeled state intervals of one state item, drawn behind numeric lines.
/// Each state item gets its own lane, so several enums can be compared
/// without overlapping fills.
pub(super) struct StateLane<'a> {
    pub label: String,
    pub spans: Vec<StateSpan<'a>>,
    pub lane: usize,
}

/// Find the lane and state under a hovered plot position, using the bounds
/// that map the plot frame to plot coordinates.
pub(super) fn state_at<'a>(
    lanes: &'a [StateLane<'a>],
    bounds: &PlotBounds,
    position: PlotPoint,
) -> Option<(&'a StateLane<'a>, &'a StateSpan<'a>)> {
    let height = bounds.max()[1] - bounds.min()[1];
    let fraction = (bounds.max()[1] - position.y) / height;
    if !(0.0..1.0).contains(&fraction) {
        return None;
    }
    let index = (fraction * lanes.len() as f64) as usize;
    let lane = lanes.iter().find(|lane| lane.lane == index)?;
    let span = lane
        .spans
        .iter()
        .find(|span| span.x.contains(&position.x))?;
    Some((lane, span))
}

/// Adjacent spans too narrow to tell apart, drawn as one neutral fill.
#[derive(Default)]
struct DenseRun(Option<Rect>);

impl DenseRun {
    fn extend(&mut self, rect: Rect, shapes: &mut Vec<Shape>, color: Color32) {
        match &mut self.0 {
            Some(run) if rect.left() - run.right() < MIN_SPAN_WIDTH => *run = run.union(rect),
            _ => {
                self.flush(shapes, color);
                self.0 = Some(rect);
            }
        }
    }

    fn flush(&mut self, shapes: &mut Vec<Shape>, color: Color32) {
        if let Some(run) = self.0.take() {
            shapes.push(Shape::rect_filled(
                run,
                0.0,
                color.gamma_multiply(FILL_OPACITY),
            ));
        }
    }
}

/// Plot item for a [`StateLane`]. It has no Y extent, so it never affects
/// the numeric axis bounds.
pub(super) struct StateLaneItem<'a> {
    base: PlotItemBase,
    lane: &'a StateLane<'a>,
    lanes: usize,
}

impl<'a> StateLaneItem<'a> {
    pub fn new(lane: &'a StateLane<'a>, lanes: usize) -> Self {
        Self {
            base: PlotItemBase::new(lane.label.clone()),
            lane,
            lanes,
        }
    }
}

impl PlotItem for StateLaneItem<'_> {
    fn shapes(&self, ui: &Ui, transform: &PlotTransform, shapes: &mut Vec<Shape>) {
        let lane = lane_rect(*transform.frame(), self.lane.lane, self.lanes);
        let text_color = ui.visuals().weak_text_color();
        let mut dense = DenseRun::default();
        for span in &self.lane.spans {
            let left = transform.position_from_point_x(span.x.start);
            let right = transform.position_from_point_x(span.x.end);
            let rect = Rect::from_x_y_ranges(left..=right, lane.y_range()).intersect(lane);
            if !rect.is_positive() {
                continue;
            }
            if rect.width() < MIN_SPAN_WIDTH {
                dense.extend(rect, shapes, text_color);
                continue;
            }
            dense.flush(shapes, text_color);
            let color = state_color(span.index);
            shapes.push(Shape::rect_filled(
                rect,
                0.0,
                color.gamma_multiply(FILL_OPACITY),
            ));
            if lane.x_range().contains(left) {
                shapes.push(Shape::vline(
                    left,
                    lane.y_range(),
                    Stroke::new(1.0, color.gamma_multiply(BOUNDARY_OPACITY)),
                ));
            }
            let width = rect.width() - 2.0 * LABEL_PADDING;
            if width > 0.0 {
                let mut job = LayoutJob::simple_singleline(
                    span.name.to_owned(),
                    TextStyle::Small.resolve(ui.style()),
                    text_color,
                );
                job.wrap.max_width = width;
                job.wrap.max_rows = 1;
                job.wrap.break_anywhere = true;
                job.wrap.overflow_character = Some('…');
                let galley = ui.painter().layout_job(job);
                if galley.size().y + 2.0 * LABEL_PADDING <= rect.height() {
                    shapes.push(Shape::galley(
                        pos2(rect.left() + LABEL_PADDING, rect.top() + LABEL_PADDING),
                        galley,
                        text_color,
                    ));
                }
            }
        }
        dense.flush(shapes, text_color);
    }

    fn initialize(&mut self, _x_range: std::ops::RangeInclusive<f64>) {}

    /// Variants have their own colors, so the item has none.
    fn color(&self) -> Color32 {
        Color32::TRANSPARENT
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::None
    }

    fn bounds(&self) -> PlotBounds {
        PlotBounds::NOTHING
    }

    fn base(&self) -> &PlotItemBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut PlotItemBase {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::{pos2, vec2};

    use super::*;

    fn interval(start: i64, end: Option<i64>, index: u32) -> StateInterval {
        StateInterval {
            start: Time::from_nanos(start * 1_000_000_000),
            end: end.map(|end| Time::from_nanos(end * 1_000_000_000)),
            index,
            name: format!("S{index}"),
        }
    }

    #[test]
    fn spans_are_clipped_to_window_and_open_runs_reach_the_end() {
        let states = [
            interval(0, Some(4), 0),
            interval(4, Some(6), 1),
            interval(8, None, 2),
        ];

        let spans: Vec<_> = visible_spans(
            &states,
            Time::from_nanos(2_000_000_000),
            Time::from_nanos(10_000_000_000),
        )
        .map(|span| (span.x, span.index))
        .collect();

        assert_eq!(spans, [(-8.0..-6.0, 0), (-6.0..-4.0, 1), (-2.0..0.0, 2)]);
    }

    #[test]
    fn spans_outside_window_are_skipped() {
        let states = [interval(0, Some(1), 0), interval(12, None, 1)];

        let spans = visible_spans(
            &states,
            Time::from_nanos(2_000_000_000),
            Time::from_nanos(10_000_000_000),
        );

        assert_eq!(spans.count(), 0);
    }

    #[test]
    fn hovered_position_finds_the_lane_and_state() {
        let states = [interval(0, Some(4), 0), interval(4, None, 1)];
        let window = (Time::from_nanos(0), Time::from_nanos(10_000_000_000));
        let lanes: Vec<_> = (0..2)
            .map(|lane| StateLane {
                label: format!("lane {lane}"),
                spans: visible_spans(&states, window.0, window.1).collect(),
                lane,
            })
            .collect();
        let bounds = PlotBounds::from_min_max([-10.0, -1.0], [0.0, 1.0]);

        let (lane, span) = state_at(&lanes, &bounds, PlotPoint::new(-8.0, 0.5)).unwrap();
        assert_eq!((lane.lane, span.name), (0, "S0"));
        let (lane, span) = state_at(&lanes, &bounds, PlotPoint::new(-2.0, -0.5)).unwrap();
        assert_eq!((lane.lane, span.name), (1, "S1"));
        assert!(state_at(&lanes, &bounds, PlotPoint::new(-2.0, 2.0)).is_none());
    }

    #[test]
    fn lanes_split_the_frame_height() {
        let frame = Rect::from_min_size(pos2(0.0, 10.0), vec2(100.0, 90.0));

        assert_eq!(lane_rect(frame, 0, 1), frame);
        assert_eq!(
            lane_rect(frame, 2, 3),
            Rect::from_min_max(pos2(0.0, 70.0), pos2(100.0, 100.0))
        );
    }

    #[test]
    fn narrow_spans_merge_into_one_shape() {
        let lane = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 10.0));
        let mut shapes = Vec::new();
        let mut dense = DenseRun::default();
        for index in 0..50 {
            let left = index as f32 * 0.2;
            let rect = Rect::from_x_y_ranges(left..=left + 0.2, lane.y_range());
            dense.extend(rect, &mut shapes, Color32::WHITE);
        }
        dense.extend(
            Rect::from_x_y_ranges(50.0..=50.5, lane.y_range()),
            &mut shapes,
            Color32::WHITE,
        );
        dense.flush(&mut shapes, Color32::WHITE);

        assert_eq!(shapes.len(), 2);
        assert_eq!(shapes[0].visual_bounding_rect().width(), 10.0);
    }
}
