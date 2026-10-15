//! Serializable, bounded expressions over timestamped sources. Storage and payload
//! decoding are deliberately outside the evaluator.
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use ros_z_debug::query::{Backend, CursorHandle, TimeEncoding, TimeProjection};
//! use ros_z_debug::query::plan::{self, InputSpec, Selection, Source, State, TopicSource};
//! # fn example(backend: Arc<dyn Backend>, cursor: CursorHandle) -> ros_z_debug::Result<()> {
//! let images = Source::from(TopicSource::new("/robot/image",
//!     TimeProjection::field(".header.stamp", TimeEncoding::RosTime)?));
//! let objects = Source::from(TopicSource::new("/robot/objects",
//!     TimeProjection::field(".time", TimeEncoding::RosZTime)?));
//! let time = plan::available([objects.latest(plan::cursor()).effective_time()]).min();
//! let observation = images.exact(time).or_else(images.latest(plan::cursor()))
//!     .attach([InputSpec { source: objects, selection: Selection::Exact }])
//!     .observe(backend, cursor)?;
//! if let State::Ready(Some(frame)) = observation.snapshot().state {
//!     // Payloads are still raw/shared. Decode only the selected image and inputs.
//!     println!("image time: {:?}", frame.primary.record.time);
//! }
//! # Ok(()) }
//! ```
use std::{marker::PhantomData, num::NonZeroUsize, sync::Arc, time::Duration};

use ros_z::time::Time;
use serde::{Deserialize, Serialize};

use super::*;
use crate::ObservationPolicy;

const MAX_NODES: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicSource {
    pub topic: String,
    pub projection: TimeProjection,
    #[serde(skip)]
    pub live_policy: ObservationPolicy,
}

impl TopicSource {
    pub fn new(topic: impl Into<String>, projection: TimeProjection) -> Self {
        Self {
            topic: topic.into(),
            projection,
            live_policy: Default::default(),
        }
    }
}

/// Source fallback depends on presence, never on payload or query success.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source(Vec<TopicSource>);

