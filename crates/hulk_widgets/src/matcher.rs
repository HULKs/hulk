use std::{
    cmp::Reverse,
    sync::{LazyLock, Mutex},
};

use nucleo_matcher::{
    Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};

#[derive(Default)]
struct MatcherResources {
    matcher: Matcher,
    buffer: Vec<char>,
}

static MATCHER_RESOURCES: LazyLock<Mutex<MatcherResources>> =
    LazyLock::new(|| Mutex::new(MatcherResources::default()));

pub(crate) fn fuzzy_matches<T: ToString>(query: &str, items: &[T]) -> Vec<(usize, String)> {
    let mut strings: Vec<_> = items.iter().map(|item| Some(item.to_string())).collect();
    let mut matches = Vec::new();
    fuzzy_match_indices(query, &strings, &mut matches, |item| {
        item.as_deref().unwrap_or_default()
    });
    matches
        .into_iter()
        .map(|(_, index)| (index, strings[index].take().unwrap_or_default()))
        .collect()
}

pub(crate) fn fuzzy_match_indices<T>(
    query: &str,
    items: &[T],
    matches: &mut Vec<(u32, usize)>,
    mut search_text: impl for<'a> FnMut(&'a T) -> &'a str,
) {
    matches.clear();
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    if pattern.atoms.is_empty() {
        matches.extend((0..items.len()).map(|index| (0, index)));
        return;
    }

    let search_texts: Vec<_> = items.iter().map(&mut search_text).collect();
    {
        let mut resources = MATCHER_RESOURCES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let MatcherResources { matcher, buffer } = &mut *resources;
        buffer.clear();
        matches.extend(
            search_texts
                .into_iter()
                .enumerate()
                .filter_map(|(index, search_text)| {
                    pattern
                        .score(Utf32Str::new(search_text, buffer), matcher)
                        .map(|score| (score, index))
                }),
        );
    }
    matches.sort_by_key(|(score, _)| Reverse(*score));
}

#[cfg(test)]
mod tests {
    use super::{fuzzy_match_indices, fuzzy_matches};

    #[test]
    fn ranks_stronger_fuzzy_match_first() {
        let items = ["zzalpha", "unrelated", "alpha"];

        let matches = fuzzy_matches("alpha", &items);

        assert_eq!(
            matches,
            vec![(2, "alpha".to_owned()), (0, "zzalpha".to_owned())]
        );
    }

    #[test]
    fn ranks_borrowed_items_without_copying_their_text() {
        let items = ["zzalpha", "unrelated", "alpha"];
        let mut matches = Vec::new();

        fuzzy_match_indices("alpha", &items, &mut matches, |item| item);

        assert_eq!(
            matches
                .into_iter()
                .map(|(_, index)| index)
                .collect::<Vec<_>>(),
            vec![2, 0]
        );
    }

    #[test]
    fn search_text_callback_can_reenter_fuzzy_matching() {
        let items = ["alpha"];
        let mut matches = Vec::new();

        fuzzy_match_indices("alpha", &items, &mut matches, |item| {
            assert_eq!(
                fuzzy_matches("inner", &["inner"]),
                [(0, "inner".to_owned())]
            );
            item
        });

        assert_eq!(
            matches
                .into_iter()
                .map(|(_, index)| index)
                .collect::<Vec<_>>(),
            [0]
        );
    }
}
