use std::{
    collections::{BTreeMap, VecDeque},
    hash::{Hash, Hasher},
    time::{Duration, Instant},
};

use crate::{
    model::{should_emit_scrub_command, DisplayedRecord, WorkerCommand},
    timeline_canvas::timeline_lane_window_capacity,
};

use super::state::{
    LaneRenderPoint, LaneRenderRow, TimelineLaneKey, TimelineRenderRange, TimelineViewportState,
    ViewerApp, DEFAULT_TIMELINE_LANE_HEIGHT_PX, MIN_TIMELINE_SPAN,
};

impl ViewerApp {
    pub(super) fn current_anchor_nanos(&self) -> Option<u64> {
        self.timeline
            .global_timeline_index
            .and_then(|index| self.timeline.global_timeline.get(index).copied())
    }

    pub(super) fn timeline_full_range(&self) -> Option<TimelineRenderRange> {
        let start_ns = self.timeline.global_timeline.first().copied()?;
        let end_ns = self.timeline.global_timeline.last().copied()?;
        Some(TimelineRenderRange { start_ns, end_ns })
    }

    pub(super) fn timeline_render_range(&self) -> Option<TimelineRenderRange> {
        let full_range = self.timeline_full_range()?;
        Some(derive_timeline_render_range(
            full_range,
            self.timeline.timeline_viewport,
            self.ui.follow_live,
        ))
    }

    pub(super) fn insert_global_timestamp(&mut self, timestamp_nanos: u64) {
        if let Some(last) = self.timeline.global_timeline.last().copied() {
            if timestamp_nanos > last {
                self.timeline.global_timeline.push(timestamp_nanos);
                trim_timeline_to_capacity(
                    &mut self.timeline.global_timeline,
                    &mut self.timeline.global_timeline_index,
                    self.config.max_timeline_points,
                );
                return;
            }
        }

        match self
            .timeline
            .global_timeline
            .binary_search(&timestamp_nanos)
        {
            Ok(_) => {}
            Err(index) => {
                self.timeline.global_timeline.insert(index, timestamp_nanos);
                if let Some(current_index) = self.timeline.global_timeline_index {
                    if index <= current_index {
                        self.timeline.global_timeline_index = Some(current_index.saturating_add(1));
                    }
                }
            }
        }
        trim_timeline_to_capacity(
            &mut self.timeline.global_timeline,
            &mut self.timeline.global_timeline_index,
            self.config.max_timeline_points,
        );
    }

    pub(super) fn jump_latest_internal(&mut self, queue_scrub: bool) {
        if self.timeline.global_timeline.is_empty() {
            self.timeline.global_timeline_index = None;
            return;
        }

        self.timeline.timeline_viewport.manual_end_ns = None;
        let latest_index = self.timeline.global_timeline.len().saturating_sub(1);
        self.timeline.global_timeline_index = Some(latest_index);
        let latest_anchor = self.timeline.global_timeline[latest_index];
        if queue_scrub {
            self.timeline.pending_scrub_anchor = Some(latest_anchor);
        }
    }

    pub(super) fn set_global_timeline_anchor_by_timestamp(
        &mut self,
        timestamp_ns: u64,
        queue_scrub: bool,
    ) {
        if self.timeline.global_timeline.is_empty() {
            self.timeline.global_timeline_index = None;
            return;
        }

        let index = nearest_timestamp_index(&self.timeline.global_timeline, timestamp_ns)
            .unwrap_or_else(|| self.timeline.global_timeline.len().saturating_sub(1));
        self.set_global_timeline_index(index, queue_scrub);
    }

    pub(super) fn set_global_timeline_index(&mut self, index: usize, queue_scrub: bool) {
        if self.timeline.global_timeline.is_empty() {
            self.timeline.global_timeline_index = None;
            return;
        }
        let clamped = index.min(self.timeline.global_timeline.len().saturating_sub(1));
        self.timeline.global_timeline_index = Some(clamped);
        if queue_scrub {
            self.timeline.pending_scrub_anchor =
                self.timeline.global_timeline.get(clamped).copied();
        }
    }

    pub(super) fn mark_manual_timeline_navigation(&mut self) {
        if self.ui.follow_live {
            self.freeze_timeline_window_at_current_range();
        }
        self.ui.follow_live = false;
    }

