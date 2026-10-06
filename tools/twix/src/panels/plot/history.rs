use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ros_z::{
    dynamic::{DynamicPayload, DynamicValue, SelectedValue, ValuePath},
    time::Time,
};
use ros_z_debug::{
    CachedSubscriptionStatus, DynamicTopicObservation, ObservationPolicy, RetentionPolicy,
    SampleRecord, TopicObservationStatus, TopicObservationUpdateReceiver,
};

use crate::repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates};

use super::status::format_topic_observation_status;

type Records = Arc<[Arc<SampleRecord<DynamicPayload>>]>;

/// One time-window observation per distinct topic in this panel. Field selections
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

    /// Newest sample of the displayed snapshot, which stays fixed while paused.
    pub fn latest(&self, topic: &str) -> Option<Arc<SampleRecord<DynamicPayload>>> {
        self.topics
            .get(topic)?
            .as_ref()
            .ok()?
            .records
            .last()
            .cloned()
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
            .policy(
                ObservationPolicy::time_window(duration)?
                    .with_retention(RetentionPolicy::time_window_without_sample_limit(duration)?),
            )
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

/// Projection of one retained snapshot into numeric segments or
/// state intervals, depending on the selected field's type. Missing and
/// non-finite values separate both, so the renderer cannot draw across
/// unavailable data.
#[derive(Default)]
pub(super) struct SeriesData {
    records: Option<Records>,
    path: String,
    pub segments: Vec<Vec<(Time, f64)>>,
    pub states: Vec<StateInterval>,
    pub issue: Option<String>,
    pub gaps: usize,
}

/// A run of one enum variant or boolean value. The current state has no end
/// and extends to the newest displayed time.
#[derive(Debug, PartialEq)]
pub(super) struct StateInterval {
    pub start: Time,
    pub end: Option<Time>,
    pub index: u32,
    pub name: String,
}

enum SampleValue<'a> {
    Number(f64),
    State { index: u32, name: &'a str },
}

impl SeriesData {
    /// True when the selected field is a boolean or enum.
    pub fn is_state(&self) -> bool {
        !self.states.is_empty()
    }

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
        self.states.clear();
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
        let mut state: Option<StateInterval> = None;
        for (time, payload) in samples {
            let value = path
                .select(payload)
                .map_err(|error| error.to_string())
                .and_then(sample_value);
            match value {
                Ok(SampleValue::Number(value)) => {
                    self.issue = None;
                    segment.push((time, value));
                }
                Ok(SampleValue::State { index, name }) => {
                    self.issue = None;
                    // Variant indices identify states within one schema, so
                    // repeated samples extend the run without allocating.
                    if state.as_ref().is_some_and(|state| state.index == index) {
                        continue;
                    }
                    self.close_state(&mut state, time);
                    state = Some(StateInterval {
                        start: time,
                        end: None,
                        index,
                        name: name.to_owned(),
                    });
                }
                Err(error) => {
                    self.issue = Some(error);
                    self.gaps += 1;
                    if !segment.is_empty() {
                        self.segments.push(std::mem::take(&mut segment));
                    }
                    self.close_state(&mut state, time);
                }
            }
        }
        if !segment.is_empty() {
            self.segments.push(segment);
        }
        self.states.extend(state);
    }

    fn close_state(&mut self, state: &mut Option<StateInterval>, end: Time) {
        if let Some(mut state) = state.take() {
            state.end = Some(end);
            self.states.push(state);
        }
    }
}

fn sample_value(selected: SelectedValue<'_>) -> Result<SampleValue<'_>, String> {
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
            DynamicValue::Bool(value) => {
                return Ok(SampleValue::State {
                    index: u32::from(*value),
                    name: if *value { "true" } else { "false" },
                });
            }
            DynamicValue::Enum(value) => {
                return Ok(SampleValue::State {
                    index: value.variant_index,
                    name: &value.variant_name,
                });
            }
            DynamicValue::Optional(Some(value)) => {
                return sample_value(SelectedValue::Value(value));
            }
            DynamicValue::Optional(None) => {
                return Err("Value unavailable: optional is absent.".into());
            }
            _ => return Err(UNSUPPORTED_SELECTION.into()),
        },
        SelectedValue::EnumPayload(_) => return Err(UNSUPPORTED_SELECTION.into()),
    };
    finite(value, "Value").map(SampleValue::Number)
}

