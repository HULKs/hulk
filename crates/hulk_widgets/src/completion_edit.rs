use std::hash::Hash;

use egui::{
    AreaState, CompletionPopup, Context, FocusDirection, Frame, Id, Key, LayerId, Modifiers, Order,
    Rect, Response, Suggestion, TextEdit, Ui, Widget,
    cache::{ComputerMut, FrameCache},
    emath::RectAlign,
    response::Flags,
    text::{CCursor, CCursorRange},
    text_edit::TextEditState,
};

use crate::matcher::fuzzy_matches;

/// Fuzzy whole-field completion. Changes are reported only on acceptance or Enter.
/// Click, type, or press Ctrl+Space for suggestions; Tab moves focus and Escape dismisses them.
pub struct CompletionEdit<'a, T> {
    id: Id,
    suggestions: &'a [T],
    text: &'a mut String,
    select_all_on_focus: bool,
    request_focus: bool,
}

#[derive(Clone, Default)]
struct CompletionEditState {
    active: bool,
    was_focused: bool,
    above: Option<bool>,
    viewport: Option<Rect>,
}

impl CompletionEditState {
    fn close(&mut self) {
        self.active = false;
        self.above = None;
    }

    /// Returns whether dismissal or focus traversal should suppress activation this frame.
    fn handle_input(&mut self, ui: &Ui, id: Id) -> bool {
        let focused = ui.memory(|memory| memory.has_focus(id));
        if !focused {
            self.close();
        }
        let clicked = ui.input(|input| input.pointer.primary_pressed())
            && ui.read_response(id).is_some_and(|response| {
                ui.ctx()
                    .rect_contains_pointer(response.layer_id, response.interact_rect)
            });
        self.active |= clicked;
        let mut focus_direction = None;
        let mut dismissed = false;
        if focused {
            ui.input_mut(|input| {
                focus_direction = if input.consume_key(Modifiers::SHIFT, Key::Tab) {
                    Some(FocusDirection::Previous)
                } else if input.consume_key(Modifiers::NONE, Key::Tab) {
                    Some(FocusDirection::Next)
                } else {
                    None
                };
                dismissed = self.active && input.consume_key(Modifiers::NONE, Key::Escape);
                if input.consume_key(Modifiers::CTRL, Key::Space)
                    || input.key_pressed(Key::ArrowDown)
                {
                    self.active = true;
                }
            });
        }
        let suppress_activation = dismissed || focus_direction.is_some();
        if suppress_activation {
            self.close();
        }
        if let Some(direction) = focus_direction {
            ui.memory_mut(|memory| memory.move_focus(direction));
        }
        suppress_activation
    }

    fn popup_geometry(&mut self, ui: &Ui, id: Id, frame_height: f32) -> (RectAlign, f32) {
        let editor_rect = ui.read_response(id).map_or_else(
            || {
                let mut rect = ui.available_rect_before_wrap();
                rect.set_height(ui.spacing().interact_size.y);
                rect
            },
            |response| response.rect,
        );
        let viewport = ui.content_rect();
        if self.viewport != Some(viewport) {
            self.above = None;
            self.viewport = Some(viewport);
        }
        let below = (viewport.bottom() - editor_rect.bottom()).max(0.0);
        let above = (editor_rect.top() - viewport.top()).max(0.0);
        let open_above = *self
            .above
            .get_or_insert(below < 200.0 + frame_height && above > below);
        let align = if open_above {
            RectAlign::TOP_START
        } else {
            RectAlign::BOTTOM_START
        };
        let max_height =
            ((if open_above { above } else { below }) - frame_height).clamp(0.0, 200.0);
        (align, max_height)
    }
}

#[derive(Default)]
struct MatcherSearch;
type CachedMatcherSearch = FrameCache<Vec<String>, MatcherSearch>;

impl<'a, T: ToString> ComputerMut<(&str, &'a [T]), Vec<String>> for MatcherSearch {
    fn compute(&mut self, (key, items): (&str, &'a [T])) -> Vec<String> {
        fuzzy_matches(key, items)
            .into_iter()
            .map(|(_, text)| text)
            .collect()
    }
}

fn set_cursor(context: &Context, id: Id, start: usize, end: usize) {
    let mut state = TextEditState::load(context, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(start),
        CCursor::new(end),
    )));
    state.store(context, id);
}

