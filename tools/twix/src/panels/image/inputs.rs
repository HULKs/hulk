//! Overlay declarations and typed decoding. Temporal selection belongs to the
//! shared query-plan evaluator, not to the renderer.
use color_eyre::{Report, eyre::bail};
use ros_z::{Message, TypeInfo, time::Time};
use ros_z_debug::{
    SampleRecord, TargetIdentity, TopicReference,
    query::{
        TemporalMatch, TimeEncoding, TimeProjection, decode_record,
        plan::{InputSpec, Sample, Selection, State, TopicSource},
    },
};
use std::{borrow::Cow, sync::Arc};

/// Borrow for comparisons; own only the small identity when caching a texture.
#[derive(PartialEq, Eq)]
pub(super) struct SampleIdentity<'a> {
    topic: Cow<'a, str>,
    type_info: Cow<'a, TypeInfo>,
    publication: Option<([u8; 16], i64)>,
    id: u64,
    pub time: Time,
}

impl<'a> From<&'a Sample> for SampleIdentity<'a> {
    fn from(sample: &'a Sample) -> Self {
        Self {
            topic: Cow::Borrowed(&sample.topic.name),
            type_info: Cow::Borrowed(&sample.topic.type_info),
            publication: sample.record.publication,
            id: sample.record.id,
            time: sample.record.time,
        }
    }
}

impl SampleIdentity<'_> {
    pub fn type_name(&self) -> &str {
        &self.type_info.name
    }

    pub fn into_owned(self) -> SampleIdentity<'static> {
        SampleIdentity {
            topic: Cow::Owned(self.topic.into_owned()),
            type_info: Cow::Owned(self.type_info.into_owned()),
            publication: self.publication,
            id: self.id,
            time: self.time,
        }
    }
}

pub(super) struct OverlayInput {
    pub source: TopicSource,
    pub selection: Selection,
    pub prefer_frame: bool,
}

impl OverlayInput {
    pub fn new<T: Message>(topic: &str, selection: Selection) -> Self {
        let mut source = TopicSource::new(
            topic,
            TimeProjection::field(".time", TimeEncoding::RosZTime).expect("overlay timestamp"),
        );
        source.expected_type = Some(T::type_info());
        Self {
            source,
            selection,
            prefer_frame: false,
        }
    }

    pub fn prefer_frame(mut self) -> Self {
        self.prefer_frame = true;
        self
    }

    pub fn into_spec(mut self, namespace: &str) -> Result<InputSpec, Report> {
        self.source.topic =
            TopicReference::new(&self.source.topic)?.resolve(&TargetIdentity::new(namespace)?)?;
        Ok(InputSpec {
            source: self.source.into(),
            selection: self.selection,
        })
    }
}

#[derive(Default)]
pub(super) struct OverlayInputs {
    states: Vec<State<Vec<Sample>>>,
    errors: Vec<String>,
}

impl OverlayInputs {
    pub fn update(&mut self, inputs: &[State<Vec<Sample>>]) -> bool {
        let same = self.states.len() == inputs.len()
            && self.states.iter().zip(inputs).all(|(a, b)| match (a, b) {
                (State::Pending, State::Pending) => true,
                (State::Failed(a), State::Failed(b)) => a == b,
                (State::Ready(a), State::Ready(b)) => a
                    .iter()
                    .map(SampleIdentity::from)
                    .eq(b.iter().map(SampleIdentity::from)),
                _ => false,
            });
        if !same {
            inputs.clone_into(&mut self.states);
        }
        !same
    }

    pub fn clear(&mut self) {
        self.states.clear();
        self.errors.clear();
    }

    pub fn begin_prepare(&mut self) {
        self.errors.clear();
    }

    pub fn errors(&self) -> impl Iterator<Item = &str> {
        self.errors
            .iter()
            .map(String::as_str)
            .chain(self.states.iter().filter_map(|state| match state {
                State::Failed(error) => Some(error.as_str()),
                _ => None,
            }))
    }

    pub fn decode<T: Message>(
        &mut self,
        slot: usize,
        time: Time,
    ) -> Result<Option<TemporalMatch<Arc<SampleRecord<T>>>>, Report> {
        let result = (|| {
            let Some(state) = self.states.get(slot) else {
                return Ok(None);
            };
            let records = match state {
                State::Pending => return Ok(None),
                State::Failed(error) => bail!(error.clone()),
                State::Ready(records) => records,
            };
            Ok(match records.as_slice() {
                [] => None,
                [one] => {
                    let sample = decode_record(&one.record, &one.topic)?;
                    Some(if one.record.time == time {
                        TemporalMatch::Exact(sample)
                    } else {
                        TemporalMatch::Before(sample)
                    })
                }
                [before, after] => Some(TemporalMatch::Bracket {
                    before: decode_record(&before.record, &before.topic)?,
                    after: decode_record(&after.record, &after.topic)?,
                }),
                _ => bail!("overlay expected a single match or bracket"),
            })
        })();
        if let Err(error) = &result {
            self.errors.push(format!("{error:#}"));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::{super::tests::raw_sample, *};

    #[test]
    fn optional_decode_errors_remain_visible_and_identity_changes_invalidate_inputs() {
        let sample = raw_sample("/input", 1, 10, 20, &7u64);
        let mut inputs = OverlayInputs::default();
        assert!(inputs.update(&[State::Ready(vec![sample.clone()])]));
        assert!(!inputs.update(&[State::Ready(vec![sample.clone()])]));
        assert!(
            inputs
                .decode::<u32>(0, Time::from_nanos(10))
                .ok()
                .flatten()
                .is_none()
        );
        assert!(
            inputs
                .errors()
                .any(|error| error.contains("schema does not match"))
        );
        assert_eq!(
            inputs
                .decode::<u64>(0, Time::from_nanos(10))
                .unwrap()
                .unwrap()
                .last()
                .value,
            7
        );
        let mut changed = sample.clone();
        Arc::make_mut(&mut changed.record).publication = Some(([2; 16], 1));
        assert!(inputs.update(&[State::Ready(vec![changed])]));
        assert!(inputs.update(&[State::Failed("unavailable".into())]));
        assert!(inputs.errors().any(|error| error == "unavailable"));
        inputs.clear();
        assert!(inputs.errors().next().is_none());
    }
}
