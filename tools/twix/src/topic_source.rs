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

    /// Topic and field path formatted as one source, as entered in the input.
    pub fn source(&self) -> String {
        source_path(&self.topic, &self.field_path)
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
