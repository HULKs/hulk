use std::{fs, path::PathBuf};

use eframe::egui::{
    self, Align, Button, Context, Id, Key, Modal, Rect, RichText, TextEdit, Ui, Window, vec2,
};
use egui_material_icons::icons;
use egui_tiles::TileId;
use hulk_widgets::SearchableSelector;
use serde_json::Value;

use super::tree::LayoutRequest;
use crate::{PanelKind, presets};

struct SaveDialog {
    name: String,
    saved: Value,
    focus: bool,
    overwrite: bool,
    error: Option<String>,
}

enum Source {
    Blank,
    Panel(PanelKind),
    Provided(&'static str),
    User(PathBuf),
}

#[derive(Default)]
pub(super) struct PresetUi {
    user_presets: Option<Result<Vec<PathBuf>, String>>,
    save_dialog: Option<SaveDialog>,
    delete: Option<PathBuf>,
    delete_error: Option<String>,
    pub error: Option<String>,
}

impl PresetUi {
    pub fn save(&mut self, name: String, saved: Value) {
        self.save_dialog = Some(SaveDialog {
            name,
            saved,
            focus: true,
            overwrite: false,
            error: None,
        });
    }

    pub fn picker(
        &mut self,
        ui: &mut Ui,
        id: Id,
        tabs: TileId,
        root: bool,
        reset: bool,
    ) -> Option<LayoutRequest> {
        ui.strong("Add panel or preset");
        let mut choices = Vec::new();
        if root {
            choices.push(("Blank workspace".into(), Source::Blank));
        }
        choices.extend(
            PanelKind::registered()
                .iter()
                .map(|&kind| (kind.display_name().to_owned(), Source::Panel(kind))),
        );
        choices.extend(
            presets::PROVIDED
                .iter()
                .map(|&(title, saved)| (title.to_owned(), Source::Provided(saved))),
        );
        if reset {
            self.user_presets = None;
        }
        let user_presets = self.user_presets.get_or_insert_with(|| {
            presets::directory()
                .and_then(|directory| presets::list(&directory))
                .map_err(|error| format!("{error:#}"))
        });
        match user_presets {
            Ok(paths) => choices.extend(paths.iter().map(|path| {
                (
                    path.file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    Source::User(path.clone()),
                )
            })),
            Err(error) => {
                ui.colored_label(ui.visuals().error_fg_color, error.as_str());
            }
        }
        let selected = show_choices(ui, id, &choices, reset, &mut self.delete)?;
        ui.close();
        let (title, source) = choices.swap_remove(selected);
        match source {
            Source::Blank => Some(LayoutRequest::Blank(tabs)),
            Source::Panel(panel) => Some(LayoutRequest::Add { tabs, panel }),
            Source::Provided(saved) => Some(LayoutRequest::Import {
                tabs,
                title,
                saved: saved.into(),
            }),
            Source::User(path) => match fs::read_to_string(&path) {
                Ok(saved) => Some(LayoutRequest::Import { tabs, title, saved }),
                Err(error) => {
                    self.error = Some(format!("{}: {error}", path.display()));
                    None
                }
            },
        }
    }

    pub fn dialog_open(&self) -> bool {
        self.save_dialog.is_some() || self.delete.is_some()
    }