    pub(super) fn freeze_timeline_window_at_current_range(&mut self) {
        let Some(current_range) = self.timeline_render_range() else {
            return;
        };
        if self.timeline.timeline_viewport.span.is_none() {
            self.timeline.timeline_viewport.span = Some(current_range.span());
        }
        self.timeline.timeline_viewport.manual_end_ns = Some(current_range.end_ns);
    }

    pub(super) fn set_timeline_hover_preview(&mut self, timestamp_ns: Option<u64>) {
        self.timeline.timeline_hover_preview = timestamp_ns;
    }

    pub(super) fn apply_timeline_zoom(&mut self, zoom_factor: f32, focus_timestamp_ns: u64) {
        let Some(full_range) = self.timeline_full_range() else {
            return;
        };
        let Some(current_range) = self.timeline_render_range() else {
            return;
        };

        let next_range = zoom_range_around_focus(
            full_range,
            current_range,
            focus_timestamp_ns,
            zoom_factor,
            MIN_TIMELINE_SPAN,
        );
        let lane_scroll_offset = self.timeline.timeline_viewport.lane_scroll_offset;
        let lane_height_px = self.timeline.timeline_viewport.lane_height_px;
        self.timeline.timeline_viewport = viewport_state_from_range(next_range, full_range);
        self.timeline.timeline_viewport.lane_scroll_offset = lane_scroll_offset;
        self.timeline.timeline_viewport.lane_height_px = lane_height_px;
    }

    pub(super) fn apply_timeline_pan_fraction(&mut self, pan_delta_fraction: f32) {
        let Some(full_range) = self.timeline_full_range() else {
            return;
        };
        let Some(current_range) = self.timeline_render_range() else {
            return;
        };
        let span_ns = current_range.span_nanos();
        if span_ns == 0 {
            return;
        }

        let delta_ns = (pan_delta_fraction as f64 * span_ns as f64).round() as i128;
        let candidate_end = i128::from(current_range.end_ns).saturating_add(delta_ns);
        let candidate_start = candidate_end.saturating_sub(i128::from(span_ns));
        let clamped_range = clamp_timeline_range(full_range, candidate_start, span_ns);
        let lane_scroll_offset = self.timeline.timeline_viewport.lane_scroll_offset;
        let lane_height_px = self.timeline.timeline_viewport.lane_height_px;
        self.timeline.timeline_viewport = viewport_state_from_range(clamped_range, full_range);
        self.timeline.timeline_viewport.lane_scroll_offset = lane_scroll_offset;
        self.timeline.timeline_viewport.lane_height_px = lane_height_px;
    }

    pub(super) fn apply_timeline_lane_scroll(
        &mut self,
        lane_delta: f32,
        total_lanes: usize,
        canvas_height_px: f32,
    ) {
        self.timeline.timeline_viewport.lane_scroll_offset = next_lane_scroll_offset(
            self.timeline.timeline_viewport.lane_scroll_offset,
            lane_delta,
            total_lanes,
            self.timeline.timeline_viewport.lane_height_px,
            canvas_height_px,
        );
    }

    fn lane_label_for_key(&self, key: &TimelineLaneKey) -> String {
        let default_namespace = self.ui.default_namespace.trim();
        if default_namespace.is_empty() || default_namespace == key.namespace {
            key.path_expression.clone()
        } else {
            format!("{} @ {}", key.path_expression, key.namespace)
        }
    }

    pub(super) fn bind_stream_lane(
        &mut self,
        stream_id: crate::model::StreamId,
        binding: crate::model::SourceBindingInfo,
    ) {
        let key = TimelineLaneKey {
            namespace: binding.namespace,
            path_expression: binding.path_expression,
        };

        if let Some(previous_key) = self.timeline.stream_lane_bindings.get(&stream_id).cloned() {
            if previous_key != key {
                self.decrement_lane_binding(&previous_key);
            }
        }
        self.timeline
            .stream_lane_bindings
            .insert(stream_id, key.clone());

        let lane = self
            .timeline
            .timeline_lanes
            .entry(key.clone())
            .or_insert_with(|| super::state::TimelineLaneState {
                key,
                sample_timestamps: VecDeque::new(),
                last_seen_ns: 0,
                active_bindings: 0,
            });
        lane.active_bindings = lane.active_bindings.saturating_add(1);
        self.timeline.lane_order_dirty = true;
        self.evict_inactive_lanes_if_needed();
    }

