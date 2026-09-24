use std::{hash::Hash, sync::Arc};

use egui::{
    Color32, Context, EventFilter, Id, Key, Modifiers, Popup, PopupCloseBehavior, Response,
    ScrollArea, TextEdit, TextStyle, Ui, Widget,
    cache::{ComputerMut, FrameCache},
    response::Flags,
    text::{CCursor, CCursorRange},
    text_edit::{TextEditOutput, TextEditState},
};

use crate::{
    matcher::fuzzy_matches,
    selection_list::{
        BoundaryBehavior, ListInput, SelectionListState, lock_text_edit_navigation, show_results,
    },
};

pub struct CompletionEdit<'a, T> {
    id: Id,
    suggestions: &'a [T],
    selected: &'a mut String,
    select_all_on_focus: bool,
    request_focus: bool,
}

#[derive(Debug, Clone, Default)]
struct CompletionEditState {
    selection: SelectionListState,
    typed_since_focused: bool,
    textedit_was_focused: bool,
}

#[derive(Default)]
struct MatcherSearch;
type MatchingItems = Arc<Vec<(usize, String)>>;
type CachedMatcherSearch = FrameCache<MatchingItems, MatcherSearch>;

impl<'a, T: ToString> ComputerMut<(&String, &'a [T]), MatchingItems> for MatcherSearch {
    fn compute(&mut self, (key, items): (&String, &'a [T])) -> MatchingItems {
        Arc::new(fuzzy_matches(key, items))
    }
}

fn get_matching_items<T: ToString + Hash>(
    ui: &mut Ui,
    query: &String,
    items: &[T],
) -> MatchingItems {
    ui.memory_mut(|memory| {
        let cache = memory.caches.cache::<CachedMatcherSearch>();
        Arc::clone(cache.get((query, items)))
    })
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

fn set_cursor(
    context: &Context,
    response: &Response,
    mut state: TextEditState,
    start: usize,
    end: usize,
) {
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(start),
        CCursor::new(end),
    )));
    state.store(context, response.id);
}

impl<'a, T: ToString + Hash> CompletionEdit<'a, T> {
    pub fn new(id_salt: impl Into<Id>, items: &'a [T], selected: &'a mut String) -> Self {
        Self {
            id: id_salt.into(),
            suggestions: items,
            selected,
            select_all_on_focus: true,
            request_focus: false,
        }
    }

    /// Disable automatic selection of the entire input when restoring focus with
    /// a specific cursor selection, such as an array-index placeholder.
    pub fn select_all_on_focus(mut self, enabled: bool) -> Self {
        self.select_all_on_focus = enabled;
        self
    }

    /// Request keyboard focus and select the input for replacement this frame.
    pub fn request_focus(mut self, requested: bool) -> Self {
        self.request_focus = requested;
        self
    }

