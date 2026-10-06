use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use rhai::AST;
use ros_z::{
    dynamic::{DynamicPayload, DynamicValue, SelectedValue, SelectionError, ValuePath},
    time::Time,
};
use ros_z_debug::{
    CachedSubscriptionStatus, DynamicTopicObservation, ObservationPolicy, RetentionPolicy,
    SampleRecord, TopicObservationStatus, TopicObservationUpdateReceiver,
};

use crate::repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates};

use super::{
    conversion::{Conversion, Output, json_to_dynamic, run},
    status::format_topic_observation_status,
};

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

    /// Project a topic's history, converting samples until `deadline`.
    /// Returns true when samples remain for a later frame.
    pub fn project(
        &self,
        topic: &str,
        path: &str,
        conversion: &Conversion,
        series: &mut SeriesData,
        deadline: Instant,
    ) -> bool {
        let records = self.topics.get(topic).and_then(|entry| entry.as_ref().ok());
        series.refresh(
            records.map(|history| &history.records),
            path,
            conversion,
            deadline,
        )
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

/// A retained sample. Snapshots of one history share sample allocations, so
/// a projection reuses the results of samples it has already converted.
pub(super) trait Sample {
    fn time(&self) -> Time;
    fn payload(&self) -> &DynamicPayload;
}

impl Sample for SampleRecord<DynamicPayload> {
    fn time(&self) -> Time {
        self.source_time
    }

    fn payload(&self) -> &DynamicPayload {
        &self.value
    }
}

/// Projection of retained samples into numeric segments or state intervals,
/// depending on the type of the (converted) value. Missing and non-finite
/// values separate both, so the renderer cannot draw across unavailable data.
pub(super) struct SeriesData<S = SampleRecord<DynamicPayload>> {
    records: Option<Arc<[Arc<S>]>>,
    path: String,
    selection: Result<ValuePath, String>,
    script: Option<Rc<AST>>,
    /// Converted samples, covering a prefix of `records` while `pending`.
    projected: VecDeque<(Arc<S>, Projected)>,
    state_names: HashMap<u32, String>,
    pending: bool,
    pub segments: Vec<Vec<(Time, f64)>>,
    pub states: Vec<StateInterval>,
    pub issue: Option<Arc<str>>,
    pub gaps: usize,
    /// Whether the last projection with data had states, kept while no data
    /// is available, so the item's kind does not flicker.
    shows_states: bool,
}

impl<S> Default for SeriesData<S> {
    fn default() -> Self {
        Self {
            records: None,
            path: String::new(),
            selection: parse_path(""),
            script: None,
            projected: VecDeque::new(),
            state_names: HashMap::new(),
            pending: false,
            segments: Vec::new(),
            states: Vec::new(),
            issue: None,
            gaps: 0,
            shows_states: false,
        }
    }
}

/// A run of one enum variant, boolean, or converted string. The current state
/// has no end and extends to the newest displayed time.
#[derive(Debug, PartialEq)]
pub(super) struct StateInterval {
    pub start: Time,
    pub end: Option<Time>,
    pub index: u32,
    pub name: String,
}

/// One converted sample. States refer to their names by index, so repeated
/// samples do not allocate.
enum Projected {
    Number(f64),
    State(u32),
    Skipped,
    Error(Arc<str>),
}

enum SampleValue<'a> {
    Number(f64),
    State { index: u32, name: &'a str },
}

impl<S: Sample> SeriesData<S> {
    /// True when the item shows states. Without any valid samples, the
    /// previous kind is kept.
    pub fn is_state(&self) -> bool {
        self.shows_states
    }

    /// Convert samples that are not yet projected until `deadline`, then
    /// rebuild the displayed data. Returns true when samples remain for a
    /// later frame.
    fn refresh(
        &mut self,
        records: Option<&Arc<[Arc<S>]>>,
        path: &str,
        conversion: &Conversion,
        deadline: Instant,
    ) -> bool {
        let script = conversion.script();
        let same_source = self.path == path
            && match (&self.script, script) {
                (Some(old), Some(new)) => Rc::ptr_eq(old, new),
                (None, None) => true,
                _ => false,
            };
        let same_records = match (&self.records, records) {
            (Some(old), Some(new)) => Arc::ptr_eq(old, new),
            (None, None) => true,
            _ => false,
        };
        if same_source && same_records && !self.pending {
            return false;
        }
        if !same_source {
            self.path = path.to_owned();
            self.selection = parse_path(path);
            self.script = script.cloned();
            self.projected.clear();
            self.state_names.clear();
        }
        self.records = records.cloned();
        let records: &[Arc<S>] = records.map_or(&[], |records| records);
        align(&mut self.projected, records);
        self.pending = false;
        if let Ok(selection) = &self.selection {
            let mut last_error = match self.projected.back() {
                Some((_, Projected::Error(error))) => Some(error.clone()),
                _ => None,
            };
            for record in &records[self.projected.len()..] {
                if Instant::now() >= deadline {
                    self.pending = true;
                    break;
                }
                let projected = project(
                    record.payload(),
                    selection,
                    self.script.as_deref(),
                    &mut self.state_names,
                    &mut last_error,
                );
                self.projected.push_back((record.clone(), projected));
            }
        }
        self.rebuild();
        self.pending
    }

    fn rebuild(&mut self) {
        self.segments.clear();
        self.states.clear();
        self.gaps = 0;
        self.issue = self
            .selection
            .as_ref()
            .err()
            .map(|error| error.as_str().into());
        let mut segment = Vec::new();
        let mut state: Option<StateInterval> = None;
        for (record, projected) in &self.projected {
            let time = record.time();
            if !matches!(projected, Projected::Number(_)) && !segment.is_empty() {
                self.segments.push(std::mem::take(&mut segment));
            }
            let continues_state = matches!(
                (projected, &state),
                (Projected::State(index), Some(state)) if state.index == *index
            );
            if !continues_state && let Some(mut state) = state.take() {
                state.end = Some(time);
                self.states.push(state);
            }
            match projected {
                Projected::Number(value) => {
                    self.issue = None;
                    segment.push((time, *value));
                }
                Projected::State(index) => {
                    self.issue = None;
                    state.get_or_insert_with(|| StateInterval {
                        start: time,
                        end: None,
                        index: *index,
                        name: self.state_names.get(index).cloned().unwrap_or_default(),
                    });
                }
                Projected::Skipped => self.issue = None,
                Projected::Error(error) => {
                    self.issue = Some(error.clone());
                    self.gaps += 1;
                }
            }
        }
        if !segment.is_empty() {
            self.segments.push(segment);
        }
        self.states.extend(state);
        if !self.segments.is_empty() || !self.states.is_empty() {
            self.shows_states = !self.states.is_empty();
        }
    }
}

fn parse_path(path: &str) -> Result<ValuePath, String> {
    path.parse()
        .map_err(|error: SelectionError| error.to_string())
}

/// Drop projected samples that left the history and keep the rest when they
/// are still in the same order. Otherwise, the history starts over.
fn align<S>(projected: &mut VecDeque<(Arc<S>, Projected)>, records: &[Arc<S>]) {
    let start = records.first().and_then(|first| {
        projected
            .iter()
            .position(|(record, _)| Arc::ptr_eq(record, first))
    });
    match start {
        Some(start) => {
            projected.drain(..start);
        }
        None => projected.clear(),
    }
    let in_order = projected.len() <= records.len()
        && projected
            .iter()
            .zip(records)
            .all(|((projected, _), record)| Arc::ptr_eq(projected, record));
    if !in_order {
        projected.clear();
    }
}

fn project(
    payload: &DynamicPayload,
    selection: &ValuePath,
    script: Option<&AST>,
    state_names: &mut HashMap<u32, String>,
    last_error: &mut Option<Arc<str>>,
) -> Projected {
    let mut state = |index: u32, name: &str| {
        state_names.entry(index).or_insert_with(|| name.to_owned());
        Projected::State(index)
    };
    let result = selection
        .select(payload)
        .map_err(|error| error.to_string())
        .and_then(|selected| match script {
            None => sample_value(selected).map(|value| match value {
                SampleValue::Number(value) => Projected::Number(value),
                SampleValue::State { index, name } => state(index, name),
            }),
            Some(script) => run(
                script,
                json_to_dynamic(&selected.to_json(Default::default())),
            )
            .and_then(|output| match output {
                Output::Number(value) => finite(value, "Converted value").map(Projected::Number),
                Output::Bool(value) => Ok(state(u32::from(value), bool_name(value))),
                Output::Text(text) => Ok(state(text_index(&text), &text)),
                Output::Nothing => Ok(Projected::Skipped),
            }),
        });
    result.unwrap_or_else(|message| {
        // Errors usually repeat for many samples, so share their text.
        let error = match last_error {
            Some(last) if **last == *message => last.clone(),
            _ => Arc::from(message),
        };
        *last_error = Some(error.clone());
        Projected::Error(error)
    })
}

fn bool_name(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

/// Stable state index of a converted string, so its color does not change
/// between sessions. This is the 32-bit FNV-1a hash.
fn text_index(text: &str) -> u32 {
    text.bytes().fold(0x811c_9dc5, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    })
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
                    name: bool_name(*value),
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

const UNSUPPORTED_SELECTION: &str =
    "Select a numeric, boolean, or enum field, or convert the value with a script.";

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
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    use ros_z::{
        dynamic::{
            DynamicPayload, DynamicValue, EnumDef, EnumPayloadDef, EnumPayloadValue, EnumValue,
            EnumVariantDef, PrimitiveTypeDef, SchemaBundle, TypeDef, TypeDefinition,
            TypeDefinitions, TypeName,
        },
        time::Time,
    };

    use super::{Conversion, Sample, SeriesData, StateInterval, text_index};

    struct TestSample {
        time: Time,
        payload: DynamicPayload,
    }

    impl Sample for TestSample {
        fn time(&self) -> Time {
            self.time
        }

        fn payload(&self) -> &DynamicPayload {
            &self.payload
        }
    }

    type Records = Arc<[Arc<TestSample>]>;

    fn later() -> Instant {
        Instant::now() + Duration::from_secs(60)
    }

    fn samples(root: TypeDef, definitions: TypeDefinitions, values: Vec<DynamicValue>) -> Records {
        let schema = Arc::new(SchemaBundle { root, definitions });
        values
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                Arc::new(TestSample {
                    time: Time::from_nanos(index as i64),
                    payload: DynamicPayload::new(schema.clone(), value).unwrap(),
                })
            })
            .collect()
    }

    fn numbers(values: &[f64]) -> Records {
        samples(
            TypeDef::Primitive(PrimitiveTypeDef::F64),
            Default::default(),
            values.iter().copied().map(DynamicValue::Float64).collect(),
        )
    }

    fn project(records: &Records, source: &str) -> SeriesData<TestSample> {
        let mut series = SeriesData::default();
        let pending = series.refresh(
            Some(records),
            "",
            &Conversion::new(source.to_owned()),
            later(),
        );
        assert!(!pending);
        series
    }

    fn points(values: &[(i64, f64)]) -> Vec<(Time, f64)> {
        values
            .iter()
            .map(|(time, value)| (Time::from_nanos(*time), *value))
            .collect()
    }

    fn state(start: i64, end: Option<i64>, index: u32, name: &str) -> StateInterval {
        StateInterval {
            start: Time::from_nanos(start),
            end: end.map(Time::from_nanos),
            index,
            name: name.to_owned(),
        }
    }

    fn motion() -> (TypeDef, TypeDefinitions, impl Fn(u32, &str) -> DynamicValue) {
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
        (TypeDef::Named(name), definitions, variant)
    }

    #[test]
    fn projection_applies_conversion() {
        let series = project(&numbers(&[1.0, 2.0]), "value * 2.0 + 1.0");

        assert_eq!(series.segments, [points(&[(0, 3.0), (1, 5.0)])]);
        assert_eq!(series.gaps, 0);
        assert!(!series.is_state());
    }

    #[test]
    fn overflowing_conversion_breaks_the_line() {
        let series = project(&numbers(&[1.0, f64::MAX, 2.0]), "value * 10.0");

        assert_eq!(
            series.segments,
            [points(&[(0, 10.0)]), points(&[(2, 20.0)])]
        );
        assert_eq!(series.gaps, 1);
        assert_eq!(series.issue, None);
    }

    #[test]
    fn script_errors_are_gaps_with_the_latest_issue() {
        let series = project(
            &numbers(&[1.0, 2.0]),
            "if value > 1.0 { value.x } else { value }",
        );

        assert_eq!(series.segments, [points(&[(0, 1.0)])]);
        assert_eq!(series.gaps, 1);
        assert!(series.issue.is_some());
    }

    #[test]
    fn unit_results_skip_samples_without_issues() {
        let series = project(
            &numbers(&[1.0, 5.0, 2.0]),
            "if value > 4.0 { () } else { value }",
        );

        assert_eq!(series.segments, [points(&[(0, 1.0)]), points(&[(2, 2.0)])]);
        assert_eq!((series.gaps, series.issue), (0, None));
    }

    #[test]
    fn enum_samples_merge_into_state_runs() {
        let (root, definitions, variant) = motion();
        let records = samples(
            root,
            definitions,
            vec![
                variant(0, "Stand"),
                variant(0, "Stand"),
                variant(1, "Walk"),
                variant(0, "Stand"),
            ],
        );

        let series = project(&records, "");

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
    fn scripts_turn_enums_into_numbers_and_numbers_into_states() {
        let (root, definitions, variant) = motion();
        let records = samples(
            root,
            definitions,
            vec![variant(0, "Stand"), variant(1, "Walk")],
        );
        let series = project(&records, "value.variant_index");
        assert_eq!(series.segments, [points(&[(0, 0.0), (1, 1.0)])]);
        assert!(!series.is_state());

        let records = numbers(&[0.2, 0.7, 0.9, 0.1]);
        let series = project(&records, "value > 0.5");
        assert_eq!(
            series.states,
            [
                state(0, Some(1), 0, "false"),
                state(1, Some(3), 1, "true"),
                state(3, None, 0, "false"),
            ]
        );

        let series = project(&records, "if value > 0.5 { \"high\" } else { \"low\" }");
        assert_eq!(
            series.states,
            [
                state(0, Some(1), text_index("low"), "low"),
                state(1, Some(3), text_index("high"), "high"),
                state(3, None, text_index("low"), "low"),
            ]
        );
    }

    #[test]
    fn unavailable_states_end_the_current_run() {
        let optional_bool = |value: Option<bool>| {
            DynamicValue::Optional(value.map(|value| Box::new(DynamicValue::Bool(value))))
        };
        let records = samples(
            TypeDef::Optional(Box::new(TypeDef::Primitive(PrimitiveTypeDef::Bool))),
            Default::default(),
            vec![
                optional_bool(Some(true)),
                optional_bool(None),
                optional_bool(Some(true)),
                optional_bool(Some(false)),
            ],
        );

        let series = project(&records, "");

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

    #[test]
    fn new_snapshots_reuse_converted_samples() {
        let records = numbers(&[1.0, 2.0, 3.0]);
        let conversion = Conversion::new("value * 2.0".to_owned());
        let mut series = SeriesData::default();
        series.refresh(Some(&records), "", &conversion, later());

        // The oldest sample left the window and a new one arrived. Without
        // time to convert, only the shared samples remain projected.
        let added = numbers(&[0.0, 0.0, 0.0, 4.0]);
        let next: Records = [records[1].clone(), records[2].clone(), added[3].clone()].into();
        assert!(series.refresh(Some(&next), "", &conversion, Instant::now()));
        assert_eq!(series.segments, [points(&[(1, 4.0), (2, 6.0)])]);

        assert!(!series.refresh(Some(&next), "", &conversion, later()));
        assert_eq!(series.segments, [points(&[(1, 4.0), (2, 6.0), (3, 8.0)])]);
    }

    #[test]
    fn changing_the_script_reprojects_the_history() {
        let records = numbers(&[1.0, 2.0]);
        let mut series = SeriesData::default();
        series.refresh(Some(&records), "", &Conversion::default(), later());
        assert_eq!(series.segments, [points(&[(0, 1.0), (1, 2.0)])]);

        series.refresh(
            Some(&records),
            "",
            &Conversion::new("-value".to_owned()),
            later(),
        );
        assert_eq!(series.segments, [points(&[(0, -1.0), (1, -2.0)])]);
    }

    #[test]
    fn items_keep_their_kind_without_valid_samples() {
        let records = numbers(&[0.7]);
        let mut series = project(&records, "value > 0.5");
        assert!(series.is_state());

        series.refresh(
            Some(&records),
            "",
            &Conversion::new("value.x".to_owned()),
            later(),
        );
        assert!(series.states.is_empty() && series.segments.is_empty());
        assert!(series.is_state());
    }
}