    fn decrement_lane_binding(&mut self, lane_key: &TimelineLaneKey) {
        if decrement_lane_binding(&mut self.timeline.timeline_lanes, lane_key) {
            self.timeline.lane_order_dirty = true;
        }
    }

    pub(super) fn unbind_stream_lane(&mut self, stream_id: crate::model::StreamId) {
        if let Some(key) = self.timeline.stream_lane_bindings.remove(&stream_id) {
            self.decrement_lane_binding(&key);
        }
    }

    pub(super) fn append_lane_samples(
        &mut self,
        stream_id: crate::model::StreamId,
        records: &[DisplayedRecord],
    ) {
        let Some(lane_key) = self.timeline.stream_lane_bindings.get(&stream_id).cloned() else {
            return;
        };
        let lane = self
            .timeline
            .timeline_lanes
            .entry(lane_key.clone())
            .or_insert_with(|| super::state::TimelineLaneState {
                key: lane_key,
                sample_timestamps: VecDeque::new(),
                last_seen_ns: 0,
                active_bindings: 0,
            });
        if lane.sample_timestamps.is_empty() {
            self.timeline.lane_order_dirty = true;
        }

        for record in records {
            if lane.sample_timestamps.back().copied() != Some(record.timestamp_nanos) {
                lane.sample_timestamps.push_back(record.timestamp_nanos);
            }
            lane.last_seen_ns = lane.last_seen_ns.max(record.timestamp_nanos);
        }
        while lane.sample_timestamps.len() > self.config.max_samples_per_lane {
            let _ = lane.sample_timestamps.pop_front();
        }
        self.evict_inactive_lanes_if_needed();
    }

    pub(super) fn evict_inactive_lanes_if_needed(&mut self) {
        if evict_inactive_lanes_if_needed(
            &mut self.timeline.timeline_lanes,
            self.config.max_retained_lanes,
        ) {
            self.timeline.lane_order_dirty = true;
        }
    }

    pub(super) fn timeline_lane_rows(
        &mut self,
        viewport_range: TimelineRenderRange,
        lane_window_start: usize,
        lane_window_count: usize,
        pixel_width: f32,
    ) -> (Vec<LaneRenderRow>, usize) {
        if self.timeline.lane_order_dirty {
            let mut ordered = self
                .timeline
                .timeline_lanes
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            ordered.sort_by(|left, right| {
                left.path_expression
                    .cmp(&right.path_expression)
                    .then_with(|| left.namespace.cmp(&right.namespace))
            });
            self.timeline.lane_order_cache = ordered;
            self.timeline.lane_order_dirty = false;
        }

        let total = self.timeline.lane_order_cache.len();
        if total == 0 || lane_window_count == 0 {
            return (Vec::new(), total);
        }
        let start = lane_window_start.min(total.saturating_sub(1));
        let end = (start + lane_window_count).min(total);
        let slot_count = ((pixel_width.max(80.0) / 8.0).round() as usize).clamp(32, 1024);

        let mut rows = Vec::with_capacity(end.saturating_sub(start));
        for key in &self.timeline.lane_order_cache[start..end] {
            let Some(lane) = self.timeline.timeline_lanes.get(key) else {
                continue;
            };
            let clustered = cluster_lane_samples(
                &lane.sample_timestamps,
                viewport_range.start_ns,
                viewport_range.end_ns,
                slot_count,
            );
            let points = merge_lane_points_by_pixel_distance(
                clustered,
                viewport_range,
                pixel_width.max(64.0),
                5.0,
            );
            rows.push(LaneRenderRow {
                key: lane.key.clone(),
                label: self.lane_label_for_key(&lane.key),
                points,
                color_index: lane_color_index(&lane.key),
                active_bindings: lane.active_bindings,
            });
        }

        (rows, total)
    }

    pub(super) fn maybe_emit_scrub_command(&mut self) {
        let Some(anchor_nanos) = self.timeline.pending_scrub_anchor else {
            return;
        };

        let now = Instant::now();
        if should_emit_scrub_command(
            self.timeline.last_scrub_emitted,
            now,
            self.config.scrub_debounce,
        ) {
            for stream_id in self
                .workspace
                .stream_states
                .keys()
                .copied()
                .collect::<Vec<_>>()
            {
                self.send_command(WorkerCommand::SetScrubAnchor {
                    stream_id,
                    anchor_nanos,
                });
            }
            self.timeline.last_scrub_emitted = now;
            self.timeline.pending_scrub_anchor = None;
        }
    }
}