    pub fn dialogs(&mut self, context: &Context) {
        if let Some(mut dialog) = self.save_dialog.take() {
            let response = Modal::new(Id::new("save-preset")).show(context, |ui| {
                ui.set_width(320.0);
                ui.heading(format!("{}  Save preset", icons::ICON_SAVE.codepoint));
                ui.add_space(8.0);
                ui.label("Name");
                let editor = TextEdit::singleline(&mut dialog.name)
                    .id(Id::new("preset-name"))
                    .desired_width(f32::INFINITY)
                    .show(ui);
                let refresh_overwrite = dialog.focus || editor.response.changed();
                if std::mem::take(&mut dialog.focus) {
                    select_name(ui.ctx(), editor, &dialog.name);
                }
                let path = presets::directory()
                    .and_then(|directory| presets::path(&directory, &dialog.name));
                if refresh_overwrite {
                    dialog.overwrite = path.as_ref().is_ok_and(|path| path.exists());
                }
                let label = if dialog.overwrite {
                    "Overwrite"
                } else {
                    "Save"
                };
                if let Err(error) = &path {
                    ui.weak(error.to_string());
                } else if label == "Overwrite" {
                    ui.weak("Replaces the saved preset. Open groups are unaffected.");
                }
                if let Some(error) = &dialog.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.add_space(12.0);
                let save = ui
                    .with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        let save = ui
                            .add_enabled(
                                path.is_ok(),
                                Button::new(format!("{}  {label}", icons::ICON_SAVE.codepoint)),
                            )
                            .clicked();
                        if ui.button("Cancel").clicked() {
                            ui.close();
                        }
                        save
                    })
                    .inner
                    || (path.is_ok()
                        && ui.input_mut(|input| {
                            input.consume_key(egui::Modifiers::NONE, Key::Enter)
                        }));
                save.then_some(path).and_then(Result::ok)
            });
            if let Some(path) = response.inner.as_ref() {
                let result = serde_json::to_string_pretty(&dialog.saved)
                    .map_err(Into::into)
                    .and_then(|saved| presets::save(path, &saved));
                if let Err(error) = result {
                    dialog.error = Some(format!("{error:#}"));
                    self.save_dialog = Some(dialog);
                }
            } else if !response.should_close() {
                self.save_dialog = Some(dialog);
            }
        }
        if let Some(path) = self.delete.take() {
            let response = Modal::new(Id::new("delete-preset")).show(context, |ui| {
                ui.set_width(320.0);
                ui.heading(format!("{}  Delete preset?", icons::ICON_DELETE.codepoint));
                ui.add_space(8.0);
                ui.strong(format!(
                    "“{}”",
                    path.file_stem().unwrap_or_default().to_string_lossy()
                ));
                ui.label("This removes the saved preset.\nOpen groups are unaffected.");
                if let Some(error) = &self.delete_error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let delete = ui
                        .button(
                            RichText::new(format!("{}  Delete", icons::ICON_DELETE.codepoint))
                                .color(ui.visuals().error_fg_color),
                        )
                        .clicked();
                    if ui.button("Cancel").clicked() {
                        ui.close();
                    }
                    delete
                })
                .inner
            });
            if response.inner {
                match fs::remove_file(&path) {
                    Ok(()) => self.delete_error = None,
                    Err(error) => {
                        self.delete_error = Some(format!("Could not delete preset: {error}"));
                        self.delete = Some(path);
                    }
                }
            } else if !response.should_close() {
                self.delete = Some(path);
            } else {
                self.delete_error = None;
            }
        }
        if let Some(error) = &self.error {
            let mut open = true;
            Window::new("Layout error")
                .open(&mut open)
                .show(context, |ui| {
                    ui.label(error);
                });
            if !open {
                self.error = None;
            }
        }
    }
}

pub(super) fn select_name(
    context: &Context,
    mut editor: egui::text_edit::TextEditOutput,
    name: &str,
) {
    editor.response.request_focus();
    editor
        .state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(0),
            egui::text::CCursor::new(name.chars().count()),
        )));
    editor.state.store(context, editor.response.id);
}

fn show_choices(
    ui: &mut Ui,
    id: Id,
    choices: &[(String, Source)],
    reset: bool,
    delete: &mut Option<PathBuf>,
) -> Option<usize> {
    SearchableSelector::new(id, choices)
        .reset_on_show(reset)
        .show(
            ui,
            |choice| &choice.0,
            |ui, highlighted, (title, source)| {
                let (label, icon) = match source {
                    Source::Blank => ("", icons::ICON_DASHBOARD_2.codepoint),
                    Source::Panel(kind) => ("Panel", kind.icon()),
                    Source::Provided(_) => ("Provided", icons::ICON_DASHBOARD_2.codepoint),
                    Source::User(_) => ("Yours", icons::ICON_DASHBOARD_2.codepoint),
                };
                let (_, row) = ui.allocate_space(vec2(ui.available_width(), 28.0));
                let action = Rect::from_min_max(egui::pos2(row.right() - 28.0, row.top()), row.max);
                let content = Rect::from_min_max(
                    row.min,
                    egui::pos2(action.left() - ui.spacing().item_spacing.x, row.bottom()),
                );
                if let Source::User(path) = source {
                    let button = ui
                        .put(
                            action,
                            Button::new(icons::ICON_DELETE.codepoint).frame(false),
                        )
                        .on_hover_text("Delete preset");
                    button.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Delete preset")
                    });
                    if button.clicked() {
                        *delete = Some(path.clone());
                        ui.close();
                    }
                }
                ui.put(
                    content,
                    Button::selectable(highlighted, format!("{icon}  {title}"))
                        .right_text(RichText::new(label).weak())
                        .truncate(),
                )
            },
        )
}

