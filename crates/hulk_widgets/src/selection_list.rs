use egui::{EventFilter, Id, Key, Modifiers, Response, Ui};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BoundaryBehavior {
    Wrap,
    ReturnToQuery,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SelectionListState {
    highlighted: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ListInput {
    previous: bool,
    next: bool,
    accept: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ClickedItem {
    pub(crate) visible_index: usize,
    pub(crate) item_index: usize,
}

impl ListInput {
    pub(crate) fn consume(ui: &mut Ui, active: bool, consume_accept: bool) -> Self {
        if !active {
            return Self::default();
        }

        ui.input_mut(|input| Self {
            previous: input.consume_key(Modifiers::NONE, Key::ArrowUp)
                || input.consume_key(Modifiers::SHIFT, Key::Tab),
            next: input.consume_key(Modifiers::NONE, Key::ArrowDown)
                || input.consume_key(Modifiers::NONE, Key::Tab),
            accept: consume_accept && input.consume_key(Modifiers::NONE, Key::Enter),
        })
    }

    pub(crate) fn navigation_requested(self) -> bool {
        self.previous || self.next
    }

    pub(crate) fn accepted(self) -> bool {
        self.accept
    }
}

impl SelectionListState {
    pub(crate) fn highlighted(self) -> Option<usize> {
        self.highlighted
    }

    pub(crate) fn clear(&mut self) {
        self.highlighted = None;
    }

    pub(crate) fn select(&mut self, index: usize) {
        self.highlighted = Some(index);
    }

    pub(crate) fn clamp(&mut self, item_count: usize) {
        self.highlighted = self.highlighted.filter(|&index| index < item_count);
    }

    pub(crate) fn navigate(
        &mut self,
        input: ListInput,
        item_count: usize,
        boundary_behavior: BoundaryBehavior,
    ) {
        self.clamp(item_count);
        if item_count == 0 || !input.navigation_requested() {
            return;
        }

        self.highlighted = match boundary_behavior {
            BoundaryBehavior::Wrap if input.previous => match self.highlighted {
                None | Some(0) => Some(item_count - 1),
                Some(index) => Some(index - 1),
            },
            BoundaryBehavior::Wrap => match self.highlighted {
                None => Some(0),
                Some(index) => Some((index + 1) % item_count),
            },
            BoundaryBehavior::ReturnToQuery => {
                match (input.previous, input.next, self.highlighted) {
                    (_, true, None) => Some(0),
                    (true, _, None) => Some(item_count - 1),
                    (true, _, Some(0)) => None,
                    (true, _, Some(index)) => Some(index - 1),
                    (_, true, Some(index)) if index == item_count - 1 => None,
                    (_, true, Some(index)) => Some(index + 1),
                    (false, false, highlighted) => highlighted,
                }
            }
        };
    }

    pub(crate) fn item_index<M>(
        self,
        matches: &[M],
        item_index: impl Fn(&M) -> usize,
    ) -> Option<usize> {
        self.highlighted
            .and_then(|index| matches.get(index))
            .map(item_index)
    }

    pub(crate) fn item_index_or_first<M>(
        self,
        matches: &[M],
        item_index: impl Fn(&M) -> usize,
    ) -> Option<usize> {
        self.highlighted
            .and_then(|index| matches.get(index))
            .or_else(|| matches.first())
            .map(item_index)
    }
}

pub(crate) fn lock_text_edit_navigation(ui: &mut Ui, id: Id, horizontal_arrows: bool) {
    ui.memory_mut(|memory| {
        memory.set_focus_lock_filter(
            id,
            EventFilter {
                tab: true,
                horizontal_arrows,
                vertical_arrows: true,
                ..Default::default()
            },
        );
    });
}

pub(crate) fn show_results<T, M>(
    ui: &mut Ui,
    items: &[T],
    matches: &[M],
    highlighted: Option<usize>,
    scroll_to_highlight: bool,
    item_index: impl Fn(&M) -> usize,
    mut show_row: impl FnMut(&mut Ui, bool, &T) -> Response,
) -> Option<ClickedItem> {
    if matches.is_empty() {
        ui.label("No results");
        return None;
    }

    let mut clicked = None;
    for (visible_index, matched_item) in matches.iter().enumerate() {
        let item_index = item_index(matched_item);
        let item = &items[item_index];
        let is_highlighted = highlighted == Some(visible_index);
        let response = show_row(ui, is_highlighted, item);

        if scroll_to_highlight && is_highlighted {
            response.scroll_to_me(None);
        }
        if response.clicked() {
            clicked = Some(ClickedItem {
                visible_index,
                item_index,
            });
        }
    }
    clicked
}

#[cfg(test)]
mod tests {
    use super::{BoundaryBehavior, ListInput, SelectionListState};

    fn previous() -> ListInput {
        ListInput {
            previous: true,
            ..Default::default()
        }
    }

    fn next() -> ListInput {
        ListInput {
            next: true,
            ..Default::default()
        }
    }

    #[test]
    fn navigation_is_safe_with_no_items() {
        for boundary_behavior in [BoundaryBehavior::Wrap, BoundaryBehavior::ReturnToQuery] {
            for input in [previous(), next()] {
                let mut state = SelectionListState {
                    highlighted: Some(usize::MAX),
                };

                state.navigate(input, 0, boundary_behavior);

                assert_eq!(state.highlighted(), None);
            }
        }
    }

    #[test]
    fn wrapping_navigation_wraps_at_both_ends() {
        let mut state = SelectionListState::default();
        state.navigate(previous(), 3, BoundaryBehavior::Wrap);
        assert_eq!(state.highlighted(), Some(2));

        state.navigate(next(), 3, BoundaryBehavior::Wrap);
        assert_eq!(state.highlighted(), Some(0));

        state.navigate(previous(), 3, BoundaryBehavior::Wrap);
        assert_eq!(state.highlighted(), Some(2));
    }

    #[test]
    fn completion_navigation_returns_to_the_query_at_both_ends() {
        let mut state = SelectionListState::default();
        state.navigate(next(), 3, BoundaryBehavior::ReturnToQuery);
        assert_eq!(state.highlighted(), Some(0));

        state.navigate(previous(), 3, BoundaryBehavior::ReturnToQuery);
        assert_eq!(state.highlighted(), None);

        state.navigate(previous(), 3, BoundaryBehavior::ReturnToQuery);
        assert_eq!(state.highlighted(), Some(2));

        state.navigate(next(), 3, BoundaryBehavior::ReturnToQuery);
        assert_eq!(state.highlighted(), None);
    }

    #[test]
    fn one_item_list_preserves_each_boundary_behavior() {
        let mut wrapping = SelectionListState::default();
        wrapping.navigate(next(), 1, BoundaryBehavior::Wrap);
        wrapping.navigate(next(), 1, BoundaryBehavior::Wrap);
        assert_eq!(wrapping.highlighted(), Some(0));

        let mut completion = SelectionListState::default();
        completion.navigate(next(), 1, BoundaryBehavior::ReturnToQuery);
        completion.navigate(next(), 1, BoundaryBehavior::ReturnToQuery);
        assert_eq!(completion.highlighted(), None);
    }

    #[test]
    fn stale_selection_is_clamped_before_navigation() {
        let mut state = SelectionListState {
            highlighted: Some(4),
        };

        state.navigate(ListInput::default(), 2, BoundaryBehavior::Wrap);

        assert_eq!(state.highlighted(), None);
    }
}