fn completion_popup_layer(editor_id: Id) -> LayerId {
    // shortcut: egui 0.37's internal popup ID, remove when CompletionPopup exposes sizing control.
    LayerId::new(Order::Foreground, editor_id.with("completion_popup"))
}

fn restore_popup_height(context: &Context, editor_id: Id, height: f32) {
    let layer = completion_popup_layer(editor_id);
    if let Some(mut state) = AreaState::load(context, layer.id)
        && let Some(size) = &mut state.size
    {
        size.y = height;
        context.memory_mut(|memory| {
            memory.areas_mut().set_state(layer, state);
        });
    }
}

impl<'a, T: ToString + Hash> CompletionEdit<'a, T> {
    pub fn new(id_salt: impl Into<Id>, items: &'a [T], text: &'a mut String) -> Self {
        Self {
            id: id_salt.into(),
            suggestions: items,
            text,
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
}

impl<T: ToString + Hash> Widget for CompletionEdit<'_, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let id = self.id.with("text-edit");
        let context = ui.ctx().clone();
        let state_id = self.id.with("completion-state");
        let mut state = context
            .data(|data| data.get_temp::<CompletionEditState>(state_id))
            .unwrap_or_default();
        if self.request_focus {
            context.memory_mut(|memory| memory.request_focus(id));
        }
        let suppress_activation = state.handle_input(ui, id);
        let enter_pressed = ui.input(|input| input.key_pressed(Key::Enter));
        let frame_height = Frame::popup(ui.style()).total_margin().sum().y;
        let (align, max_height) = state.popup_geometry(ui, id, frame_height);
        let previous_text = self.text.clone();
        let pointer_on_popup = state.active
            && context
                .pointer_interact_pos()
                .and_then(|pos| context.layer_id_at(pos))
                == Some(completion_popup_layer(id));
        let output = CompletionPopup::new(id)
            .word_boundary(|_| false)
            .accept_keys(vec![Key::Enter])
            .align(align)
            .max_height(max_height)
            .show(
                ui,
                TextEdit::singleline(self.text).hint_text("Search"),
                |query| {
                    if !suppress_activation && query.text != previous_text {
                        state.active = true;
                    }
                    if !state.active {
                        return Vec::new();
                    }
                    if pointer_on_popup {
                        // Keep editor focus until the popup rows have handled the click.
                        context.memory_mut(|memory| memory.request_focus(id));
                    }
                    let suggestions = context.memory_mut(|memory| {
                        memory
                            .caches
                            .cache::<CachedMatcherSearch>()
                            .get((query.text, self.suggestions))
                            .iter()
                            .map(Suggestion::new)
                            .collect::<Vec<_>>()
                    });
                    // set_state also marks the area visible; only touch it if it will be drawn.
                    if !suggestions.is_empty()
                        && !enter_pressed
                        && context.memory(|memory| memory.has_focus(id))
                    {
                        restore_popup_height(&context, id, max_height + frame_height);
                    }
                    suggestions
                },
            );
        let mut response = output.text_edit.response.response;
        if response.clicked() && !state.active {
            state.active = true;
            context.request_repaint();
        }
        if self.request_focus
            || self.select_all_on_focus && !state.was_focused && response.has_focus()
        {
            set_cursor(&context, id, 0, self.text.chars().count());
        }

        // Typing edits the query; only accepting a suggestion or Enter commits it.
        let manual_enter = response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
        let committed = output.accepted.is_some() || manual_enter;
        if let Some(accepted) = output.accepted {
            // Native completion replaces only the word before the cursor, not the whole field.
            *self.text = accepted.insert;
        }
        if committed {
            state.close();
            response.request_focus();
            let end = self.text.chars().count();
            set_cursor(&context, id, end, end);
        }
        response.flags.set(Flags::CHANGED, committed);
        state.was_focused = response.has_focus();
        if !state.was_focused || !state.active {
            state.close();
        }
        context.data_mut(|data| data.insert_temp(state_id, state));
        response
    }
}