fn nearest_timestamp_index(timeline: &[u64], target_ns: u64) -> Option<usize> {
    if timeline.is_empty() {
        return None;
    }
    Some(match timeline.binary_search(&target_ns) {
        Ok(index) => index,
        Err(0) => 0,
        Err(index) if index >= timeline.len() => timeline.len().saturating_sub(1),
        Err(index) => {
            let prev = index.saturating_sub(1);
            let prev_delta = target_ns.saturating_sub(timeline[prev]);
            let next_delta = timeline[index].saturating_sub(target_ns);
            if prev_delta <= next_delta {
                prev
            } else {
                index
            }
        }
    })
}

fn derive_timeline_render_range(
    full_range: TimelineRenderRange,
    viewport: TimelineViewportState,
    follow_live: bool,
) -> TimelineRenderRange {
    let full_span = full_range.span();
    let Some(span) = viewport
        .span
        .map(|span| clamp_timeline_span(span, full_span))
    else {
        return full_range;
    };
    let span_ns = duration_to_nanos(span);

    if follow_live {
        let start_ns = full_range.end_ns.saturating_sub(span_ns);
        return TimelineRenderRange {
            start_ns,
            end_ns: full_range.end_ns,
        };
    }

    let manual_end_ns = viewport.manual_end_ns.unwrap_or(full_range.end_ns);
    let manual_end_i128 = i128::from(manual_end_ns);
    let span_i128 = i128::from(span_ns);
    let candidate_start = manual_end_i128 - span_i128;
    clamp_timeline_range(full_range, candidate_start, span_ns)
}

fn clamp_timeline_span(requested_span: Duration, full_span: Duration) -> Duration {
    if full_span.is_zero() {
        return Duration::ZERO;
    }
    requested_span
        .max(MIN_TIMELINE_SPAN.min(full_span))
        .min(full_span)
}

fn clamp_timeline_range(
    full_range: TimelineRenderRange,
    start_candidate_ns: i128,
    span_ns: u64,
) -> TimelineRenderRange {
    let full_span_ns = full_range.span_nanos();
    if span_ns >= full_span_ns {
        return full_range;
    }

    let min_start = i128::from(full_range.start_ns);
    let max_start = i128::from(full_range.end_ns.saturating_sub(span_ns));
    let start_ns = start_candidate_ns.clamp(min_start, max_start);
    let start_ns = u64::try_from(start_ns).unwrap_or(full_range.start_ns);
    TimelineRenderRange {
        start_ns,
        end_ns: start_ns.saturating_add(span_ns),
    }
}

fn zoom_range_around_focus(
    full_range: TimelineRenderRange,
    current_range: TimelineRenderRange,
    focus_ns: u64,
    zoom_factor: f32,
    min_span: Duration,
) -> TimelineRenderRange {
    let current_span_ns = current_range.span_nanos();
    let full_span_ns = full_range.span_nanos();
    if current_span_ns == 0 || full_span_ns == 0 {
        return full_range;
    }

    let clamped_factor = zoom_factor.clamp(0.1, 10.0) as f64;
    let desired_span = ((current_span_ns as f64) * clamped_factor).round() as u64;
    let min_span_ns = duration_to_nanos(min_span);
    let target_span = desired_span
        .max(min_span_ns.min(full_span_ns))
        .min(full_span_ns);
    if target_span == full_span_ns {
        return full_range;
    }

    let focus_ns = focus_ns.clamp(current_range.start_ns, current_range.end_ns);
    let relative = (focus_ns.saturating_sub(current_range.start_ns) as f64
        / current_span_ns.max(1) as f64)
        .clamp(0.0, 1.0);
    let start_candidate = i128::from(focus_ns) - ((target_span as f64 * relative).round() as i128);
    clamp_timeline_range(full_range, start_candidate, target_span)
}

fn viewport_state_from_range(
    range: TimelineRenderRange,
    full_range: TimelineRenderRange,
) -> TimelineViewportState {
    if range == full_range {
        return TimelineViewportState::default();
    }
    TimelineViewportState {
        span: Some(range.span()),
        manual_end_ns: Some(range.end_ns),
        lane_scroll_offset: 0.0,
        lane_height_px: DEFAULT_TIMELINE_LANE_HEIGHT_PX,
    }
}

