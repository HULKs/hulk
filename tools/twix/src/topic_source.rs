//! Shared topic and field selection controls for dynamic panels.

use eframe::egui::{
    Ui,
    text::{CCursor, CCursorRange},
    text_edit::TextEditState,
};
use hulk_widgets::CompletionEdit;
use ros_z::{
    dynamic::{
        DynamicPayload, DynamicValue, EnumPayloadDef, SelectedType, SelectedValue,
        SequenceLengthDef, TypeDef, TypeDefinition, ValuePath, ValuePathStep,
    },
    entity::EndpointKind,
};

use crate::{backend::RobotBackend, graph::TopicCompletionQuery};

pub struct TopicSourceEditor {
    editor: String,
    topic: String,
    field_path: String,
    focus_requested: bool,
}

impl TopicSourceEditor {
    pub fn new(topic: String, field_path: String) -> Self {
        Self {
            editor: source_path(&topic, &field_path),
            topic,
            field_path,
            focus_requested: false,
        }
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn field_path(&self) -> &str {
        &self.field_path
    }

    pub fn request_focus(&mut self) {
        self.focus_requested = true;
    }

    /// Returns true only when the topic changes. Field changes never reconnect.
    /// Call inside a scope with a stable, unique UI id for each source.
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        backend: &RobotBackend,
        sample: Option<&DynamicPayload>,
    ) -> bool {
        ui.label("Topic");
        let namespace = backend.namespace();
        let topics = {
            let graph = backend.graph().lock();
            TopicCompletionQuery::new(&namespace, &self.editor)
                .endpoint_kind(EndpointKind::Publisher)
                .complete(graph.publishers())
        };
        let completions = source_completions(&topics, &self.topic, sample, &self.editor);
        let response = ui
            .add(
                CompletionEdit::new(ui.id().with("topic"), &completions, &mut self.editor)
                    .select_all_on_focus(false)
                    .request_focus(std::mem::take(&mut self.focus_requested)),
            )
            .on_hover_text(concat!(
                "Topic followed by a field path, for example ",
                "detected_objects.inner[2].bounding_box.confidence. ",
                "Enter only the topic for the whole message. ",
                "Ctrl+Space opens completions. Press Enter to apply.",
            ));
        if !response.changed() {
            return false;
        }
        if let Some(sample) = sample
            && let Some(range) = array_template_range(&self.editor, &topics, &self.topic, sample)
        {
            // A template is an editing aid, not a subscription or value path.
            response.request_focus();
            if let Some(mut state) = TextEditState::load(ui.ctx(), response.id) {
                state.cursor.set_char_range(Some(CCursorRange::two(
                    CCursor::new(range.start),
                    CCursor::new(range.end),
                )));
                state.store(ui.ctx(), response.id);
            }
            return false;
        }
        self.commit(&topics)
    }

    fn commit(&mut self, topics: &[String]) -> bool {
        let (topic, field_path) = split_source(self.editor.trim(), topics, &self.topic);
        let topic_changed = topic != self.topic;
        self.topic = topic.to_owned();
        self.field_path = field_path.to_owned();
        topic_changed
    }
}

fn source_path(topic: &str, field_path: &str) -> String {
    if field_path.is_empty() {
        topic.to_owned()
    } else if field_path.starts_with('[') || field_path.starts_with("::") {
        format!("{topic}{field_path}")
    } else {
        format!("{topic}.{field_path}")
    }
}

fn strip_topic<'a>(input: &'a str, topic: &str) -> Option<&'a str> {
    let suffix = input.strip_prefix(topic)?;
    if let Some(field_path) = suffix.strip_prefix('.') {
        Some(field_path)
    } else if suffix.is_empty() || suffix.starts_with('[') || suffix.starts_with("::") {
        Some(suffix)
    } else {
        None
    }
}

