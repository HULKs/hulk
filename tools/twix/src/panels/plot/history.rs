use std::{collections::BTreeMap, sync::Arc, time::Duration};

#[cfg(test)]
mod tests;

use ros_z::{
    dynamic::{DynamicPayload, DynamicValue, SelectedValue, ValuePath},
    time::Time,
};
use ros_z_debug::{
    CachedSubscriptionStatus, DynamicTopicObservation, ObservationPolicy, SampleRecord,
    TopicObservationStatus, TopicObservationUpdateReceiver,
};

use crate::{
    repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates},
    status::format_topic_observation_status,
};

type Records = Arc<[Arc<SampleRecord<DynamicPayload>>]>;

/// One bounded observation per distinct topic in this panel. Field selections
/// share its decoded history, including samples received between UI frames.
#[derive(Default)]
pub(super) struct PlotHistory {
    topics: BTreeMap<String, Result<TopicHistory, String>>,
    namespace: String,
    duration: Duration,
}

struct TopicHistory {
    observation: DynamicTopicObservation,
    _repaint: ObservationRepaint,
    updates: TopicObservationUpdateReceiver,
    records: Records,
    dirty: bool,
}

impl PlotHistory {
    /// Reconcile subscriptions with the configured sources. Returns true when
    /// switching namespace or retention has discarded the displayed history.
    pub fn reconcile<'a>(
        &mut self,
        context: &impl ObservationContext,
        topics: impl Iterator<Item = &'a str>,
        duration: Duration,
    ) -> bool {
        let namespace = context.backend().namespace();
        let reset = self.namespace != namespace || self.duration != duration;
        if reset {
            self.topics.clear();
            self.namespace = namespace;
            self.duration = duration;
        }
        let topics: std::collections::BTreeSet<_> =
            topics.filter(|topic| !topic.is_empty()).collect();
        self.topics
            .retain(|topic, _| topics.contains(topic.as_str()));
        for topic in topics {
            self.topics.entry(topic.to_owned()).or_insert_with(|| {
                TopicHistory::new(context, topic, duration).map_err(|error| error.to_string())
            });
        }
        reset
    }

    pub fn refresh(&mut self) {
        for history in self
            .topics
            .values_mut()
            .filter_map(|entry| entry.as_mut().ok())
        {
            while let Ok(Some(_)) = history.updates.try_recv() {
                history.dirty = true;
            }
            if history.dirty {
                history.records = history
                    .observation
                    .window(Time::zero(), Time::from_nanos(i64::MAX))
                    .into();
                history.dirty = false;
            }
        }
    }

    pub fn latest(&self, topic: &str) -> Option<Arc<SampleRecord<DynamicPayload>>> {
        self.topics.get(topic)?.as_ref().ok()?.observation.latest()
    }

    pub fn end_time(&self) -> Option<Time> {
        self.topics
            .values()
            .filter_map(|entry| {
                entry
                    .as_ref()
                    .ok()?
                    .records
                    .last()
                    .map(|sample| sample.source_time)
            })
            .max()
    }

    pub fn status(&self, topic: &str) -> String {
        match self.topics.get(topic) {
            Some(Ok(history)) => match history.observation.status() {
                TopicObservationStatus::Observing { cache }
                    if cache.status() == &CachedSubscriptionStatus::Ready =>
                {
                    format!("{} samples", history.records.len())
                }
                status => format_topic_observation_status(status),
            },
            Some(Err(error)) => error.clone(),
            None => "Enter a topic and select a numeric field.".into(),
        }
    }

    pub fn project(&self, topic: &str, path: &str, series: &mut SeriesData) {
        let records = self.topics.get(topic).and_then(|entry| entry.as_ref().ok());
        series.refresh(records.map(|history| &history.records), path);
    }
}