impl From<TopicSource> for Source {
    fn from(source: TopicSource) -> Self {
        Self(vec![source])
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Selection {
    AtOrBefore { count: NonZeroUsize },
    After { count: NonZeroUsize },
    Exact,
    Bracket { max_gap: Duration },
    Window { before: Duration, after: Duration },
}

impl Selection {
    pub fn latest() -> Self {
        Self::AtOrBefore {
            count: NonZeroUsize::MIN,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputSpec {
    pub source: Source,
    pub selection: Selection,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum Node {
    Cursor,
    Select {
        source: Source,
        at: Box<Node>,
        selection: Selection,
        required: Vec<InputSpec>,
    },
    Time(Box<Node>),
    Available(Vec<Node>),
    Min(Box<Node>),
    OrElse(Box<Node>, Box<Node>),
    Attach {
        primary: Box<Node>,
        inputs: Vec<InputSpec>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Time,
    Times,
    Records,
    Frames,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryPlan {
    root: Node,
}

#[derive(Clone)]
pub struct Expr<T> {
    node: Node,
    marker: PhantomData<fn() -> T>,
}

impl<T> Expr<T> {
    fn new(node: Node) -> Self {
        Self {
            node,
            marker: PhantomData,
        }
    }
    pub fn plan(self) -> QueryPlan {
        QueryPlan { root: self.node }
    }
}

pub fn cursor() -> Expr<Option<Time>> {
    Expr::new(Node::Cursor)
}

impl Source {
    pub fn first_present(sources: impl IntoIterator<Item = TopicSource>) -> Self {
        Self(sources.into_iter().collect())
    }
    pub fn select(&self, selection: Selection, at: Expr<Option<Time>>) -> Expr<Vec<Sample>> {
        Expr::new(Node::Select {
            source: self.clone(),
            at: Box::new(at.node),
            selection,
            required: vec![],
        })
    }
    pub fn latest(&self, at: Expr<Option<Time>>) -> Expr<Option<Sample>> {
        Expr::new(self.select(Selection::latest(), at).node)
    }
    pub fn exact(&self, at: Expr<Option<Time>>) -> Expr<Option<Sample>> {
        Expr::new(self.select(Selection::Exact, at).node)
    }
}

#[cfg(test)]
mod tests;

impl Expr<Option<Sample>> {
    pub fn effective_time(self) -> Expr<Option<Time>> {
        Expr::new(Node::Time(Box::new(self.node)))
    }
    pub fn or_else(self, fallback: Self) -> Self {
        Self::new(Node::OrElse(Box::new(self.node), Box::new(fallback.node)))
    }
    pub fn attach(self, inputs: impl IntoIterator<Item = InputSpec>) -> Expr<Option<Frame>> {
        Expr::new(Node::Attach {
            primary: Box::new(self.node),
            inputs: inputs.into_iter().collect(),
        })
    }
}

pub fn available(values: impl IntoIterator<Item = Expr<Option<Time>>>) -> Expr<Vec<Time>> {
    Expr::new(Node::Available(
        values.into_iter().map(|value| value.node).collect(),
    ))
}

impl Expr<Vec<Time>> {
    pub fn min(self) -> Expr<Option<Time>> {
        Expr::new(Node::Min(Box::new(self.node)))
    }
}

impl Expr<Vec<Sample>> {
    /// Constrain candidate eligibility before selection/counting. Unlike attached
    /// inputs, a required input with no match rejects that primary candidate.
    pub fn requiring(mut self, inputs: impl IntoIterator<Item = InputSpec>) -> Self {
        let Node::Select { required, .. } = &mut self.node else {
            unreachable!("record collections are selections")
        };
        required.extend(inputs);
        self
    }
    pub fn attach(self, inputs: impl IntoIterator<Item = InputSpec>) -> Expr<Vec<Frame>> {
        Expr::new(Node::Attach {
            primary: Box::new(self.node),
            inputs: inputs.into_iter().collect(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum State<T> {
    Pending,
    Ready(T),
    Failed(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sample {
    pub topic: TopicInfo,
    pub record: Arc<RawRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame<R = Sample> {
    pub primary: R,
    pub inputs: Vec<State<Vec<R>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Value<R = Sample> {
    Time(Option<Time>),
    Times(Vec<Time>),
    Records(Vec<R>),
    Frames(Vec<Frame<R>>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dependency {
    pub source: TopicSource,
    pub resolution: State<()>,
    pub coverage: Option<Coverage>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evaluation {
    pub cursor: Time,
    pub valid_for: CursorRange,
    pub bounds: Option<(Time, Time)>,
    /// First rejected candidate of a required-input selection, in evaluation order.
    pub missing: Option<MissingMatches>,
    pub state: State<Value>,
    pub dependencies: Vec<Dependency>,
}

impl Evaluation {
    pub(super) fn unresolved(cursor: Time, state: State<Value>) -> Self {
        Self {
            cursor,
            valid_for: CursorRange {
                start: cursor,
                end: None,
            },
            bounds: None,
            missing: None,
            state,
            dependencies: vec![],
        }
    }
    pub fn settled(&self) -> bool {
        self.dependencies
            .iter()
            .all(|d| d.resolution != State::Pending)
    }
}

/// Implemented by immutable Live snapshots and pinned recording indexes.
pub trait TimeIndex {
    fn len(&self) -> usize;
    fn time(&self, index: usize) -> Time;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct SourceView {
    pub topic: TopicInfo,
    pub index: Arc<dyn TimeIndex + Send + Sync>,
    pub coverage: Coverage,
    pub diagnostics: Vec<String>,
}

/// Absent is known catalog absence, distinct from an unresolved source.
pub enum SourceState {
    Absent,
    Pending,
    Failed(String),
    Ready(SourceView),
}

#[derive(Clone, Copy)]
struct Reference {
    source: usize,
    slot: usize,
    time: Time,
}

pub type LoadedRecord = std::result::Result<Arc<RawRecord>, String>;

impl QueryPlan {
    /// Only the first advertised alternative of each source group acquires data.
    /// Catalog changes invalidate this set independently of sample availability.
    pub fn active_sources(&self, catalog: &TopicSnapshot) -> Result<Vec<bool>> {
        let sources = self.sources()?;
        let mut active = vec![false; sources.len()];
        visit_source_groups(&self.root, &mut |group| {
            if let Some(source) = group.0.iter().find(|source| {
                catalog
                    .topics
                    .iter()
                    .any(|topic| topic.name == source.topic)
            }) {
                let index = sources
                    .iter()
                    .position(|known| known == source)
                    .expect("collected source");
                active[index] = true;
            }
        });
        Ok(active)
    }
    /// Validate remote shape, source identity and independently attached selections.
    /// Index completeness cannot be proven by a client without the recording.
    pub fn validate_result(&self, result: &mut Evaluation, catalog: &[TopicInfo]) -> Result<()> {
        self.validate()?;
        if !result.valid_for.contains(result.cursor) {
            return Err(query_error("invalid plan validity range"));
        }
        let sources = self.sources()?;
        if result.dependencies.len() > sources.len()
            || result.dependencies.iter().any(|d| {
                !sources.iter().any(|source| {
                    source.topic == d.source.topic && source.projection == d.source.projection
                })
            })
        {
            return Err(query_error("unknown plan dependency"));
        }
        let mut count = 0;
        let mut sample = |sample: &mut Sample| -> Result<()> {
            count += 1;
            if count > MAX_ROWS {
                return Err(query_error("plan response exceeds record limit"));
            }
            let known = catalog
                .iter()
                .find(|topic| {
                    topic.name == sample.topic.name && topic.type_info == sample.topic.type_info
                })
                .ok_or_else(|| query_error("plan response topic/schema mismatch"))?;
            if !sources.iter().any(|source| source.topic == known.name) {
                return Err(query_error("undeclared response source"));
            }
            sample.topic.schema = known.schema.clone();
            Ok(())
        };
        let State::Ready(value) = &mut result.state else {
            return Ok(());
        };
        let mut remaining = MAX_NODES;
        let kind = validate_node(&self.root, &mut remaining)?;
        fn primary_matches(node: &Node, record: &Sample, catalog: &[TopicInfo]) -> bool {
            match node {
                Node::Select { source, .. } => source
                    .0
                    .iter()
                    .find(|source| catalog.iter().any(|topic| topic.name == source.topic))
                    .is_some_and(|source| source.topic == record.topic.name),
                Node::OrElse(a, b) => {
                    primary_matches(a, record, catalog) || primary_matches(b, record, catalog)
                }
                Node::Attach { primary, .. } => primary_matches(primary, record, catalog),
                _ => false,
            }
        }
        match (kind, value) {
            (Kind::Time, Value::Time(_)) => {}
            (Kind::Times, Value::Times(times)) if times.len() <= MAX_NODES => {}
            (Kind::Records, Value::Records(records)) => {
                for record in records {
                    if !primary_matches(&self.root, record, catalog) {
                        return Err(query_error("invalid primary source"));
                    }
                    sample(record)?;
                }
            }
            (Kind::Frames, Value::Frames(frames)) => {
                let Node::Attach { inputs, .. } = &self.root else {
                    return Err(query_error("invalid frame root"));
                };
                for frame in frames {
                    if !primary_matches(&self.root, &frame.primary, catalog) {
                        return Err(query_error("invalid primary source"));
                    }
                    sample(&mut frame.primary)?;
                    if frame.inputs.len() != inputs.len() {
                        return Err(query_error("plan input count mismatch"));
                    }
                    for (state, spec) in frame.inputs.iter_mut().zip(inputs) {
                        if let State::Ready(records) = state {
                            for record in records.iter_mut() {
                                sample(record)?;
                                if !spec
                                    .source
                                    .0
                                    .iter()
                                    .find(|source| {
                                        catalog.iter().any(|topic| topic.name == source.topic)
                                    })
                                    .is_some_and(|source| source.topic == record.topic.name)
                                {
                                    return Err(query_error("plan input source mismatch"));
                                }
                            }
                            if !valid_selection(records, frame.primary.record.time, &spec.selection)
                            {
                                return Err(query_error("invalid attached selection"));
                            }
                        }
                    }
                }
            }
            _ => return Err(query_error("plan result type mismatch")),
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<()> {
        let mut remaining = MAX_NODES;
        validate_node(&self.root, &mut remaining)?;
        Ok(())
    }

    pub fn sources(&self) -> Result<Vec<TopicSource>> {
        self.validate()?;
        let mut sources = Vec::new();
        visit_sources(&self.root, &mut |source| {
            if !sources.contains(source) {
                sources.push(source.clone());
            }
        });
        if sources.len() > 64 {
            return Err(query_error("query plan exceeds 64 sources"));
        }
        Ok(sources)
    }

    pub fn resolve_topics(&mut self, namespace: &str) -> Result<()> {
        self.validate()?;
        let identity = crate::TargetIdentity::new(namespace)?;
        map_sources(&mut self.root, &mut |source| {
            source.topic = crate::TopicReference::new(&source.topic)?.resolve(&identity)?;
            Ok(())
        })
    }

    pub fn evaluate(
        &self,
        cursor: Time,
        views: &[SourceState],
        mut load: impl FnMut(usize, usize) -> Result<Arc<RawRecord>>,
    ) -> Result<Evaluation> {
        self.evaluate_batched(cursor, views, |locations| {
            locations
                .iter()
                .map(|&(source, slot)| load(source, slot).map_err(|error| error.to_string()))
                .collect()
        })
    }

    /// Selection is index-only. Disk adapters receive all unique locations at
    /// once, allowing payload reads to be grouped by chunk without changing semantics.
    pub fn evaluate_batched(
        &self,
        cursor: Time,
        views: &[SourceState],
        load: impl FnOnce(&[(usize, usize)]) -> Vec<LoadedRecord>,
    ) -> Result<Evaluation> {
        let sources = self.sources()?;
        if sources.len() != views.len() {
            return Err(query_error("plan source count mismatch"));
        }
        let mut evaluator = Evaluator {
            sources: &sources,
            views,
            cursor,
            used: vec![false; sources.len()],
            truncated: vec![false; sources.len()],
            remaining: MAX_ROWS,
            missing: None,
        };
        let state = evaluator.eval(&self.root);
        let mut locations = std::collections::BTreeSet::new();
        let mut remember = |reference: &Reference| {
            locations.insert((reference.source, reference.slot));
        };
        match &state {
            State::Ready(Value::Records(records)) => records.iter().for_each(&mut remember),
            State::Ready(Value::Frames(frames)) => {
                for frame in frames {
                    remember(&frame.primary);
                    for input in &frame.inputs {
                        if let State::Ready(records) = input {
                            records.iter().for_each(&mut remember);
                        }
                    }
                }
            }
            _ => {}
        }
        let locations: Vec<_> = locations.into_iter().collect();
        let records = load(&locations);
        if records.len() != locations.len() {
            return Err(query_error("payload loader returned invalid record count"));
        }
        let records: std::collections::HashMap<_, _> = locations.into_iter().zip(records).collect();
        let mut bytes = 0usize;
        let mut loaded = std::collections::HashMap::new();
        let state = map_state(state, &mut |reference: Reference| {
            if let Some(sample) = loaded.get(&(reference.source, reference.slot)) {
                return Ok(Clone::clone(sample));
            }
            let SourceState::Ready(view) = &views[reference.source] else {
                return Err(query_error("selected source is not ready"));
            };
            let record = records[&(reference.source, reference.slot)]
                .as_ref()
                .map_err(query_error)?
                .clone();
            if record.time != reference.time {
                return Err(query_error("payload timestamp differs from selected index"));
            }
            bytes = bytes.saturating_add(record.data.len());
            if bytes as u64 > MAX_PAYLOAD_BYTES {
                return Err(query_error("plan payload exceeds 32 MiB"));
            }
            let sample = Sample {
                topic: view.topic.clone(),
                record,
            };
            loaded.insert((reference.source, reference.slot), sample.clone());
            Ok(sample)
        });
        let dependencies = sources
            .iter()
            .enumerate()
            .filter(|(i, _)| evaluator.used[*i])
            .map(|(i, source)| {
                let (resolution, coverage, diagnostics) = match &views[i] {
                    SourceState::Absent => (State::Ready(()), None, vec![]),
                    SourceState::Pending => (State::Pending, None, vec![]),
                    SourceState::Failed(error) => (State::Failed(error.clone()), None, vec![]),
                    SourceState::Ready(view) => {
                        let mut coverage = view.coverage;
                        coverage.truncated |= evaluator.truncated[i];
                        (State::Ready(()), Some(coverage), view.diagnostics.clone())
                    }
                };
                Dependency {
                    source: source.clone(),
                    resolution,
                    coverage,
                    diagnostics,
                }
            })
            .collect();
        let valid_for = plan_range(&self.root, &sources, views, cursor);
        let mut bounds = None;
        let primary = match &self.root {
            Node::Attach { primary, .. } => primary.as_ref(),
            other => other,
        };
        if let Node::Select { source, .. } = primary {
            let view = source.0.iter().find_map(|source| {
                let i = sources.iter().position(|known| known == source)?;
                match &views[i] {
                    SourceState::Absent => None,
                    other => Some(other),
                }
            });
            if let Some(SourceState::Ready(view)) = view
                && !view.index.is_empty()
            {
                bounds = Some((view.index.time(0), view.index.time(view.index.len() - 1)));
            }
        }
        Ok(Evaluation {
            cursor,
            valid_for,
            bounds,
            missing: evaluator.missing,
            state,
            dependencies,
        })
    }
}

fn plan_range(
    node: &Node,
    sources: &[TopicSource],
    views: &[SourceState],
    cursor: Time,
) -> CursorRange {
    let all = CursorRange {
        start: Time::zero(),
        end: None,
    };
    let point = CursorRange {
        start: cursor,
        end: Some(cursor.saturating_add(Duration::from_nanos(1))).filter(|end| *end > cursor),
    };
    let intersect = |a: CursorRange, b: CursorRange| CursorRange {
        start: a.start.max(b.start),
        end: match (a.end, b.end) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        },
    };
    match node {
        Node::Cursor => point,
        Node::Select {
            source,
            at,
            selection,
            ..
        } if matches!(at.as_ref(), Node::Cursor) => {
            for source in &source.0 {
                let i = sources
                    .iter()
                    .position(|known| known == source)
                    .expect("collected source");
                match &views[i] {
                    SourceState::Absent => continue,
                    SourceState::Ready(view) => {
                        return selection_range(view.index.as_ref(), cursor, selection);
                    }
                    _ => return all,
                }
            }
            all
        }
        Node::Select { at, .. } | Node::Time(at) | Node::Min(at) => {
            plan_range(at, sources, views, cursor)
        }
        Node::Available(values) => values.iter().fold(all, |range, node| {
            intersect(range, plan_range(node, sources, views, cursor))
        }),
        Node::OrElse(a, b) => intersect(
            plan_range(a, sources, views, cursor),
            plan_range(b, sources, views, cursor),
        ),
        Node::Attach { primary, .. } => plan_range(primary, sources, views, cursor),
    }
}

fn selection_range(index: &dyn TimeIndex, cursor: Time, selection: &Selection) -> CursorRange {
    let mut range = CursorRange {
        start: Time::zero(),
        end: None,
    };
    let mut constrain = |boundary: &dyn Fn(Time) -> Option<Time>| {
        let (mut lo, mut hi) = (0, index.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if boundary(index.time(mid)).is_some_and(|t| t <= cursor) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        if lo > 0
            && let Some(start) = boundary(index.time(lo - 1))
        {
            range.start = range.start.max(start);
        }
        if lo < index.len()
            && let Some(end) = boundary(index.time(lo))
        {
            range.end = Some(range.end.map_or(end, |old| old.min(end)));
        }
    };
    match selection {
        Selection::AtOrBefore { .. } | Selection::After { .. } => constrain(&Some),
        Selection::Window { before, after } => {
            constrain(&|time| Some(time.saturating_sub(*after)));
            constrain(&|time| {
                time.duration_since(Time::zero())
                    .checked_add(*before)?
                    .checked_add(Duration::from_nanos(1))
                    .map(|d| Time::zero().saturating_add(d))
            });
        }
        _ => {
            return CursorRange {
                start: cursor,
                end: Some(cursor.saturating_add(Duration::from_nanos(1)))
                    .filter(|end| *end > cursor),
            };
        }
    }
    range
}

struct Evaluator<'a> {
    sources: &'a [TopicSource],
    views: &'a [SourceState],
    cursor: Time,
    used: Vec<bool>,
    truncated: Vec<bool>,
    remaining: usize,
    missing: Option<MissingMatches>,
}

impl Evaluator<'_> {
    fn source(&mut self, source: &Source) -> State<Option<usize>> {
        for candidate in &source.0 {
            let index = self
                .sources
                .iter()
                .position(|known| known == candidate)
                .expect("collected plan source");
            self.used[index] = true;
            match &self.views[index] {
                SourceState::Absent => continue,
                SourceState::Pending => return State::Pending,
                SourceState::Failed(error) => return State::Failed(error.clone()),
                SourceState::Ready(_) => return State::Ready(Some(index)),
            }
        }
        State::Ready(None)
    }

    fn select(
        &mut self,
        source: &Source,
        at: Time,
        selection: &Selection,
        required: &[InputSpec],
    ) -> State<Vec<Reference>> {
        let index = match self.source(source) {
            State::Ready(Some(index)) => index,
            State::Ready(None) => return State::Ready(vec![]),
            State::Pending => return State::Pending,
            State::Failed(error) => return State::Failed(error),
        };
        let mut constraints = Vec::new();
        for input in required {
            match self.source(&input.source) {
                State::Ready(Some(index)) => constraints.push((index, &input.selection)),
                State::Ready(None) => return State::Ready(vec![]),
                State::Pending => return State::Pending,
                State::Failed(error) => return State::Failed(error),
            }
        }
        let SourceState::Ready(view) = &self.views[index] else {
            unreachable!()
        };
        let mut missing = None;
        let eligible = |slot| {
            let mut missing_inputs =
                constraints
                    .iter()
                    .enumerate()
                    .filter_map(|(i, (index, selection))| {
                        let SourceState::Ready(other) = &self.views[*index] else {
                            unreachable!()
                        };
                        select_indices(
                            other.index.as_ref(),
                            view.index.time(slot),
                            selection,
                            1,
                            |_| true,
                        )
                        .0
                        .is_empty()
                        .then_some(i)
                    });
            let Some(first) = missing_inputs.next() else {
                return true;
            };
            missing.get_or_insert_with(|| MissingMatches {
                time: view.index.time(slot),
                inputs: std::iter::once(first).chain(missing_inputs).collect(),
            });
            false
        };
        let (slots, truncated) =
            select_indices(view.index.as_ref(), at, selection, self.remaining, eligible);
        if self.missing.is_none() {
            self.missing = missing;
        }
        self.truncated[index] |= truncated;
        if slots.len() > self.remaining || (truncated && slots.is_empty()) {
            self.truncated[index] = true;
            return State::Failed("plan exceeds record limit".into());
        }
        self.remaining -= slots.len();
        State::Ready(
            slots
                .into_iter()
                .map(|slot| Reference {
                    source: index,
                    slot,
                    time: view.index.time(slot),
                })
                .collect(),
        )
    }

    fn eval(&mut self, node: &Node) -> State<Value<Reference>> {
        match node {
            Node::Cursor => State::Ready(Value::Time(Some(self.cursor))),
            Node::Select {
                source,
                at,
                selection,
                required,
            } => match self.eval(at) {
                State::Ready(Value::Time(Some(at))) => {
                    match self.select(source, at, selection, required) {
                        State::Ready(records) => State::Ready(Value::Records(records)),
                        State::Pending => State::Pending,
                        State::Failed(error) => State::Failed(error),
                    }
                }
                State::Ready(Value::Time(None)) => State::Ready(Value::Records(vec![])),
                State::Pending => State::Pending,
                State::Failed(error) => State::Failed(error),
                _ => unreachable!("validated time expression"),
            },
            Node::Time(value) => match self.eval(value) {
                State::Ready(Value::Records(records)) if records.len() <= 1 => {
                    State::Ready(Value::Time(records.first().map(|record| record.time)))
                }
                State::Ready(_) => {
                    State::Failed("timestamp extraction requires at most one record".into())
                }
                other => other,
            },
            Node::Available(values) => {
                let mut times = Vec::new();
                for value in values {
                    if let State::Ready(Value::Time(Some(time))) = self.eval(value) {
                        times.push(time);
                    }
                }
                State::Ready(Value::Times(times))
            }
            Node::Min(value) => match self.eval(value) {
                State::Ready(Value::Times(times)) => {
                    State::Ready(Value::Time(times.into_iter().min()))
                }
                other => other,
            },
            Node::OrElse(first, fallback) => match self.eval(first) {
                State::Ready(Value::Records(records)) if records.is_empty() => self.eval(fallback),
                other => other,
            },
            Node::Attach { primary, inputs } => match self.eval(primary) {
                State::Ready(Value::Records(records)) => State::Ready(Value::Frames(
                    records
                        .into_iter()
                        .map(|primary| {
                            let inputs = inputs
                                .iter()
                                .map(|input| {
                                    self.select(&input.source, primary.time, &input.selection, &[])
                                })
                                .collect();
                            Frame { primary, inputs }
                        })
                        .collect(),
                )),
                other => other,
            },
        }
    }
}

/// Index-only selection shared by memory and disk adapters. Required constraints
/// are evaluated lazily while scanning candidates, before count/bracket selection.
fn select_indices(
    index: &dyn TimeIndex,
    at: Time,
    selection: &Selection,
    limit: usize,
    mut eligible: impl FnMut(usize) -> bool,
) -> (Vec<usize>, bool) {
    let partition = |predicate: &dyn Fn(Time) -> bool| {
        let (mut lo, mut hi) = (0, index.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if predicate(index.time(mid)) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    };
    let upper = partition(&|time| time <= at);
    if matches!(selection, Selection::Exact) {
        let start = partition(&|time| time < at);
        return (
            (start..upper)
                .rev()
                .find(|&slot| eligible(slot))
                .into_iter()
                .collect(),
            false,
        );
    }
    if matches!(selection, Selection::Exact | Selection::Bracket { .. }) {
        let before = (0..upper).rev().find(|&slot| eligible(slot));
        if let Some(before) = before {
            if index.time(before) == at {
                return (vec![before], false);
            }
            if let Selection::Bracket { max_gap } = selection
                && let Some(after) = (upper..index.len()).find(|&slot| eligible(slot))
                && index.time(after).duration_since(index.time(before)) <= *max_gap
            {
                let end = partition(&|time| time <= index.time(after));
                let after = (after..end)
                    .rev()
                    .find(|&slot| eligible(slot))
                    .expect("eligible endpoint");
                return (vec![before, after], false);
            }
        }
        return (vec![], false);
    }
    let (start, end, reverse, count) = match selection {
        Selection::AtOrBefore { count } => (0, upper, true, count.get()),
        Selection::After { count } => (upper, index.len(), false, count.get()),
        Selection::Window { before, after } => (
            partition(&|t| t < at.saturating_sub(*before)),
            partition(&|t| t <= at.saturating_add(*after)),
            true,
            usize::MAX,
        ),
        _ => unreachable!(),
    };
    let mut selected = Vec::new();
    let mut truncated = false;
    for offset in 0..end - start {
        let slot = if reverse {
            end - 1 - offset
        } else {
            start + offset
        };
        if !eligible(slot) {
            continue;
        }
        if selected.len() == limit {
            truncated = true;
            break;
        }
        selected.push(slot);
        if selected.len() == count {
            break;
        }
    }
    if reverse {
        selected.reverse();
    }
    (selected, truncated)
}

fn map_state(
    state: State<Value<Reference>>,
    load: &mut impl FnMut(Reference) -> Result<Sample>,
) -> State<Value> {
    let value = match state {
        State::Ready(value) => value,
        State::Pending => return State::Pending,
        State::Failed(e) => return State::Failed(e),
    };
    let result: Result<Value> = (|| {
        Ok(match value {
            Value::Time(time) => Value::Time(time),
            Value::Times(times) => Value::Times(times),
            Value::Records(records) => {
                Value::Records(records.into_iter().map(&mut *load).collect::<Result<_>>()?)
            }
            Value::Frames(frames) => Value::Frames(
                frames
                    .into_iter()
                    .map(|frame| {
                        let primary = load(frame.primary)?;
                        let inputs = frame
                            .inputs
                            .into_iter()
                            .map(|input| match input {
                                State::Pending => State::Pending,
                                State::Failed(e) => State::Failed(e),
                                State::Ready(records) => {
                                    match records.into_iter().map(&mut *load).collect::<Result<_>>()
                                    {
                                        Ok(records) => State::Ready(records),
                                        Err(e) => State::Failed(e.to_string()),
                                    }
                                }
                            })
                            .collect();
                        Ok(Frame { primary, inputs })
                    })
                    .collect::<Result<_>>()?,
            ),
        })
    })();
    match result {
        Ok(value) => State::Ready(value),
        Err(error) => State::Failed(error.to_string()),
    }
}

fn validate_node(node: &Node, remaining: &mut usize) -> Result<Kind> {
    *remaining = remaining
        .checked_sub(1)
        .ok_or_else(|| query_error("query plan exceeds 128 nodes"))?;
    let require = |actual, expected| {
        if actual == expected {
            Ok(())
        } else {
            Err(query_error("query plan type mismatch"))
        }
    };
    let source = |source: &Source| -> Result<()> {
        if source.0.is_empty() || source.0.len() > 32 {
            return Err(query_error("invalid source alternatives"));
        }
        for source in &source.0 {
            crate::TopicReference::new(&source.topic)?;
        }
        Ok(())
    };
    match node {
        Node::Cursor => Ok(Kind::Time),
        Node::Select {
            source: input,
            at,
            required,
            ..
        } => {
            source(input)?;
            if required.len() > 32 {
                return Err(query_error("too many required inputs"));
            }
            for input in required {
                source(&input.source)?;
            }
            require(validate_node(at, remaining)?, Kind::Time)?;
            Ok(Kind::Records)
        }
        Node::Time(value) => {
            require(validate_node(value, remaining)?, Kind::Records)?;
            fn single(node: &Node) -> bool {
                match node {
                    Node::Select {
                        selection: Selection::Exact,
                        ..
                    } => true,
                    Node::Select {
                        selection: Selection::AtOrBefore { count } | Selection::After { count },
                        ..
                    } => count.get() == 1,
                    Node::OrElse(a, b) => single(a) && single(b),
                    _ => false,
                }
            }
            if !single(value) {
                return Err(query_error(
                    "timestamp extraction requires a scalar selection",
                ));
            }
            Ok(Kind::Time)
        }
        Node::Available(values) => {
            for value in values {
                require(validate_node(value, remaining)?, Kind::Time)?;
            }
            Ok(Kind::Times)
        }
        Node::Min(value) => {
            require(validate_node(value, remaining)?, Kind::Times)?;
            Ok(Kind::Time)
        }
        Node::OrElse(a, b) => {
            require(validate_node(a, remaining)?, Kind::Records)?;
            require(validate_node(b, remaining)?, Kind::Records)?;
            Ok(Kind::Records)
        }
        Node::Attach { primary, inputs } => {
            require(validate_node(primary, remaining)?, Kind::Records)?;
            if inputs.len() > 32 {
                return Err(query_error("too many attached inputs"));
            }
            for input in inputs {
                source(&input.source)?;
            }
            Ok(Kind::Frames)
        }
    }
}

fn valid_selection(records: &[Sample], at: Time, selection: &Selection) -> bool {
    if records.windows(2).any(|pair| {
        (pair[0].record.time, pair[0].record.id) > (pair[1].record.time, pair[1].record.id)
    }) {
        return false;
    }
    match selection {
        Selection::AtOrBefore { count } => {
            records.len() <= count.get() && records.iter().all(|r| r.record.time <= at)
        }
        Selection::After { count } => {
            records.len() <= count.get() && records.iter().all(|r| r.record.time > at)
        }
        Selection::Exact => records.len() <= 1 && records.iter().all(|r| r.record.time == at),
        Selection::Window { before, after } => records.iter().all(|r| {
            r.record.time >= at.saturating_sub(*before)
                && r.record.time <= at.saturating_add(*after)
        }),
        Selection::Bracket { max_gap } => match records {
            [] => true,
            [one] => one.record.time == at,
            [before, after] => {
                before.record.time < at
                    && after.record.time > at
                    && after.record.time.duration_since(before.record.time) <= *max_gap
            }
            _ => false,
        },
    }
}

fn visit_sources(node: &Node, f: &mut impl FnMut(&TopicSource)) {
    visit_source_groups(node, &mut |group| {
        for source in &group.0 {
            f(source);
        }
    });
}

fn visit_source_groups(node: &Node, f: &mut impl FnMut(&Source)) {
    match node {
        Node::Cursor => {}
        Node::Select {
            source,
            at,
            required,
            ..
        } => {
            f(source);
            visit_source_groups(at, f);
            for input in required {
                f(&input.source);
            }
        }
        Node::Time(node) | Node::Min(node) => visit_source_groups(node, f),
        Node::Available(nodes) => {
            for node in nodes {
                visit_source_groups(node, f);
            }
        }
        Node::OrElse(a, b) => {
            visit_source_groups(a, f);
            visit_source_groups(b, f);
        }
        Node::Attach { primary, inputs } => {
            visit_source_groups(primary, f);
            for input in inputs {
                f(&input.source);
            }
        }
    }
}

fn map_sources(node: &mut Node, f: &mut impl FnMut(&mut TopicSource) -> Result<()>) -> Result<()> {
    match node {
        Node::Cursor => {}
        Node::Select {
            source,
            at,
            required,
            ..
        } => {
            for source in &mut source.0 {
                f(source)?;
            }
            map_sources(at, f)?;
            for input in required {
                for source in &mut input.source.0 {
                    f(source)?;
                }
            }
        }
        Node::Time(node) | Node::Min(node) => map_sources(node, f)?,
        Node::Available(nodes) => {
            for node in nodes {
                map_sources(node, f)?;
            }
        }
        Node::OrElse(a, b) => {
            map_sources(a, f)?;
            map_sources(b, f)?;
        }
        Node::Attach { primary, inputs } => {
            map_sources(primary, f)?;
            for input in inputs {
                for source in &mut input.source.0 {
                    f(source)?;
                }
            }
        }
    }
    Ok(())
}