fn timeline_visible_lane_count(lane_height_px: f32, canvas_height_px: f32) -> usize {
    timeline_lane_window_capacity(lane_height_px, canvas_height_px)
}

fn next_lane_scroll_offset(
    current_offset: f32,
    lane_delta: f32,
    total_lanes: usize,
    lane_height_px: f32,
    canvas_height_px: f32,
) -> f32 {
    let visible_lanes = timeline_visible_lane_count(lane_height_px, canvas_height_px);
    if total_lanes == 0 || total_lanes <= visible_lanes {
        return 0.0;
    }
    let lane_height = lane_height_px.max(12.0);
    let max_offset = total_lanes.saturating_sub(visible_lanes) as f32;
    let scaled_delta = lane_delta / lane_height;
    (current_offset + scaled_delta).clamp(0.0, max_offset)
}

fn decrement_lane_binding(
    lanes: &mut BTreeMap<TimelineLaneKey, super::state::TimelineLaneState>,
    lane_key: &TimelineLaneKey,
) -> bool {
    let mut should_remove = false;
    if let Some(lane) = lanes.get_mut(lane_key) {
        lane.active_bindings = lane.active_bindings.saturating_sub(1);
        should_remove = lane.active_bindings == 0 && lane.sample_timestamps.is_empty();
    }
    if should_remove {
        lanes.remove(lane_key);
        true
    } else {
        false
    }
}

fn evict_inactive_lanes_if_needed(
    lanes: &mut BTreeMap<TimelineLaneKey, super::state::TimelineLaneState>,
    max_retained_lanes: usize,
) -> bool {
    if lanes.len() <= max_retained_lanes {
        return false;
    }

    let mut candidates = lanes
        .iter()
        .filter(|(_, lane)| lane.active_bindings == 0)
        .map(|(key, lane)| (key.clone(), lane.last_seen_ns))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(_, last_seen_ns)| *last_seen_ns);

    let mut over = lanes.len().saturating_sub(max_retained_lanes);
    let mut removed_any = false;
    for (key, _) in candidates {
        if over == 0 {
            break;
        }
        lanes.remove(&key);
        removed_any = true;
        over = over.saturating_sub(1);
    }
    removed_any
}

fn lane_color_index(key: &TimelineLaneKey) -> usize {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish() as usize
}

fn cluster_lane_samples(
    samples: &VecDeque<u64>,
    start_ns: u64,
    end_ns: u64,
    slot_count: usize,
) -> Vec<LaneRenderPoint> {
    if samples.is_empty() || slot_count == 0 || start_ns > end_ns {
        return Vec::new();
    }

    let visible = collect_visible_lane_samples(samples, start_ns, end_ns);
    if visible.is_empty() {
        return Vec::new();
    }

    if start_ns == end_ns {
        return vec![LaneRenderPoint {
            timestamp_ns: start_ns,
            count: u32::try_from(visible.len()).unwrap_or(u32::MAX),
        }];
    }

    if visible.len() <= slot_count.saturating_mul(2) {
        return visible
            .into_iter()
            .map(|timestamp_ns| LaneRenderPoint {
                timestamp_ns,
                count: 1,
            })
            .collect();
    }

    let span_ns = end_ns.saturating_sub(start_ns).max(1);
    let mut first_per_slot = vec![None::<u64>; slot_count];
    let mut last_per_slot = vec![None::<u64>; slot_count];
    let mut count_per_slot = vec![0_u32; slot_count];

    for timestamp_ns in visible {
        let relative = timestamp_ns.saturating_sub(start_ns);
        let slot_index = ((relative as u128).saturating_mul(slot_count.saturating_sub(1) as u128)
            / span_ns as u128) as usize;
        let slot = slot_index.min(slot_count.saturating_sub(1));
        if first_per_slot[slot].is_none() {
            first_per_slot[slot] = Some(timestamp_ns);
        }
        last_per_slot[slot] = Some(timestamp_ns);
        count_per_slot[slot] = count_per_slot[slot].saturating_add(1);
    }

    let mut points = Vec::new();
    for slot in 0..slot_count {
        let Some(first) = first_per_slot[slot] else {
            continue;
        };
        let last = last_per_slot[slot].unwrap_or(first);
        let count = count_per_slot[slot].max(1);
        let center = first.saturating_add(last.saturating_sub(first) / 2);
        points.push(LaneRenderPoint {
            timestamp_ns: center,
            count,
        });
    }
    points
}

