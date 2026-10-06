//! Display-only conversion scripts. Received samples stay unchanged, so
//! editing a script reprojects the whole retained history.

use std::rc::Rc;

use eframe::egui::{Button, Label, RichText, TextEdit, Ui};
use rhai::{AST, Dynamic, Engine, Scope, module_resolvers::DummyModuleResolver};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::warning;

/// Upper bound of script steps per sample, so a runaway loop cannot freeze
/// the UI.
const MAX_OPERATIONS: u64 = 50_000;
const PREVIEW_LENGTH: usize = 200;
const LABEL_LENGTH: usize = 24;

thread_local! {
    static ENGINE: Engine = engine();
}

fn engine() -> Engine {
    let mut engine = Engine::new();
    // Scripts come from shared layouts, so they may only compute values:
    // no modules, no output, and bounded time and memory.
    engine
        .set_module_resolver(DummyModuleResolver::new())
        .disable_symbol("eval")
        .on_print(|_| {})
        .on_debug(|_, _, _| {})
        .set_strict_variables(true)
        .set_max_operations(MAX_OPERATIONS)
        .set_max_call_levels(32)
        .set_max_string_size(1 << 20)
        .set_max_array_size(1 << 20)
        .set_max_map_size(1 << 16);
    engine
}

fn scope(value: Dynamic) -> Scope<'static> {
    let mut scope = Scope::new();
    scope.push("value", value);
    scope
}

struct Example {
    name: &'static str,
    source: &'static str,
}

const EXAMPLES: [Example; 10] = [
    Example {
        name: "Radians to degrees",
        source: "value.to_degrees()",
    },
    Example {
        name: "Degrees to radians",
        source: "value.to_radians()",
    },
    Example {
        name: "Invert sign",
        source: "-value",
    },
    Example {
        name: "Absolute value",
        source: "abs(value)",
    },
    Example {
        name: "Scale and offset",
        source: "value * 2.0 + 1.0",
    },
    Example {
        name: "Vector length",
        source: "hypot(value.x, value.y)",
    },
    Example {
        name: "Array length",
        source: "value.len()",
    },
    Example {
        name: "Enum variant index",
        source: "value.variant_index",
    },
    Example {
        name: "Above threshold (states)",
        source: "value > 0.5",
    },
    Example {
        name: "Fallback for absent optionals",
        source: "if value == () { 0.0 } else { value }",
    },
];

/// Result of a conversion: numbers draw lines, booleans and strings draw
/// states, and `()` skips the sample.
#[derive(Debug, PartialEq)]
pub(super) enum Output {
    Number(f64),
    Bool(bool),
    Text(String),
    Nothing,
}

/// A Rhai script with the selected field bound to `value`. While the source
/// does not compile, the last valid script stays applied, and layouts save
/// that script.
#[derive(Clone, Default)]
pub(super) struct Conversion {
    source: String,
    /// Source of `script`.
    applied: String,
    script: Option<Rc<AST>>,
    error: Option<String>,
}

impl Conversion {
    pub fn new(source: String) -> Self {
        let mut conversion = Self {
            source,
            ..Default::default()
        };
        conversion.compile();
        conversion
    }