#[cfg(test)]
mod tests {
    use super::super::tests::{click, context, key, painted_text, text_center};
    use super::*;

    fn frame(
        context: &Context,
        events: Vec<egui::Event>,
        show: impl FnMut(&mut Ui),
    ) -> egui::FullOutput {
        context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Default::default(), vec2(480.0, 400.0))),
                events,
                ..Default::default()
            },
            show,
        )
    }

    #[test]
    fn save_dialog_focuses_selects_and_reopens() {
        let context = context();
        let mut ui = PresetUi::default();
        for _ in 0..2 {
            ui.save("Old name".into(), Value::Null);
            frame(&context, vec![], |_| ui.dialogs(&context));
            assert!(context.memory(|memory| memory.has_focus(Id::new("preset-name"))));
            frame(&context, vec![egui::Event::Text("New name".into())], |_| {
                ui.dialogs(&context)
            });
            assert_eq!(ui.save_dialog.as_ref().unwrap().name, "New name");
            frame(&context, vec![egui::Event::Text("!".into())], |_| {
                ui.dialogs(&context)
            });
            assert_eq!(ui.save_dialog.as_ref().unwrap().name, "New name!");
            frame(&context, vec![key(Key::Escape)], |_| ui.dialogs(&context));
            assert!(ui.save_dialog.is_none());
            frame(&context, vec![], |_| {});
        }
    }

    #[test]
    fn picker_actions_align_and_delete_without_selecting() {
        let context = context();
        let choices = vec![
            ("Bundled".into(), Source::Provided("")),
            ("Short".into(), Source::User("short.json".into())),
            (
                "Longer preset name".into(),
                Source::User("long.json".into()),
            ),
        ];
        let mut delete = None;
        let id = Id::new("picker-test");
        frame(&context, vec![], |ui| {
            assert_eq!(show_choices(ui, id, &choices, true, &mut delete), None);
        });
        let output = frame(&context, vec![], |ui| {
            show_choices(ui, id, &choices, false, &mut delete);
        });
        let bins: Vec<_> = painted_text(&output)
            .filter(|text| text.galley.text() == icons::ICON_DELETE.codepoint)
            .map(|text| text.pos + text.galley.size() / 2.0)
            .collect();
        assert_eq!(bins.len(), 2);
        assert!((bins[0].x - bins[1].x).abs() < 0.5);
        for (name, bin) in ["Short", "Longer preset name"].into_iter().zip(&bins) {
            assert!((text_center(&output, name).y - bin.y).abs() < 0.5);
        }
        frame(
            &context,
            click(bins[0], egui::PointerButton::Primary),
            |ui| {
                assert_eq!(show_choices(ui, id, &choices, false, &mut delete), None);
            },
        );
        assert_eq!(delete, Some("short.json".into()));
    }

    #[test]
    fn delete_dialog_cancels_and_keeps_errors_for_retry() {
        let context = context();
        let path = std::env::temp_dir().join(format!("twix-delete-{}.json", uuid::Uuid::new_v4()));
        fs::write(&path, "{}").unwrap();
        let mut ui = PresetUi {
            delete: Some(path.clone()),
            ..Default::default()
        };
        frame(&context, vec![], |_| ui.dialogs(&context));
        frame(&context, vec![key(Key::Escape)], |_| ui.dialogs(&context));
        assert!(ui.delete.is_none());
        assert!(path.exists());
        ui.delete = Some(path.clone());
        fs::remove_file(&path).unwrap();
        for attempt in 0..2 {
            for _ in 0..2 {
                frame(&context, vec![], |_| ui.dialogs(&context));
            }
            let output = frame(&context, vec![], |_| ui.dialogs(&context));
            let action = text_center(&output, "  Delete");
            assert!((action.y - text_center(&output, "Cancel").y).abs() < 0.5);
            frame(
                &context,
                click(action, egui::PointerButton::Primary),
                |_| ui.dialogs(&context),
            );
            if attempt == 0 {
                assert_eq!(ui.delete.as_ref(), Some(&path));
                assert!(ui.delete_error.is_some());
                assert!(ui.error.is_none());
                fs::write(&path, "{}").unwrap();
            }
        }
        assert!(ui.delete.is_none());
        assert!(ui.delete_error.is_none());
        assert!(!path.exists());
    }
}