fn split_source<'a>(input: &'a str, topics: &[String], current_topic: &str) -> (&'a str, &'a str) {
    // ROS-Z permits dots in topic names. Prefer the longest discovered topic,
    // also retaining the current topic when its publisher has disappeared.
    if let Some(source) = topics
        .iter()
        .map(String::as_str)
        .chain([current_topic])
        .filter(|topic| !topic.is_empty())
        .filter_map(|topic| strip_topic(input, topic).map(|field| (&input[..topic.len()], field)))
        .max_by_key(|(topic, _)| topic.len())
    {
        return source;
    }
    // Permit entering a complete source before discovering its publisher.
    for (index, character) in input.char_indices() {
        match character {
            '.' => return (&input[..index], &input[index + 1..]),
            '[' => return (&input[..index], &input[index..]),
            ':' if input[index..].starts_with("::") => return (&input[..index], &input[index..]),
            _ => {}
        }
    }
    (input, "")
}

fn source_completions(
    topics: &[String],
    current_topic: &str,
    sample: Option<&DynamicPayload>,
    input: &str,
) -> Vec<String> {
    let (topic, field_path) = split_source(input, topics, current_topic);
    let mut completions = topics.to_vec();
    if topic == current_topic
        && !topic.is_empty()
        && let Some(sample) = sample
    {
        for path in field_completions(sample, field_path) {
            let source = source_path(topic, &path);
            if !completions.contains(&source) {
                completions.push(source);
            }
        }
    }
    completions
}

fn array_template_range(
    input: &str,
    topics: &[String],
    current_topic: &str,
    sample: &DynamicPayload,
) -> Option<std::ops::Range<usize>> {
    let (topic, field_path) = split_source(input, topics, current_topic);
    if topic != current_topic {
        return None;
    }
    for (index, _) in field_path.match_indices("[...]") {
        let Ok(path) = field_path[..index].parse::<ValuePath>() else {
            continue;
        };
        if path.resolve_type(&sample.schema).is_ok_and(is_sequence) {
            let byte_start = input.len() - field_path.len() + index + 1;
            let start = input[..byte_start].chars().count();
            return Some(start..start + 3);
        }
    }
    None
}

fn is_sequence(mut shape: SelectedType<'_>) -> bool {
    while let SelectedType::Value(TypeDef::Optional(inner)) = shape {
        shape = SelectedType::Value(inner);
    }
    matches!(shape, SelectedType::Value(TypeDef::Sequence { .. }))
}

// Complete one level at a time, so recursive schemas and large arrays cannot
// expand into an unbounded list. Any index can still be entered manually.
const MAX_INDEX_SUGGESTIONS: usize = 64;

fn field_completions(sample: &DynamicPayload, query: &str) -> Vec<String> {
    // Keep valid manual selections first, including unavailable sequence indices
    // and the empty root path. Enter must not silently choose another field.
    let mut suggestions: Vec<String> = query
        .parse::<ValuePath>()
        .ok()
        .filter(|path| path.resolve_type(&sample.schema).is_ok())
        .map(|path| path.to_string())
        .into_iter()
        .collect();
    // Find the deepest valid prefix. This also handles partial names, brackets,
    // variant separators, and quoted names without a second path parser.
    for end in query
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(query.len()))
        .rev()
    {
        let Ok(path) = query[..end].parse::<ValuePath>() else {
            continue;
        };
        let Ok(shape) = path.resolve_type(&sample.schema) else {
            continue;
        };
        let children = child_steps(shape, sample, &path);
        if children.is_empty() && !is_sequence(shape) && end != 0 {
            continue;
        }
        if is_sequence(shape) {
            suggestions.push(format!("{path}[...]"));
        }
        for step in children {
            let child_path = path.child(step);
            let child_text = child_path.to_string();
            if !suggestions.contains(&child_text) {
                suggestions.push(child_text);
            }
            if child_path
                .resolve_type(&sample.schema)
                .is_ok_and(is_sequence)
            {
                suggestions.push(format!("{child_path}[...]"));
            }
        }
        return suggestions;
    }
    suggestions
}

