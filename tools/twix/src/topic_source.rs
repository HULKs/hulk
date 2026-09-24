//! Shared topic and field selection controls for dynamic panels.

use eframe::egui::Ui;
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
    topic_editor: String,
    topic: String,
    field_editor: String,
    field_path: String,
}

impl TopicSourceEditor {
    pub fn new(topic: String, field_path: String) -> Self {
        Self {
            topic_editor: topic.clone(),
            topic,
            field_editor: field_path.clone(),
            field_path,
        }
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn field_path(&self) -> &str {
        &self.field_path
    }

    /// Returns true only when the topic changes. Field changes never reconnect.
    /// Call inside a scope with a stable, unique UI id for each source.
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        backend: &RobotBackend,
        sample: Option<&DynamicPayload>,
    ) -> bool {
        let mut topic_changed = false;
        ui.horizontal(|ui| {
            ui.label("Topic");
            let namespace = backend.namespace();
            let completions = {
                let graph = backend.graph().lock();
                TopicCompletionQuery::new(&namespace, &self.topic_editor)
                    .endpoint_kind(EndpointKind::Publisher)
                    .complete(graph.publishers())
            };
            let response = ui.add(CompletionEdit::new(
                ui.id().with("topic"),
                &completions,
                &mut self.topic_editor,
            ));
            if response.changed() {
                let topic = self.topic_editor.trim();
                if topic != self.topic {
                    self.topic = topic.to_owned();
                    topic_changed = true;
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Field");
            let completions = sample
                .filter(|_| !topic_changed)
                .map(|sample| field_completions(sample, &self.field_editor))
                .unwrap_or_default();
            let response =
                CompletionEdit::new(ui.id().with("field"), &completions, &mut self.field_editor)
                    .ui(ui, |ui, highlighted, path| {
                        ui.selectable_label(
                            highlighted,
                            if path.is_empty() {
                                "Whole message"
                            } else {
                                path
                            },
                        )
                    })
                    .on_hover_text(concat!(
                        "Empty selects the whole message. Examples: pose.x, joints[3].position, ",
                        "state::Walking.speed. Press Enter to apply.",
                    ));
            if response.changed() {
                self.field_path = self.field_editor.trim().to_owned();
            }
        });
        if !self.field_path.is_empty() && ui.small_button("Whole message").clicked() {
            self.field_editor.clear();
            self.field_path.clear();
        }
        topic_changed
    }
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
        if children.is_empty() && end != 0 {
            continue;
        }
        for step in children {
            let child = path.child(step).to_string();
            if !suggestions.contains(&child) {
                suggestions.push(child);
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
            MAX_INDEX_SUGGESTIONS
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
            ["joints[0]", "joints[1]"]
        );
    }
}
