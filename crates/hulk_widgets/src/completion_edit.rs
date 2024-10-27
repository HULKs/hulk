use std::{cmp::Reverse, fmt::Debug};

use egui::{
    popup_below_widget,
    text::{CCursor, CCursorRange},
    text_edit::TextEditOutput,
    util::cache::{ComputerMut, FrameCache},
    Context, Id, Key, PopupCloseBehavior, Response, ScrollArea, TextEdit, TextStyle, Ui, Widget,
};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Matcher, Utf32Str,
};

pub struct CompletionEdit<'a, T> {
    id: Id,
    suggestions: &'a [T],
    selected: &'a mut String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum UserState {
    #[default]
    Typing,
    Selecting {
        index: usize,
    },
}

impl UserState {
    fn handle_arrow(self, pressed_down: bool, pressed_up: bool, number_of_items: usize) -> Self {
        match (pressed_up, pressed_down, self) {
            (_, true, UserState::Typing) => UserState::Selecting { index: 0 },
            (true, _, UserState::Selecting { index: 0 }) => UserState::Typing,
            (true, _, UserState::Selecting { index }) => UserState::Selecting { index: index - 1 },
            (_, true, UserState::Selecting { index }) => UserState::Selecting {
                index: (index + 1).min(number_of_items - 1),
            },
            (_, _, state) => state,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct CompletionEditState {
    user_state: UserState,
}

#[derive(Default)]
struct MatcherSearch;
type CachedMatcherSearch = FrameCache<Vec<(usize, String)>, MatcherSearch>;

impl<'a, T: ToString> ComputerMut<(&String, &'a [T]), Vec<(usize, String)>> for MatcherSearch {
    fn compute(&mut self, (key, items): (&String, &'a [T])) -> Vec<(usize, String)> {
        let mut matcher = Matcher::default();
        let pattern = Pattern::parse(key.as_str(), CaseMatching::Smart, Normalization::Smart);

        if pattern.atoms.is_empty() {
            return items.iter().map(ToString::to_string).enumerate().collect();
        }

        let mut buffer = Vec::new();
        let mut items: Vec<_> = items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let string = item.to_string();
                pattern
                    .score(Utf32Str::new(string.as_str(), &mut buffer), &mut matcher)
                    .map(|score| (score, index, string))
            })
            .collect();

        items.sort_by_key(|(score, _, _)| Reverse(*score));

        items
            .into_iter()
            .map(|(_score, index, item)| (index, item))
            .collect()
    }
}

impl CompletionEditState {
    fn load(context: &Context, id: Id) -> Self {
        context
            .data(|reader| reader.get_temp(id))
            .unwrap_or_default()
    }

    fn save(self, context: &Context, id: Id) {
        context.data_mut(|writer| writer.insert_temp(id, self));
    }
}

impl<'a, T: ToString + Debug + std::hash::Hash> CompletionEdit<'a, T> {
    pub fn new(id_salt: impl Into<Id>, items: &'a [T], selected: &'a mut String) -> Self {
        Self {
            id: id_salt.into(),
            suggestions: items,
            selected,
        }
    }

    pub fn ui(
        mut self,
        ui: &mut Ui,
        show_value: impl Fn(&mut Ui, bool, &T) -> Response,
    ) -> Response {
        let mut state = CompletionEditState::load(ui.ctx(), self.id);
        let output = self.show(ui, &mut state, show_value);
        state.save(ui.ctx(), self.id);
        output
    }

    fn show(
        &mut self,
        ui: &mut Ui,
        state: &mut CompletionEditState,
        show_value: impl Fn(&mut Ui, bool, &T) -> Response,
    ) -> Response {
        let matching_items = ui.memory_mut(|writer| {
            let cache = writer.caches.cache::<CachedMatcherSearch>();
            cache.get((self.selected, self.suggestions))
        });

        let TextEditOutput {
            mut response,
            state: mut text_edit_state,
            ..
        } = match state.user_state {
            UserState::Typing => TextEdit::singleline(self.selected)
                .hint_text("Search")
                .show(ui),
            UserState::Selecting { index } => {
                let mut selected = matching_items
                    .get(index)
                    .map(|(_, value)| value.clone())
                    .unwrap_or_default();
                let output = TextEdit::singleline(&mut selected)
                    .hint_text("Search")
                    .show(ui);
                if output.response.changed() {
                    *self.selected = selected;
                }
                output
            }
        };
        response.changed = false;

        if !response.has_focus() {
            return response;
        }

        let pressed_down = ui.input_mut(|reader| reader.key_pressed(Key::ArrowDown));
        let pressed_up = ui.input_mut(|reader| reader.key_pressed(Key::ArrowUp));
        if pressed_down || pressed_up {
            // Set the cursor to the right of the new word
            text_edit_state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(usize::MAX))));
            text_edit_state.store(ui.ctx(), response.id);
        }
        state.user_state =
            state
                .user_state
                .handle_arrow(pressed_down, pressed_up, matching_items.len());

        if matching_items.is_empty() {
            state.user_state = UserState::Typing;
        }

        let popup_id = self.id.with("popup");
        let text_size = ui.text_style_height(&TextStyle::Body);

        let selection_may_have_changed = response.changed() || pressed_down || pressed_up;
        let should_close_popup = popup_below_widget(
            ui,
            popup_id,
            &response,
            PopupCloseBehavior::CloseOnClickOutside,
            |ui| {
                let mut close_me = false;
                ui.set_max_height(text_size * 20.0);

                if matching_items.is_empty() {
                    ui.label("No results");
                    return close_me;
                }

                ScrollArea::vertical().show(ui, |ui| {
                    for (visual_index, (original_index, _)) in matching_items.iter().enumerate() {
                        let highlight = match state.user_state {
                            UserState::Selecting {
                                index: selected_index,
                            } => visual_index == selected_index,
                            UserState::Typing => false,
                        };

                        let response =
                            show_value(ui, highlight, &self.suggestions[*original_index]);

                        if selection_may_have_changed && highlight {
                            response.scroll_to_me(None);
                        }

                        if response.clicked() {
                            state.user_state = UserState::Selecting {
                                index: visual_index,
                            };
                            close_me = true;
                        }
                    }
                });

                close_me
            },
        );

        let has_focus = response.has_focus();
        let user_completed_search = matches!(should_close_popup, Some(true))
            || response.lost_focus() && ui.input(|reader| reader.key_pressed(Key::Enter));

        ui.memory_mut(|memory| {
            if has_focus {
                memory.open_popup(popup_id);
            }
            if user_completed_search {
                memory.close_popup();
            }
        });

        if user_completed_search {
            response.mark_changed();
            if let UserState::Selecting { index } = state.user_state {
                let (actual_index, _) = matching_items[index];
                *self.selected = self.suggestions[actual_index].to_string();
                state.user_state = UserState::Typing;
            }
        }

        response
    }
}

impl<'a, T: Clone + ToString + Debug + std::hash::Hash> Widget for CompletionEdit<'a, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.ui(ui, |ui, highlight, item| {
            ui.selectable_label(highlight, item.to_string())
        })
    }
}