fn child_steps(
    mut shape: SelectedType<'_>,
    sample: &DynamicPayload,
    path: &ValuePath,
) -> Vec<ValuePathStep> {
    while let SelectedType::Value(TypeDef::Optional(inner)) = shape {
        shape = SelectedType::Value(inner);
    }
    match shape {
        SelectedType::Value(TypeDef::Named(name)) => match sample.schema.definitions.get(name) {
            Some(TypeDefinition::Struct(definition)) => definition
                .fields
                .iter()
                .map(|field| ValuePathStep::Field(field.name.clone()))
                .collect(),
            Some(TypeDefinition::Enum(definition)) => definition
                .variants
                .iter()
                .map(|variant| ValuePathStep::Variant(variant.name.clone()))
                .collect(),
            None => Vec::new(),
        },
        SelectedType::Value(TypeDef::Sequence { length, .. }) => {
            let len = match length {
                SequenceLengthDef::Fixed(len) => *len,
                SequenceLengthDef::Dynamic => sequence_len(path.select(sample).ok()).unwrap_or(0),
            };
            (0..len.min(MAX_INDEX_SUGGESTIONS))
                .map(ValuePathStep::Index)
                .collect()
        }
        SelectedType::EnumPayload(EnumPayloadDef::Struct(fields)) => fields
            .iter()
            .map(|field| ValuePathStep::Field(field.name.clone()))
            .collect(),
        SelectedType::EnumPayload(EnumPayloadDef::Tuple(fields)) => {
            (0..fields.len().min(MAX_INDEX_SUGGESTIONS))
                .map(ValuePathStep::Index)
                .collect()
        }
        _ => Vec::new(),
    }
}