    fn compile(&mut self) {
        if self.source.trim().is_empty() {
            self.applied.clear();
            self.script = None;
            self.error = None;
            return;
        }
        match ENGINE.with(|engine| engine.compile_with_scope(&scope(Dynamic::UNIT), &self.source)) {
            Ok(script) => {
                self.applied.clone_from(&self.source);
                self.script = Some(Rc::new(script));
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    pub fn is_identity(&self) -> bool {
        self.script.is_none()
    }

    /// The applied script. A new script compares unequal by pointer, so
    /// projections know when to start over.
    pub fn script(&self) -> Option<&Rc<AST>> {
        self.script.as_ref()
    }

    /// Short description for plot labels, or `None` for the identity.
    pub fn label(&self) -> Option<String> {
        self.script.as_ref()?;
        let source = self.applied.trim();
        Some(
            if source.contains('\n') || source.chars().count() > LABEL_LENGTH {
                "converted".to_owned()
            } else {
                source.to_owned()
            },
        )
    }

    /// Convert a parameter value. Thresholds are lines, so the script must
    /// return a number.
    pub fn apply_number(&self, value: f64) -> Result<f64, String> {
        let Some(script) = &self.script else {
            return Ok(value);
        };
        match run(script, Dynamic::from_float(value))? {
            Output::Number(value) => Ok(value),
            _ => Err("Threshold conversions must return a number.".to_owned()),
        }
    }

    /// Editor with examples and a preview of the latest input. `input`
    /// returns the latest selected value, or `None` without samples.
    pub fn ui(&mut self, ui: &mut Ui, input: impl FnOnce() -> Option<Result<Value, String>>) {
        ui.set_width(360.0);
        ui.horizontal(|ui| {
            ui.label("Conversion");
            ui.menu_button("Examples", |ui| {
                for example in &EXAMPLES {
                    if ui
                        .button(example.name)
                        .on_hover_text(RichText::new(example.source).monospace())
                        .clicked()
                    {
                        self.source = example.source.to_owned();
                        self.compile();
                    }
                }
            });
            if ui
                .add_enabled(!self.source.is_empty(), Button::new("Clear"))
                .clicked()
            {
                self.source.clear();
                self.compile();
            }
        });
        if ui
            .add(
                TextEdit::multiline(&mut self.source)
                    .code_editor()
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .hint_text("value"),
            )
            .changed()
        {
            self.compile();
        }
        if let Some(error) = &self.error {
            warning(ui, &format!("Not applied: {error}"));
        }
        match input() {
            None => {
                ui.weak("No sample yet.");
            }
            Some(Err(error)) => warning(ui, &error),
            Some(Ok(input)) => {
                ui.add(Label::new(format!("Latest: {}", preview(&input.to_string()))).wrap());
                let output = match &self.script {
                    Some(script) => run(script, json_to_dynamic(&input)),
                    None => Ok(output(json_to_dynamic(&input))
                        .unwrap_or_else(|_| Output::Text(input.to_string()))),
                };
                match output {
                    Ok(output) => {
                        ui.add(Label::new(format!("Result: {}", describe(&output))).wrap());
                    }
                    Err(error) => warning(ui, &error),
                }
            }
        }
        ui.add(
            Label::new(
                RichText::new(
                    "Rhai script with the selected field as `value`. Return a number to \
                     draw a line, a boolean or string to draw states, or () to skip a \
                     sample. Numbers are floats.",
                )
                .weak(),
            )
            .wrap(),
        );
    }
}

impl Serialize for Conversion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.applied.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Conversion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::new)
    }
}

/// Run a script with `value` bound to the input.
pub(super) fn run(script: &AST, value: Dynamic) -> Result<Output, String> {
    let result = ENGINE
        .with(|engine| engine.eval_ast_with_scope::<Dynamic>(&mut scope(value), script))
        .map_err(|error| error.to_string())?;
    output(result)
}

fn output(result: Dynamic) -> Result<Output, String> {
    if result.is_unit() {
        Ok(Output::Nothing)
    } else if let Ok(value) = result.as_float() {
        Ok(Output::Number(value))
    } else if let Ok(value) = result.as_int() {
        Ok(Output::Number(value as f64))
    } else if let Ok(value) = result.as_bool() {
        Ok(Output::Bool(value))
    } else if result.is_string() {
        Ok(Output::Text(result.to_string()))
    } else {
        Err(format!(
            "Conversion returned {}; return a number, boolean, string, or ().",
            result.type_name()
        ))
    }
}

/// Scripts see the JSON shape shown by the Text panel. All numbers become
/// floats, so integer fields do not divide as integers.
pub(super) fn json_to_dynamic(value: &Value) -> Dynamic {
    match value {
        Value::Null => Dynamic::UNIT,
        Value::Bool(value) => (*value).into(),
        Value::Number(value) => Dynamic::from_float(value.as_f64().unwrap_or(f64::NAN)),
        Value::String(value) => value.as_str().into(),
        Value::Array(values) => Dynamic::from_array(values.iter().map(json_to_dynamic).collect()),
        Value::Object(fields) => Dynamic::from_map(
            fields
                .iter()
                .map(|(name, value)| (name.as_str().into(), json_to_dynamic(value)))
                .collect(),
        ),
    }
}

fn describe(output: &Output) -> String {
    match output {
        Output::Number(value) => value.to_string(),
        Output::Bool(value) => format!("{value} (state)"),
        Output::Text(value) => format!("{} (state)", preview(value)),
        Output::Nothing => "() (skipped)".to_owned(),
    }
}

