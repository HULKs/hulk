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

// Reuse nucleo's roughly 135 KB scratch allocation across widgets.
static MATCHER_RESOURCES: LazyLock<Mutex<MatcherResources>> =
    LazyLock::new(|| Mutex::new(MatcherResources::default()));

pub(crate) fn fuzzy_matches<T: ToString>(query: &str, items: &[T]) -> Vec<(usize, String)> {
    let mut strings: Vec<_> = items.iter().map(ToString::to_string).collect();
    let mut matches = Vec::new();
    fuzzy_match_indices(query, &strings, &mut matches);
    matches
        .into_iter()
        .map(|(_, index)| (index, std::mem::take(&mut strings[index])))
        .collect()
}

pub(crate) fn fuzzy_match_indices(
    query: &str,
    items: &[impl AsRef<str>],
    matches: &mut Vec<(u32, usize)>,
) {
    matches.clear();
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    if pattern.atoms.is_empty() {
        matches.extend((0..items.len()).map(|index| (0, index)));
        return;
    }

    {
        let mut resources = MATCHER_RESOURCES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let MatcherResources { matcher, buffer } = &mut *resources;
        buffer.clear();
        matches.extend(items.iter().enumerate().filter_map(|(index, search_text)| {
            pattern
                .score(Utf32Str::new(search_text.as_ref(), buffer), matcher)
                .map(|score| (score, index))
        }));
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

        fuzzy_match_indices("alpha", &items, &mut matches);

        assert_eq!(
            matches
                .into_iter()
                .map(|(_, index)| index)
                .collect::<Vec<_>>(),
            vec![2, 0]
        );
    }
}