fn merge_lane_points_by_pixel_distance(
    points: Vec<LaneRenderPoint>,
    viewport_range: TimelineRenderRange,
    pixel_width: f32,
    min_distance_px: f32,
) -> Vec<LaneRenderPoint> {
    if points.len() <= 1 {
        return points;
    }
    let span_ns = viewport_range
        .end_ns
        .saturating_sub(viewport_range.start_ns)
        .max(1);
    let mut merged = Vec::new();
    let mut cluster = vec![points[0].clone()];

    for point in points.into_iter().skip(1) {
        let previous = cluster.last().expect("cluster has at least one entry");
        let delta_ns = point.timestamp_ns.abs_diff(previous.timestamp_ns);
        let distance_px = (delta_ns as f64 / span_ns as f64) * pixel_width as f64;
        if distance_px < min_distance_px as f64 {
            cluster.push(point);
            continue;
        }
        merged.push(select_cluster_representative(cluster.as_slice()));
        cluster.clear();
        cluster.push(point);
    }

    if !cluster.is_empty() {
        merged.push(select_cluster_representative(cluster.as_slice()));
    }
    merged
}

fn select_cluster_representative(cluster: &[LaneRenderPoint]) -> LaneRenderPoint {
    cluster
        .iter()
        .cloned()
        .max_by(|left, right| {
            left.count
                .cmp(&right.count)
                .then_with(|| right.timestamp_ns.cmp(&left.timestamp_ns))
        })
        .expect("merge clusters are always non-empty")
}

fn collect_visible_lane_samples(samples: &VecDeque<u64>, start_ns: u64, end_ns: u64) -> Vec<u64> {
    if samples.is_empty() {
        return Vec::new();
    }

    let (first, second) = samples.as_slices();
    let mut visible = Vec::new();
    push_visible_slice(first, start_ns, end_ns, &mut visible);
    push_visible_slice(second, start_ns, end_ns, &mut visible);
    visible
}

fn push_visible_slice(slice: &[u64], start_ns: u64, end_ns: u64, out: &mut Vec<u64>) {
    if slice.is_empty() {
        return;
    }
    let lower = slice.partition_point(|timestamp| *timestamp < start_ns);
    let upper = slice.partition_point(|timestamp| *timestamp <= end_ns);
    if lower < upper {
        out.extend_from_slice(&slice[lower..upper]);
    }
}

pub(crate) fn is_manual_timeline_navigation(
    selected_timestamp_ns: Option<u64>,
    pan_delta_fraction: Option<f32>,
    zoom_factor: Option<f32>,
) -> bool {
    selected_timestamp_ns.is_some() || pan_delta_fraction.is_some() || zoom_factor.is_some()
}

fn trim_timeline_to_capacity(
    timeline: &mut Vec<u64>,
    timeline_index: &mut Option<usize>,
    max_points: usize,
) {
    if timeline.len() <= max_points {
        return;
    }

    let trim_count = timeline.len().saturating_sub(max_points);
    timeline.drain(0..trim_count);
    *timeline_index = timeline_index.map(|index| index.saturating_sub(trim_count));
    if timeline.is_empty() {
        *timeline_index = None;
    }
}