fn preview(text: &str) -> String {
    match text.char_indices().nth(PREVIEW_LENGTH) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn convert(source: &str, input: Value) -> Result<Output, String> {
        let conversion = Conversion::new(source.to_owned());
        assert_eq!(conversion.error, None);
        run(conversion.script().unwrap(), json_to_dynamic(&input))
    }

    #[test]
    fn empty_source_is_the_identity() {
        let conversion = Conversion::new("  \n".to_owned());

        assert!(conversion.is_identity());
        assert_eq!(conversion.label(), None);
        assert_eq!(conversion.apply_number(1.5), Ok(1.5));
    }

    #[test]
    fn examples_compile_and_run() {
        let inputs = [
            json!(std::f64::consts::PI),
            json!(180),
            json!(2),
            json!(-2),
            json!(3),
            json!({ "x": 3, "y": 4 }),
            json!([1, 2, 3]),
            json!({ "variant_index": 2, "variant_name": "Walking", "payload": null }),
            json!(0.7),
            Value::Null,
        ];
        let expected = [
            Output::Number(180.0),
            Output::Number(std::f64::consts::PI),
            Output::Number(-2.0),
            Output::Number(2.0),
            Output::Number(7.0),
            Output::Number(5.0),
            Output::Number(3.0),
            Output::Number(2.0),
            Output::Bool(true),
            Output::Number(0.0),
        ];

        for ((example, input), expected) in EXAMPLES.iter().zip(inputs).zip(expected) {
            assert_eq!(
                convert(example.source, input),
                Ok(expected),
                "{}",
                example.name
            );
        }
    }

    #[test]
    fn integers_divide_as_floats_and_compare_with_integer_literals() {
        assert_eq!(convert("value / 2", json!(3)), Ok(Output::Number(1.5)));
        assert_eq!(convert("value == 3", json!(3)), Ok(Output::Bool(true)));
    }

    #[test]
    fn statements_strings_and_unit_are_supported() {
        let source = "let speed = value.speed;\nif speed > 1.0 { \"fast\" } else { () }";

        assert_eq!(
            convert(source, json!({ "speed": 2 })),
            Ok(Output::Text("fast".to_owned()))
        );
        assert_eq!(convert(source, json!({ "speed": 0 })), Ok(Output::Nothing));
    }

    #[test]
    fn invalid_sources_keep_the_last_valid_script() {
        let mut conversion = Conversion::new("value * 2.0".to_owned());
        let script = conversion.script().cloned();
        conversion.source = "value *".to_owned();
        conversion.compile();

        assert!(conversion.error.is_some());
        assert!(Rc::ptr_eq(
            conversion.script().unwrap(),
            script.as_ref().unwrap()
        ));
        assert_eq!(conversion.apply_number(2.0), Ok(4.0));
    }

    #[test]
    fn unknown_variables_are_rejected_when_compiling() {
        let conversion = Conversion::new("valeu * 2.0".to_owned());

        assert!(conversion.is_identity());
        assert!(conversion.error.is_some());
    }

    #[test]
    fn scripts_cannot_loop_forever_or_import_modules() {
        let error = convert("loop {}", json!(1)).unwrap_err();
        assert!(error.contains("operations"), "{error}");

        let conversion = Conversion::new("import \"file\" as f; value".to_owned());
        let error = run(conversion.script().unwrap(), Dynamic::UNIT).unwrap_err();
        assert!(error.contains("file"), "{error}");
    }

    #[test]
    fn unsupported_results_are_errors() {
        let error = convert("[value]", json!(1)).unwrap_err();

        assert!(error.contains("array"), "{error}");
    }

    #[test]
    fn thresholds_require_numbers() {
        let conversion = Conversion::new("value > 1.0".to_owned());

        assert!(conversion.apply_number(2.0).is_err());
    }

    #[test]
    fn layouts_store_the_source() {
        let conversion: Conversion = serde_json::from_value(json!("value * 2.0")).unwrap();

        assert_eq!(serde_json::to_value(&conversion).unwrap(), "value * 2.0");
        assert_eq!(conversion.label().as_deref(), Some("value * 2.0"));
    }

    #[test]
    fn layouts_store_the_applied_script_while_editing() {
        let mut conversion = Conversion::new("value * 2.0".to_owned());
        conversion.source = "value *".to_owned();
        conversion.compile();

        assert_eq!(serde_json::to_value(&conversion).unwrap(), "value * 2.0");
        assert_eq!(conversion.label().as_deref(), Some("value * 2.0"));
    }
}