    pub fn ui(
        mut self,
        ui: &mut Ui,
        show_value: impl FnMut(&mut Ui, bool, &T) -> Response,
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
        mut show_value: impl FnMut(&mut Ui, bool, &T) -> Response,
    ) -> Response {
        if self.request_focus {
            state.selection.clear();
        }
        let mut matching_items = get_matching_items(ui, self.selected, self.suggestions);
        state.selection.clamp(matching_items.len());
        let popup_id = self.id.with("popup");
        let text_edit_id = self.id.with("text-edit");
        if ui.memory(|memory| memory.has_focus(text_edit_id))
            && ui.input(|input| input.key_pressed(Key::ArrowDown))
        {
            // Focus navigation is computed before widgets consume key events.
            ui.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
            state.typed_since_focused = true;
            Popup::open_id(ui.ctx(), popup_id);
        }
        let is_popup_open = Popup::is_id_open(ui.ctx(), popup_id);
        let list_input = ListInput::consume(ui, is_popup_open, false);
        let TextEditOutput {
            response,
            state: text_edit_state,
            ..
        } = ui
            .scope(|ui| match state.selection.highlighted() {
                None => TextEdit::singleline(self.selected)
                    .id(text_edit_id)
                    .hint_text("Search")
                    .show(ui),
                Some(index) => {
                    let mut active_stroke = ui.visuals().widgets.active.bg_stroke;
                    active_stroke.color = ui.visuals().warn_fg_color;
                    ui.visuals_mut().widgets.inactive.bg_stroke = active_stroke;

                    let mut selected = matching_items
                        .get(index)
                        .map(|(_, value)| value.clone())
                        .unwrap_or_default();
                    let output = TextEdit::singleline(&mut selected)
                        .id(text_edit_id)
                        .text_color(Color32::GRAY)
                        .hint_text("Search")
                        .show(ui);
                    let cursor_position = selected.chars().count();
                    set_cursor(
                        ui.ctx(),
                        &output.response,
                        output.state.clone(),
                        cursor_position,
                        cursor_position,
                    );
                    if output.response.changed() {
                        *self.selected = selected;
                        state.selection.clear();
                    }
                    output
                }
            })
            .inner;
        let mut response = response.response;
        if self.request_focus {
            response.request_focus();
        }

        let text_changed = response.changed();
        if text_changed {
            state.typed_since_focused = true;
            state.selection.clear();
            matching_items = get_matching_items(ui, self.selected, self.suggestions);
        }
        state.selection.clamp(matching_items.len());
        if self.request_focus
            || self.select_all_on_focus && !state.textedit_was_focused && response.has_focus()
        {
            // Select all
            set_cursor(
                ui.ctx(),
                &response,
                text_edit_state,
                0,
                self.selected.chars().count(),
            );
        }
        if response.has_focus()
            && ui.input_mut(|input| input.consume_key(Modifiers::CTRL, Key::Space))
        {
            state.typed_since_focused = true;
            Popup::open_id(ui.ctx(), popup_id);
        }
        // Report changes only when the user commits the edited value.
        response.flags.set(Flags::CHANGED, false);

        if is_popup_open {
            lock_text_edit_navigation(ui, response.id, true);
            state.selection.navigate(
                list_input,
                matching_items.len(),
                BoundaryBehavior::ReturnToQuery,
            );
        }

        let text_size = ui.text_style_height(&TextStyle::Body);

        let selection_may_have_changed = text_changed || list_input.navigation_requested();
        let should_close_popup = Popup::from_response(&response)
            .id(popup_id)
            .open_memory(None)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_max_width(f32::INFINITY);
                ui.set_max_height(text_size * 20.0);

                let clicked = ScrollArea::vertical()
                    .show(ui, |ui| {
                        show_results(
                            ui,
                            self.suggestions,
                            matching_items.as_slice(),
                            state.selection.highlighted(),
                            selection_may_have_changed,
                            |(original_index, _)| *original_index,
                            &mut show_value,
                        )
                    })
                    .inner;
                if let Some(clicked) = clicked {
                    state.selection.select(clicked.visible_index);
                    true
                } else {
                    false
                }
            })
            .is_some_and(|response| response.inner);

        if response.lost_focus() {
            state.typed_since_focused = false;
        }
        let should_open_popup = response.has_focus() && state.typed_since_focused;
        let pressed_enter = ui.input(|reader| reader.key_pressed(Key::Enter));
        let user_completed_search = should_close_popup || response.lost_focus() && pressed_enter;

        if should_open_popup {
            Popup::open_id(ui.ctx(), popup_id);
        }
        if user_completed_search {
            Popup::close_id(ui.ctx(), popup_id);
        }

        if user_completed_search {
            response.mark_changed();
            if pressed_enter
                && state.selection.highlighted().is_none()
                && !matching_items.is_empty()
            {
                state.selection.select(0);
            }
            if let Some(actual_index) = state
                .selection
                .item_index(matching_items.as_slice(), |(item_index, _)| *item_index)
            {
                *self.selected = self.suggestions[actual_index].to_string();
                state.selection.clear();
            }
            response.request_focus();
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    response.id,
                    EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                );
            });
            let cursor_position = self.selected.chars().count();
            set_cursor(
                ui.ctx(),
                &response,
                TextEditState::load(ui.ctx(), response.id).unwrap_or_default(),
                cursor_position,
                cursor_position,
            );
            state.typed_since_focused = false;
        }
        state.textedit_was_focused = response.has_focus();

        response
    }
}