fn duration_to_nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeMap, VecDeque},
        time::Duration,
    };

    use super::{
        clamp_timeline_span, cluster_lane_samples, decrement_lane_binding,
        derive_timeline_render_range, evict_inactive_lanes_if_needed,
        is_manual_timeline_navigation, merge_lane_points_by_pixel_distance,
        next_lane_scroll_offset, timeline_visible_lane_count, trim_timeline_to_capacity,
        zoom_range_around_focus, LaneRenderPoint, TimelineRenderRange, TimelineViewportState,
    };
    use crate::app::state::{TimelineLaneKey, TimelineLaneState};

    #[test]
    fn timeline_trim_keeps_most_recent_points() {
        let mut timeline = vec![1_u64, 2, 3, 4, 5];
        let mut index = Some(4_usize);

        trim_timeline_to_capacity(&mut timeline, &mut index, 3);

        assert_eq!(timeline, vec![3_u64, 4, 5]);
        assert_eq!(index, Some(2));
    }

    #[test]
    fn timeline_trim_clamps_index_to_start_when_anchor_evicted() {
        let mut timeline = vec![10_u64, 20, 30, 40];
        let mut index = Some(0_usize);

        trim_timeline_to_capacity(&mut timeline, &mut index, 2);

        assert_eq!(timeline, vec![30_u64, 40]);
        assert_eq!(index, Some(0));
    }

    #[test]
    fn timeline_range_follow_live_uses_latest_end() {
        let full = TimelineRenderRange {
            start_ns: 1_000_000_000,
            end_ns: 11_000_000_000,
        };
        let range = derive_timeline_render_range(
            full,
            TimelineViewportState {
                span: Some(Duration::from_secs(4)),
                manual_end_ns: Some(8_000_000_000),
                ..TimelineViewportState::default()
            },
            true,
        );

        assert_eq!(range.end_ns, 11_000_000_000);
        assert_eq!(range.start_ns, 7_000_000_000);
    }

    #[test]
    fn timeline_range_manual_window_is_clamped() {
        let full = TimelineRenderRange {
            start_ns: 1_000_000_000,
            end_ns: 11_000_000_000,
        };
        let range = derive_timeline_render_range(
            full,
            TimelineViewportState {
                span: Some(Duration::from_secs(4)),
                manual_end_ns: Some(500_000_000),
                ..TimelineViewportState::default()
            },
            false,
        );
        assert_eq!(
            range,
            TimelineRenderRange {
                start_ns: 1_000_000_000,
                end_ns: 5_000_000_000
            }
        );
    }

    #[test]
    fn timeline_range_manual_window_ignores_latest_growth() {
        let viewport = TimelineViewportState {
            span: Some(Duration::from_secs(4)),
            manual_end_ns: Some(8_000_000_000),
            ..TimelineViewportState::default()
        };
        let first = derive_timeline_render_range(
            TimelineRenderRange {
                start_ns: 1_000_000_000,
                end_ns: 11_000_000_000,
            },
            viewport,
            false,
        );
        let second = derive_timeline_render_range(
            TimelineRenderRange {
                start_ns: 1_000_000_000,
                end_ns: 15_000_000_000,
            },
            viewport,
            false,
        );
        assert_eq!(first.start_ns, 4_000_000_000);
        assert_eq!(first.end_ns, 8_000_000_000);
        assert_eq!(first, second);
    }

    #[test]
    fn timeline_range_manual_window_clamps_after_trim() {
        let viewport = TimelineViewportState {
            span: Some(Duration::from_secs(4)),
            manual_end_ns: Some(8_000_000_000),
            ..TimelineViewportState::default()
        };
        let clamped = derive_timeline_render_range(
            TimelineRenderRange {
                start_ns: 6_000_000_000,
                end_ns: 15_000_000_000,
            },
            viewport,
            false,
        );
        assert_eq!(
            clamped,
            TimelineRenderRange {
                start_ns: 6_000_000_000,
                end_ns: 10_000_000_000
            }
        );
    }

    #[test]
    fn clamp_span_enforces_minimum_bound() {
        let clamped = clamp_timeline_span(Duration::from_nanos(1_000), Duration::from_secs(1));
        assert!(clamped >= Duration::from_millis(50));
    }

    #[test]
    fn lane_sample_clustering_produces_monotonic_points() {
        let samples = (0..10_000_u64)
            .map(|index| index.saturating_mul(1_000))
            .collect::<VecDeque<_>>();
        let points = cluster_lane_samples(&samples, 100_000, 6_000_000, 200);
        assert!(!points.is_empty());
        assert!(points
            .windows(2)
            .all(|window| { window[0].timestamp_ns <= window[1].timestamp_ns }));
        assert!(points.iter().all(|point| point.count >= 1));
    }

    #[test]
    fn lane_window_capacity_stays_positive() {
        assert!(timeline_visible_lane_count(16.0, 236.0) >= 1);
        assert!(timeline_visible_lane_count(48.0, 236.0) >= 1);
    }

    #[test]
    fn lane_scroll_offset_clamps_at_bounds() {
        let low = next_lane_scroll_offset(0.0, -1_000.0, 50, 20.0, 236.0);
        assert_eq!(low, 0.0);

        let high = next_lane_scroll_offset(10.0, 10_000.0, 50, 20.0, 236.0);
        let max = (50usize.saturating_sub(timeline_visible_lane_count(20.0, 236.0))) as f32;
        assert_eq!(high, max);
    }

    #[test]
    fn lane_unbind_keeps_history_but_removes_empty_lane() {
        let history_key = TimelineLaneKey {
            namespace: "demo".to_string(),
            path_expression: "odometry".to_string(),
        };
        let empty_key = TimelineLaneKey {
            namespace: "demo".to_string(),
            path_expression: "imu".to_string(),
        };
        let mut lanes = BTreeMap::from([
            (
                history_key.clone(),
                TimelineLaneState {
                    key: history_key.clone(),
                    sample_timestamps: VecDeque::from([10_u64, 20, 30]),
                    last_seen_ns: 30,
                    active_bindings: 1,
                },
            ),
            (
                empty_key.clone(),
                TimelineLaneState {
                    key: empty_key.clone(),
                    sample_timestamps: VecDeque::new(),
                    last_seen_ns: 0,
                    active_bindings: 1,
                },
            ),
        ]);

        decrement_lane_binding(&mut lanes, &history_key);
        decrement_lane_binding(&mut lanes, &empty_key);

        assert!(lanes.contains_key(&history_key));
        assert!(!lanes.contains_key(&empty_key));
        assert_eq!(lanes[&history_key].active_bindings, 0);
    }

    #[test]
    fn lane_eviction_prefers_oldest_inactive_and_keeps_active() {
        let active_key = TimelineLaneKey {
            namespace: "demo".to_string(),
            path_expression: "active".to_string(),
        };
        let old_inactive = TimelineLaneKey {
            namespace: "demo".to_string(),
            path_expression: "old".to_string(),
        };
        let new_inactive = TimelineLaneKey {
            namespace: "demo".to_string(),
            path_expression: "new".to_string(),
        };
        let mut lanes = BTreeMap::from([
            (
                active_key.clone(),
                TimelineLaneState {
                    key: active_key.clone(),
                    sample_timestamps: VecDeque::from([99_u64]),
                    last_seen_ns: 99,
                    active_bindings: 1,
                },
            ),
            (
                old_inactive.clone(),
                TimelineLaneState {
                    key: old_inactive.clone(),
                    sample_timestamps: VecDeque::from([1_u64]),
                    last_seen_ns: 1,
                    active_bindings: 0,
                },
            ),
            (
                new_inactive.clone(),
                TimelineLaneState {
                    key: new_inactive.clone(),
                    sample_timestamps: VecDeque::from([50_u64]),
                    last_seen_ns: 50,
                    active_bindings: 0,
                },
            ),
        ]);

        evict_inactive_lanes_if_needed(&mut lanes, 2);

        assert!(lanes.contains_key(&active_key));
        assert!(!lanes.contains_key(&old_inactive));
        assert!(lanes.contains_key(&new_inactive));
    }

    #[test]
    fn merge_keeps_highest_density_representative() {
        let points = vec![
            LaneRenderPoint {
                timestamp_ns: 1_000,
                count: 1,
            },
            LaneRenderPoint {
                timestamp_ns: 1_002,
                count: 8,
            },
            LaneRenderPoint {
                timestamp_ns: 3_500,
                count: 2,
            },
        ];
        let merged = merge_lane_points_by_pixel_distance(
            points,
            TimelineRenderRange {
                start_ns: 0,
                end_ns: 10_000,
            },
            400.0,
            4.0,
        );
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].timestamp_ns, 1_002);
    }

    #[test]
    fn manual_navigation_flags_disable_follow_semantics() {
        assert!(is_manual_timeline_navigation(Some(100), None, None));
        assert!(is_manual_timeline_navigation(None, Some(0.2), None));
        assert!(is_manual_timeline_navigation(None, None, Some(0.8)));
        assert!(!is_manual_timeline_navigation(None, None, None));
    }

    #[test]
    fn zoom_range_keeps_focus_reasonably_stable() {
        let full = TimelineRenderRange {
            start_ns: 0,
            end_ns: 10_000,
        };
        let current = TimelineRenderRange {
            start_ns: 2_000,
            end_ns: 8_000,
        };
        let focus_ns = 5_000;
        let next = zoom_range_around_focus(full, current, focus_ns, 0.5, Duration::from_nanos(50));
        let prev_relative = (focus_ns - current.start_ns) as f64
            / (current.end_ns - current.start_ns).max(1) as f64;
        let next_relative =
            (focus_ns - next.start_ns) as f64 / (next.end_ns - next.start_ns).max(1) as f64;
        assert!((prev_relative - next_relative).abs() < 0.2);
    }
}