const UNSUPPORTED_SELECTION: &str = "Select a numeric, boolean, or enum field to plot.";

fn finite(value: f64, subject: &str) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{subject} is NaN or infinite."))
    }
}

pub(super) fn seconds_from(time: Time, origin: Time) -> f64 {
    // Subtract before converting to preserve sub-millisecond resolution at
    // wallclock-sized timestamps.
    (time.as_nanos() - origin.as_nanos()) as f64 / 1e9
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ros_z::{
        dynamic::{
            DynamicPayload, DynamicValue, EnumDef, EnumPayloadDef, EnumPayloadValue, EnumValue,
            EnumVariantDef, PrimitiveTypeDef, SchemaBundle, TypeDef, TypeDefinition,
            TypeDefinitions, TypeName, ValuePath,
        },
        time::Time,
    };

    use super::{SeriesData, StateInterval};

    fn project(
        root: TypeDef,
        definitions: TypeDefinitions,
        values: Vec<DynamicValue>,
    ) -> SeriesData {
        let schema = Arc::new(SchemaBundle { root, definitions });
        let samples: Vec<_> = values
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                let payload = DynamicPayload::new(schema.clone(), value).unwrap();
                (Time::from_nanos(index as i64), payload)
            })
            .collect();
        let mut series = SeriesData::default();
        series.project_samples(
            &ValuePath::default(),
            samples.iter().map(|(time, payload)| (*time, payload)),
        );
        series
    }

    fn state(start: i64, end: Option<i64>, index: u32, name: &str) -> StateInterval {
        StateInterval {
            start: Time::from_nanos(start),
            end: end.map(Time::from_nanos),
            index,
            name: name.to_owned(),
        }
    }

    #[test]
    fn invalid_numbers_break_the_line() {
        let series = project(
            TypeDef::Primitive(PrimitiveTypeDef::F64),
            Default::default(),
            [1.0, f64::NAN, 2.0]
                .into_iter()
                .map(DynamicValue::Float64)
                .collect(),
        );

        assert_eq!(
            series.segments,
            [
                vec![(Time::from_nanos(0), 1.0)],
                vec![(Time::from_nanos(2), 2.0)]
            ]
        );
        assert_eq!(series.gaps, 1);
        assert!(!series.is_state());
    }

    #[test]
    fn enum_samples_merge_into_state_runs() {
        let name = TypeName::new("test::Motion").unwrap();
        let definitions = [(
            name.clone(),
            TypeDefinition::Enum(EnumDef {
                variants: vec![
                    EnumVariantDef::new("Stand", EnumPayloadDef::Unit),
                    EnumVariantDef::new("Walk", EnumPayloadDef::Unit),
                ],
            }),
        )]
        .into();
        let variant = |index: u32, name: &str| {
            DynamicValue::Enum(EnumValue::new(index, name, EnumPayloadValue::Unit))
        };

        let series = project(
            TypeDef::Named(name),
            definitions,
            vec![
                variant(0, "Stand"),
                variant(0, "Stand"),
                variant(1, "Walk"),
                variant(0, "Stand"),
            ],
        );

        assert_eq!(
            series.states,
            [
                state(0, Some(2), 0, "Stand"),
                state(2, Some(3), 1, "Walk"),
                state(3, None, 0, "Stand"),
            ]
        );
        assert!(series.segments.is_empty());
        assert!(series.is_state());
    }

    #[test]
    fn unavailable_states_end_the_current_run() {
        let optional_bool = |value: Option<bool>| {
            DynamicValue::Optional(value.map(|value| Box::new(DynamicValue::Bool(value))))
        };

        let series = project(
            TypeDef::Optional(Box::new(TypeDef::Primitive(PrimitiveTypeDef::Bool))),
            Default::default(),
            vec![
                optional_bool(Some(true)),
                optional_bool(None),
                optional_bool(Some(true)),
                optional_bool(Some(false)),
            ],
        );

        assert_eq!(
            series.states,
            [
                state(0, Some(1), 1, "true"),
                state(2, Some(3), 1, "true"),
                state(3, None, 0, "false"),
            ]
        );
        assert_eq!(series.gaps, 1);
    }
}
