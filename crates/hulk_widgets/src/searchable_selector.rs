use egui::{Id, Response, ScrollArea, TextEdit, Ui};

use crate::{
    matcher::fuzzy_match_indices,
    selection_list::{
        BoundaryBehavior, ListInput, SelectionListState, lock_text_edit_navigation, show_results,
    },
};

/// Searchable content for use inside an existing popup or menu.
///
/// The ID passed to [`Self::new`] must be stable and unique among selector instances.
pub struct SearchableSelector<'a, T> {
    id: Id,
    items: &'a [T],
    reset_on_show: bool,
}

#[derive(Clone, Default)]
struct SelectorState {
    query: String,
    selection: SelectionListState,
    last_rendered_frame: Option<u64>,
    matches: Vec<(u32, usize)>,
}

impl<'a, T> SearchableSelector<'a, T> {
    /// Creates a selector with an ID that remains stable while its popup is in use.
    pub fn new(id_salt: impl Into<Id>, items: &'a [T]) -> Self {
        Self {
            id: id_salt.into(),
            items,
            reset_on_show: false,
        }
    }

    /// Clears and focuses the search field when `reset` is true for the current frame.
    pub fn reset_on_show(mut self, reset: bool) -> Self {
        self.reset_on_show = reset;
        self
    }

    /// Shows the search field and matching rows, returning the original item index on selection.
    pub fn show(
        self,
        ui: &mut Ui,
        search_text: impl for<'b> FnMut(&'b T) -> &'b str,
        mut show_row: impl FnMut(&mut Ui, bool, &T) -> Response,
    ) -> Option<usize> {
        let state_id = self.id.with("state");
        let search_id = self.id.with("search");
        let current_frame = ui.ctx().cumulative_frame_nr();
        let mut state = ui
            .ctx()
            .data_mut(|data| data.remove_temp::<SelectorState>(state_id))
            .unwrap_or_default();
        let reappeared = self.reset_on_show
            || state
                .last_rendered_frame
                .is_none_or(|last_frame| last_frame + 1 < current_frame);

        if reappeared {
            state.query.clear();
            state.selection.clear();
        }

        let had_focus = ui.memory(|memory| memory.has_focus(search_id));
        let list_input = ListInput::consume(ui, had_focus, true);

        let search_response = ui.add(
            TextEdit::singleline(&mut state.query)
                .id(search_id)
                .hint_text("Search")
                .desired_width(f32::INFINITY),
        );
        if reappeared {
            search_response.request_focus();
        }
        if search_response.has_focus() {
            lock_text_edit_navigation(ui, search_id, false);
        }
        let query_changed = search_response.changed();
        if query_changed {
            state.selection.clear();
        }

        fuzzy_match_indices(&state.query, self.items, &mut state.matches, search_text);
        state
            .selection
            .navigate(list_input, state.matches.len(), BoundaryBehavior::Wrap);

        let mut selected = list_input
            .accepted()
            .then(|| {
                state
                    .selection
                    .item_index_or_first(&state.matches, |(_, index)| *index)
            })
            .flatten();
        let highlight_changed = list_input.navigation_requested() || query_changed;
        let max_height = ui.spacing().interact_size.y * 12.0;

        ScrollArea::vertical()
            .id_salt(self.id.with("results"))
            .max_height(max_height)
            .show(ui, |ui| {
                if let Some(clicked) = show_results(
                    ui,
                    self.items,
                    &state.matches,
                    state.selection.highlighted(),
                    highlight_changed,
                    |(_, item_index)| *item_index,
                    &mut show_row,
                ) {
                    selected = Some(clicked.item_index);
                }
            });

        state.last_rendered_frame = Some(current_frame);
        ui.ctx()
            .data_mut(|writer| writer.insert_temp(state_id, state));
        selected
    }
}