fn sequence_len(mut selected: Option<SelectedValue<'_>>) -> Option<usize> {
    while let Some(SelectedValue::Value(DynamicValue::Optional(value))) = selected {
        selected = value.as_deref().map(SelectedValue::Value);
    }
    match selected? {
        SelectedValue::Value(DynamicValue::Sequence(values)) => Some(values.len()),
        SelectedValue::Value(DynamicValue::Bytes(values)) => Some(values.len()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ros_z::dynamic::{
        EnumDef, EnumVariantDef, FieldDef, PrimitiveTypeDef, SchemaBundle, StructDef,
        TypeDefinitions, TypeName,
    };
    use std::sync::Arc;

    fn sample() -> DynamicPayload {
        let root = TypeName::new("test::Root").unwrap();
        let state = TypeName::new("test::State").unwrap();
        DynamicPayload::default_for_schema(Arc::new(SchemaBundle {
            root: TypeDef::Named(root.clone()),
            definitions: TypeDefinitions::from([
                (
                    root.clone(),
                    TypeDefinition::Struct(StructDef {
                        fields: vec![
                            FieldDef::new(
                                "next",
                                TypeDef::Optional(Box::new(TypeDef::Named(root))),
                            ),
                            FieldDef::new(
                                "joints",
                                TypeDef::Sequence {
                                    element: Box::new(TypeDef::Primitive(PrimitiveTypeDef::F64)),
                                    length: SequenceLengthDef::Dynamic,
                                },
                            ),
                            FieldDef::new(
                                "fixed",
                                TypeDef::Sequence {
                                    element: Box::new(TypeDef::Primitive(PrimitiveTypeDef::U8)),
                                    length: SequenceLengthDef::Fixed(1000),
                                },
                            ),
                            FieldDef::new("state", TypeDef::Named(state.clone())),
                            FieldDef::new("with.dots", TypeDef::String),
                        ],
                    }),
                ),
                (
                    state,
                    TypeDefinition::Enum(EnumDef {
                        variants: vec![
                            EnumVariantDef::new("Idle", EnumPayloadDef::Unit),
                            EnumVariantDef::new(
                                "Walking",
                                EnumPayloadDef::Struct(vec![FieldDef::new(
                                    "speed",
                                    TypeDef::Primitive(PrimitiveTypeDef::F64),
                                )]),
                            ),
                        ],
                    }),
                ),
            ]),
        }))
        .unwrap()
    }

    #[test]
    fn completions_use_schema_for_absent_optional_and_inactive_variant() {
        let sample = sample();
        assert!(field_completions(&sample, "next.").contains(&"next.joints".to_owned()));
        assert!(field_completions(&sample, "state::").contains(&"state::Walking".to_owned()));
        assert!(
            field_completions(&sample, "state::Walking.")
                .contains(&"state::Walking.speed".to_owned())
        );
        assert!(field_completions(&sample, "with").contains(&"\"with.dots\"".to_owned()));
    }

    #[test]
    fn completions_preserve_root_and_unavailable_manual_indices() {
        let sample = sample();
        assert_eq!(field_completions(&sample, "")[0], "");
        assert_eq!(field_completions(&sample, "joints[100]")[0], "joints[100]");
        assert_eq!(field_completions(&sample, "fixed[900]")[0], "fixed[900]");
        assert!(!field_completions(&sample, "fixed[1000]").contains(&"fixed[1000]".to_owned()));
    }

    #[test]
    fn completion_expands_only_one_level_and_limits_index_suggestions() {
        let mut sample = sample();
        let root = field_completions(&sample, "");
        assert!(root.contains(&"next".to_owned()));
        assert!(!root.contains(&"next.next".to_owned()));
        assert_eq!(
            field_completions(&sample, "fixed[").len(),
            MAX_INDEX_SUGGESTIONS + 1
        );
        let DynamicValue::Struct(value) = &mut sample.value else {
            unreachable!();
        };
        value
            .set_dynamic(
                "joints",
                DynamicValue::Sequence(vec![DynamicValue::Float64(0.0); 2]),
            )
            .unwrap();
        assert_eq!(
            field_completions(&sample, "joints["),
            ["joints[...]", "joints[0]", "joints[1]"]
        );
    }

    #[test]
    fn inline_sources_split_at_the_topic_boundary() {
        let topics = vec![
            "detected_objects".to_owned(),
            "robot.v2/detected_objects".to_owned(),
        ];
        for (input, topic, field) in [
            ("detected_objects", "detected_objects", ""),
            (
                "detected_objects.inner[2].confidence",
                "detected_objects",
                "inner[2].confidence",
            ),
            (
                "detected_objects_other.inner",
                "detected_objects_other",
                "inner",
            ),
            (
                "robot.v2/detected_objects.inner",
                "robot.v2/detected_objects",
                "inner",
            ),
            (
                "/42/detected_objects.inner[2].area.min",
                "/42/detected_objects",
                "inner[2].area.min",
            ),
            ("~detected_objects.inner", "~detected_objects", "inner"),
            ("values[2]", "values", "[2]"),
            ("state::Walking.speed", "state", "::Walking.speed"),
        ] {
            assert_eq!(split_source(input, &topics, ""), (topic, field));
            assert_eq!(source_path(topic, field), input);
        }
        assert_eq!(
            split_source(
                "robot.v2/detected_objects.inner",
                &[],
                "robot.v2/detected_objects"
            ),
            ("robot.v2/detected_objects", "inner")
        );
    }

    #[test]
    fn inline_field_changes_preserve_the_subscription_topic() {
        let mut source = TopicSourceEditor::new("detected_objects".into(), "inner".into());
        assert_eq!(source.editor, "detected_objects.inner");
        source.editor = "detected_objects.inner[2].confidence".into();
        assert!(!source.commit(&[]));
        assert_eq!(source.topic(), "detected_objects");
        assert_eq!(source.field_path(), "inner[2].confidence");
        source.editor = "detected_objects".into();
        assert!(!source.commit(&[]));
        assert_eq!(source.field_path(), "");
        source.editor = "other_topic.inner[1].area".into();
        assert!(source.commit(&[]));
        assert_eq!(source.topic(), "other_topic");
        assert_eq!(source.field_path(), "inner[1].area");
    }

    fn detection_sample(objects: bool) -> DynamicPayload {
        use ros_z::{Message, dynamic::DynamicCdrCodec, message::WireEncoder, time::Time};
        use types::{
            object_detection::{Object, RobocupObjectLabel},
            time_wrapper::TimeWrapper,
        };
        fn decode<T: Message>(value: &T) -> DynamicPayload {
            let bytes = T::Codec::serialize(value).unwrap();
            DynamicCdrCodec::decode(&bytes, &Arc::new(T::schema())).unwrap()
        }
        let detections =
            vec![Object::<RobocupObjectLabel>::from([1.0, 2.0, 3.0, 4.0, 0.75, 0.0]); 3];
        if objects {
            decode(&TimeWrapper {
                time: Time::zero(),
                inner: detections,
            })
        } else {
            decode(&TimeWrapper {
                time: Time::zero(),
                inner: detections
                    .into_iter()
                    .map(|object| object.bounding_box)
                    .collect::<Vec<_>>(),
            })
        }
    }

    #[test]
    fn inline_completion_and_selection_reach_fields_inside_array_elements() {
        let topics = vec!["detected_objects".to_owned()];
        for objects in [false, true] {
            let sample = detection_sample(objects);
            let parent = if objects {
                "detected_objects.inner[2].bounding_box"
            } else {
                "detected_objects.inner[2]"
            };
            let completions = source_completions(
                &topics,
                "detected_objects",
                Some(&sample),
                &format!("{parent}."),
            );
            assert!(
                completions.contains(&format!("{parent}.confidence")),
                "{completions:?}"
            );
            assert!(
                completions.contains(&format!("{parent}.area")),
                "{completions:?}"
            );
            let input = format!("{parent}.confidence");
            let (_, path) = split_source(&input, &topics, "detected_objects");
            let selected = path.parse::<ValuePath>().unwrap().select(&sample).unwrap();
            assert_eq!(
                selected.to_json(Default::default()),
                serde_json::json!(0.75)
            );
            let area = format!("{parent}.area.");
            assert!(
                source_completions(&topics, "detected_objects", Some(&sample), &area)
                    .contains(&format!("{parent}.area.min"))
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn choosing_array_completion_keeps_editing_the_index_and_then_nested_fields() {
        use eframe::egui::{
            CentralPanel, Context, Event, FullOutput, Key, Modifiers, PointerButton, Pos2,
            RawInput, Rect, epaint::Shape, vec2,
        };

        fn frame(
            context: &Context,
            backend: &RobotBackend,
            sample: &DynamicPayload,
            source: &mut TopicSourceEditor,
            events: Vec<Event>,
        ) -> FullOutput {
            context.run_ui(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    CentralPanel::default().show(ui, |ui| {
                        ui.spacing_mut().text_edit_width = 700.0;
                        ui.horizontal(|ui| {
                            assert!(!source.ui(ui, backend, Some(sample)));
                        });
                    });
                },
            )
        }
        fn text_position(output: &FullOutput, label: &str) -> Pos2 {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    Shape::Text(text) if text.galley.text() == label => {
                        Some(text.pos + vec2(10.0, 5.0))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| {
                    panic!(
                        "missing UI text {label}: {:?}",
                        output
                            .shapes
                            .iter()
                            .filter_map(|shape| match &shape.shape {
                                Shape::Text(text) => Some(text.galley.text()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                    )
                })
        }
        fn click(pos: Pos2) -> Vec<Event> {
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::NONE,
                },
            ]
        }
        fn key(key: Key, modifiers: Modifiers) -> Event {
            Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }
        }

        let backend = RobotBackend::new(tokio::runtime::Handle::current(), None, "/".into())
            .await
            .unwrap();
        let context = Context::default();
        let sample = detection_sample(false);
        let mut source = TopicSourceEditor::new("detected_objects".into(), "in".into());
        source.request_focus();
        frame(&context, &backend, &sample, &mut source, vec![]);
        frame(
            &context,
            &backend,
            &sample,
            &mut source,
            vec![key(Key::ArrowDown, Modifiers::NONE)],
        );
        frame(
            &context,
            &backend,
            &sample,
            &mut source,
            vec![key(Key::Enter, Modifiers::NONE)],
        );
        assert_eq!(source.editor, "detected_objects.inner");
        assert_eq!(source.field_path(), "inner");
        frame(
            &context,
            &backend,
            &sample,
            &mut source,
            vec![Event::Text("[".into())],
        );
        assert_eq!(source.editor, "detected_objects.inner[");
        // Popups use their first visible frame to measure the suggestion list.
        frame(&context, &backend, &sample, &mut source, vec![]);
        let output = frame(&context, &backend, &sample, &mut source, vec![]);
        let position = text_position(&output, "detected_objects.inner[...]");
        frame(&context, &backend, &sample, &mut source, click(position));
        assert_eq!(source.editor, "detected_objects.inner[...]");
        assert_eq!(
            source.field_path(),
            "inner",
            "template must not become a committed path"
        );
        frame(
            &context,
            &backend,
            &sample,
            &mut source,
            vec![Event::Text("2".into())],
        );
        assert_eq!(source.editor, "detected_objects.inner[2]");
        frame(
            &context,
            &backend,
            &sample,
            &mut source,
            vec![
                key(Key::ArrowRight, Modifiers::NONE),
                Event::Text(".conf".into()),
            ],
        );
        frame(
            &context,
            &backend,
            &sample,
            &mut source,
            vec![key(Key::Enter, Modifiers::NONE)],
        );
        assert_eq!(source.topic(), "detected_objects");
        assert_eq!(source.field_path(), "inner[2].confidence");
        assert_eq!(
            source
                .field_path()
                .parse::<ValuePath>()
                .unwrap()
                .select(&sample)
                .unwrap()
                .to_json(Default::default()),
            serde_json::json!(0.75)
        );
    }

    #[test]
    fn inline_array_templates_work_for_empty_arrays_and_select_only_the_index() {
        let mut sample = detection_sample(false);
        let topics = vec!["detected_objects".into()];
        let input = "detected_objects.inner[...]";
        for empty in [false, true] {
            if empty {
                let DynamicValue::Struct(value) = &mut sample.value else {
                    unreachable!();
                };
                value
                    .set_dynamic("inner", DynamicValue::Sequence(vec![]))
                    .unwrap();
            }
            let completions = source_completions(
                &topics,
                "detected_objects",
                Some(&sample),
                "detected_objects.in",
            );
            assert!(completions.contains(&input.to_owned()));
            let range = array_template_range(input, &topics, "detected_objects", &sample).unwrap();
            assert_eq!(
                input
                    .chars()
                    .skip(range.start)
                    .take(range.len())
                    .collect::<String>(),
                "..."
            );
            // Element fields come from the schema even when the array is empty.
            assert!(
                source_completions(
                    &topics,
                    "detected_objects",
                    Some(&sample),
                    "detected_objects.inner[2]."
                )
                .contains(&"detected_objects.inner[2].confidence".to_owned())
            );
        }
        assert!(
            array_template_range(
                "detected_objects.inner[2]",
                &topics,
                "detected_objects",
                &sample
            )
            .is_none()
        );
        assert!(
            array_template_range(
                "other_topic.inner[...]",
                &topics,
                "detected_objects",
                &sample
            )
            .is_none()
        );
    }
}