impl TopicHistory {
    fn new(
        context: &impl ObservationContext,
        topic: &str,
        duration: Duration,
    ) -> ros_z_debug::Result<Self> {
        let _runtime = context.backend().runtime_handle().enter();
        let observation = context
            .backend()
            .observer()
            .observe_dynamic(topic)?
            .policy(ObservationPolicy::time_window(duration)?)
            .spawn();
        let repaint = observation.repaint_on_updates(context);
        let updates = observation
            .subscribe_updates()
            .expect("new observation is open");
        Ok(Self {
            observation,
            _repaint: repaint,
            updates,
            records: Arc::from([]),
            dirty: true,
        })
    }
}

/// Numeric projection of one retained snapshot. Missing and non-finite values
/// separate segments so the renderer cannot draw across unavailable data.
#[derive(Default)]
pub(super) struct SeriesData {
    records: Option<Records>,
    path: String,
    pub segments: Vec<Vec<(Time, f64)>>,
    pub issue: Option<String>,
    pub gaps: usize,
}

impl SeriesData {
    fn refresh(&mut self, records: Option<&Records>, path: &str) {
        if self.path == path
            && match (&self.records, records) {
                (Some(old), Some(new)) => Arc::ptr_eq(old, new),
                (None, None) => true,
                _ => false,
            }
        {
            return;
        }
        self.records = records.cloned();
        self.path = path.to_owned();
        self.segments.clear();
        self.issue = None;
        self.gaps = 0;
        let path = match path.parse::<ValuePath>() {
            Ok(path) => path,
            Err(error) => {
                self.issue = Some(error.to_string());
                return;
            }
        };
        if let Some(records) = records {
            self.project_samples(
                &path,
                records
                    .iter()
                    .map(|record| (record.source_time, &record.value)),
            );
        }
    }

    fn project_samples<'a>(
        &mut self,
        path: &ValuePath,
        samples: impl Iterator<Item = (Time, &'a DynamicPayload)>,
    ) {
        let mut segment = Vec::new();
        for (time, payload) in samples {
            let value = path
                .select(payload)
                .map_err(|error| error.to_string())
                .and_then(numeric_value);
            self.issue = value.as_ref().err().cloned();
            match value {
                Ok(value) => segment.push((time, value)),
                Err(_) => {
                    self.gaps += 1;
                    if !segment.is_empty() {
                        self.segments.push(std::mem::take(&mut segment));
                    }
                }
            }
        }
        if !segment.is_empty() {
            self.segments.push(segment);
        }
    }
}

fn numeric_value(selected: SelectedValue<'_>) -> Result<f64, String> {
    let value = match selected {
        SelectedValue::Byte(value) => f64::from(value),
        SelectedValue::Value(value) => match value {
            DynamicValue::Int8(value) => f64::from(*value),
            DynamicValue::Int16(value) => f64::from(*value),
            DynamicValue::Int32(value) => f64::from(*value),
            DynamicValue::Int64(value) => *value as f64,
            DynamicValue::Uint8(value) => f64::from(*value),
            DynamicValue::Uint16(value) => f64::from(*value),
            DynamicValue::Uint32(value) => f64::from(*value),
            DynamicValue::Uint64(value) => *value as f64,
            DynamicValue::Float32(value) => f64::from(*value),
            DynamicValue::Float64(value) => *value,
            DynamicValue::Optional(Some(value)) => {
                return numeric_value(SelectedValue::Value(value));
            }
            DynamicValue::Optional(None) => {
                return Err("Value unavailable: optional is absent.".into());
            }
            _ => return Err("Select a numeric field to plot.".into()),
        },
        SelectedValue::EnumPayload(_) => return Err("Select a numeric field to plot.".into()),
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err("Value is NaN or infinite.".into())
    }
}

pub(super) fn seconds_from(time: Time, origin: Time) -> f64 {
    // Subtract before converting to preserve sub-millisecond resolution at
    // wallclock-sized timestamps.
    (time.as_nanos() - origin.as_nanos()) as f64 / 1e9
}