impl<T: ToString + Hash> Widget for CompletionEdit<'_, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.ui(ui, |ui, highlight, item| {
            ui.selectable_label(highlight, item.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{CentralPanel, Event, Pos2, RawInput, Rect, vec2};

    fn frame(
        context: &Context,
        values: &mut [String; 2],
        focus: bool,
        events: Vec<Event>,
    ) -> Vec<Response> {
        let mut responses = Vec::new();
        let _ = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))),
                events,
                ..Default::default()
            },
            |ui| {
                responses.clear();
                CentralPanel::default().show(ui, |ui| {
                    for (index, value) in values.iter_mut().enumerate() {
                        responses.push(
                            ui.add(
                                CompletionEdit::new(Id::new(index), &["alpha", "beta"], value)
                                    .request_focus(focus && index == 0),
                            ),
                        );
                    }
                });
            },
        );
        responses
    }

    #[test]
    fn control_space_opens_only_the_focused_completion_without_editing() {
        fn shortcut() -> Vec<Event> {
            vec![Event::Key {
                key: Key::Space,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
            }]
        }
        for initial in ["", "al"] {
            let context = Context::default();
            let mut values = [initial.to_owned(), String::new()];
            let responses = frame(&context, &mut values, true, vec![]);
            assert_eq!(
                context.memory(|memory| memory.focused()),
                Some(responses[0].id)
            );
            assert!(!Popup::is_any_open(&context));
            let responses = frame(&context, &mut values, false, shortcut());
            assert!(Popup::is_id_open(&context, Id::new(0usize).with("popup")));
            assert!(!Popup::is_id_open(&context, Id::new(1usize).with("popup")));
            assert!(responses.iter().all(|response| !response.changed()));
            assert_eq!(values, [initial.to_owned(), String::new()]);
            frame(&context, &mut values, false, shortcut());
            assert!(Popup::is_id_open(&context, Id::new(0usize).with("popup")));

            Popup::close_all(&context);
            context.memory_mut(|memory| memory.surrender_focus(responses[0].id));
            frame(&context, &mut values, false, shortcut());
            assert!(!Popup::is_any_open(&context));
        }
    }

    fn key(key: Key) -> Vec<Event> {
        vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]
    }

    #[test]
    fn arrow_down_opens_only_the_focused_completion_and_navigates() {
        for initial in ["", "al", "unmatched"] {
            let context = Context::default();
            let mut values = [initial.to_owned(), String::new()];
            frame(&context, &mut values, true, vec![]);
            let responses = frame(&context, &mut values, false, key(Key::ArrowDown));
            assert!(Popup::is_id_open(&context, Id::new(0usize).with("popup")));
            assert!(!Popup::is_id_open(&context, Id::new(1usize).with("popup")));
            assert!(responses[0].has_focus());
            assert!(responses.iter().all(|response| !response.changed()));
            assert_eq!(values, [initial.to_owned(), String::new()]);
            let responses = frame(&context, &mut values, false, key(Key::Enter));
            assert!(responses[0].changed());
            assert!(responses[0].has_focus());
            assert_eq!(
                values[0],
                if initial == "unmatched" {
                    initial
                } else {
                    "alpha"
                }
            );
            assert!(!Popup::is_any_open(&context));

            context.memory_mut(|memory| memory.surrender_focus(responses[0].id));
            frame(&context, &mut values, false, key(Key::ArrowDown));
            assert!(!Popup::is_any_open(&context));
        }

        let context = Context::default();
        let mut values = [String::new(), String::new()];
        frame(&context, &mut values, true, vec![]);
        frame(&context, &mut values, false, key(Key::ArrowDown));
        frame(&context, &mut values, false, key(Key::ArrowDown));
        frame(&context, &mut values, false, key(Key::Enter));
        assert_eq!(values[0], "beta");
    }

    #[test]
    fn enter_keeps_focus_and_places_cursor_after_the_committed_value() {
        for initial in ["al", "unmatched", "é"] {
            for navigate in [false, true] {
                let context = Context::default();
                let mut values = [initial.to_owned(), String::new()];
                frame(&context, &mut values, true, vec![]);
                if navigate {
                    frame(&context, &mut values, false, key(Key::ArrowDown));
                }
                let responses = frame(&context, &mut values, false, key(Key::Enter));
                assert!(responses[0].changed());
                assert!(responses[0].has_focus());
                assert!(!Popup::is_any_open(&context));
                let expected = if initial == "al" { "alpha" } else { initial };
                assert_eq!(values[0], expected);
                frame(&context, &mut values, false, vec![]);
                assert!(!Popup::is_any_open(&context));
                frame(
                    &context,
                    &mut values,
                    false,
                    vec![Event::Text(".nested".into())],
                );
                assert_eq!(values[0], format!("{expected}.nested"));
                let responses = frame(&context, &mut values, false, key(Key::Enter));
                assert!(responses[0].has_focus());
                assert_eq!(values[0], format!("{expected}.nested"));
            }
        }
    }
}
