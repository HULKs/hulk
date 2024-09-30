use std::{
    iter::once,
    net::{IpAddr, Ipv4Addr},
    ops::RangeInclusive,
};

use eframe::{
    egui::{
        text::{CCursor, CCursorRange},
        Area, Context, Frame, Id, Key, Modifiers, Order, Response, ScrollArea, TextEdit, Ui,
        Widget, WidgetText,
    },
    epaint::Color32,
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::chain;
use log::error;

use crate::nao::Nao;

#[derive(Default, Clone, Copy)]
struct CompletionState {
    selected_item: Option<i64>,
}

impl CompletionState {
    fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data_mut(|data| data.get_temp(id))
    }

    fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|data| data.insert_temp(id, self));
    }
}

pub struct CompletionEntry {
    text: String,
    highlight: bool,
}

impl CompletionEntry {
    pub fn new(text: String, highlight: bool) -> Self {
        Self { text, highlight }
    }
}

impl From<String> for CompletionEntry {
    fn from(value: String) -> Self {
        Self::new(value, false)
    }
}

pub struct CompletionEdit<'key> {
    hint_text: WidgetText,
    key: &'key mut String,
    completion_items: Vec<CompletionEntry>,
}

impl<'key> CompletionEdit<'key> {
    pub fn new(
        key: &'key mut String,
        completion_items: Vec<CompletionEntry>,
        hint_text: impl Into<WidgetText>,
    ) -> Self {
        Self {
            hint_text: hint_text.into(),
            key,
            completion_items,
        }
    }

    pub fn addresses(
        key: &'key mut String,
        numbers: RangeInclusive<u8>,
        highlighted_ips: &[IpAddr],
    ) -> Self {
        let completion_items: Vec<_> = chain!(
            once(CompletionEntry::new("localhost".to_string(), true)),
            numbers.clone().map(|number| {
                let ip = IpAddr::V4(Ipv4Addr::new(10, 1, 24, number));
                CompletionEntry::new(ip.to_string(), highlighted_ips.contains(&ip))
            }),
            numbers.map(|number| CompletionEntry::new(format!("10.0.24.{number}"), false))
        )
        .collect();

        Self {
            hint_text: "Address".into(),
            key,
            completion_items,
        }
    }

    pub fn readable_paths(key: &'key mut String, nao: &Nao) -> Self {
        let completion_items = match &*nao.latest_paths() {
            Some(Ok(paths)) => paths
                .iter()
                .filter_map(|(path, entry)| {
                    if entry.is_readable {
                        Some(CompletionEntry::from(path.clone()))
                    } else {
                        None
                    }
                })
                .collect(),
            Some(Err(error)) => {
                error!("{error}");
                Vec::new()
            }
            None => Vec::new(),
        };

        Self {
            hint_text: "Path".into(),
            key,
            completion_items,
        }
    }

    pub fn writable_paths(key: &'key mut String, nao: &Nao) -> Self {
        let completion_items = match &*nao.latest_paths() {
            Some(Ok(paths)) => paths
                .iter()
                .filter_map(|(path, entry)| {
                    if entry.is_writable {
                        Some(CompletionEntry::from(path.clone()))
                    } else {
                        None
                    }
                })
                .collect(),
            Some(Err(error)) => {
                error!("{error}");
                Vec::new()
            }
            None => Vec::new(),
        };

        Self {
            hint_text: "Path".into(),
            key,
            completion_items,
        }
    }

    pub fn select_all(text: &str, ui: &mut Ui, id: Id) {
        if let Some(mut state) = TextEdit::load_state(ui.ctx(), id) {
            state.cursor.set_char_range(Some(CCursorRange::two(
                CCursor::new(0),
                CCursor::new(text.chars().count()),
            )));
            TextEdit::store_state(ui.ctx(), id, state);
        }
    }
}

impl Widget for CompletionEdit<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut response = TextEdit::singleline(self.key)
            .hint_text(self.hint_text)
            .lock_focus(true)
            .ui(ui);

        let popup_id = response.id.with("completion_popup");
        let is_open = ui.memory(|memory| memory.is_popup_open(popup_id));
        let mut state = CompletionState::load(ui.ctx(), popup_id).unwrap_or_default();
        let matcher = SkimMatcherV2::default();
        let mut completion_text_items: Vec<_> = self
            .completion_items
            .into_iter()
            .filter_map(|item| {
                matcher
                    .fuzzy_match(&item.text, self.key)
                    .map(|score| (score, item))
            })
            .collect();
        completion_text_items.sort_by_key(|(score, _)| -*score);

        if response.has_focus() != is_open {
            ui.memory_mut(|memory| memory.toggle_popup(popup_id));
            if response.gained_focus() {
                CompletionEdit::select_all(self.key, ui, response.id);
            }
        }
        if response.changed() {
            state.selected_item = if completion_text_items.is_empty() {
                None
            } else {
                Some(0)
            };
        }
        response.changed = false;

        if is_open {
            if !completion_text_items.is_empty() {
                ui.input_mut(|input| {
                    if input.consume_key(Modifiers::NONE, Key::ArrowDown)
                        || input.consume_key(Modifiers::NONE, Key::Tab)
                    {
                        state.selected_item = Some(
                            (state.selected_item.unwrap_or(-1) + 1)
                                % (completion_text_items.len() as i64),
                        );
                    } else if input.consume_key(Modifiers::NONE, Key::ArrowUp)
                        || input.consume_key(eframe::egui::Modifiers::SHIFT, Key::Tab)
                    {
                        state.selected_item = Some(
                            (state
                                .selected_item
                                .unwrap_or(completion_text_items.len() as i64)
                                - 1)
                            .rem_euclid(completion_text_items.len() as i64),
                        );
                    }
                });
            } else {
                state.selected_item = None;
            }

            if ui.input(|input| input.key_pressed(Key::Enter)) {
                if state.selected_item.is_some() {
                    *self.key = completion_text_items
                        .get(state.selected_item.unwrap() as usize)
                        .unwrap()
                        .1
                        .text
                        .to_string();
                    state.selected_item = None;
                }
                response.mark_changed();
            }
            let area = Area::new(popup_id)
                .order(Order::Foreground)
                .current_pos(response.rect.left_bottom())
                .show(ui.ctx(), |ui| {
                    Frame::popup(ui.style()).show(ui, |ui| {
                        ScrollArea::vertical().show(ui, |ui| {
                            for (i, completion_item) in
                                completion_text_items.into_iter().enumerate()
                            {
                                let completion_entry = &completion_item.1;
                                let is_selected = Some(i as i64) == state.selected_item;

                                let mut text = WidgetText::from(completion_entry.text.clone());
                                if completion_entry.highlight {
                                    text = text.color(Color32::GREEN);
                                }

                                let label = ui.selectable_label(is_selected, text);

                                if is_selected {
                                    label.scroll_to_me(None);
                                }
                                if label.is_pointer_button_down_on() {
                                    self.key.clone_from(&completion_item.1.text);
                                    response.mark_changed();
                                    ui.memory_mut(|memory| memory.close_popup());
                                }
                            }
                        });
                    })
                });
            if ui.input(|input| input.key_pressed(Key::Escape))
                || response.union(area.response).clicked_elsewhere()
            {
                ui.memory_mut(|memory| memory.close_popup());
            }
        } else {
            state.selected_item = None;
        }
        state.store(ui.ctx(), popup_id);
        response
    }
}
