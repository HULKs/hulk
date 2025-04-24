use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::coordinate_systems::AbsoluteTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmarks {
    pub replay_identifier: u64,
    pub latest: AbsoluteTime,
    pub bookmarks: BTreeMap<AbsoluteTime, Bookmark>,
}

impl Bookmarks {
    pub fn add(&mut self, at: AbsoluteTime) {
        self.bookmarks.insert(
            at,
            Bookmark {
                name: format!("#{}", self.bookmarks.len() + 1),
            },
        );
    }

    pub fn remove_if_exists(&mut self, position: &AbsoluteTime) -> Option<Bookmark> {
        self.bookmarks.remove(position)
    }

    pub fn next_after(&self, position: &AbsoluteTime) -> Option<(AbsoluteTime, &Bookmark)> {
        self.bookmarks
            .iter()
            .filter_map(|(at, bookmark)| {
                if at > position {
                    Some((*at, bookmark))
                } else {
                    None
                }
            })
            .next()
    }

    pub fn previous_before(&self, position: &AbsoluteTime) -> Option<(AbsoluteTime, &Bookmark)> {
        self.bookmarks
            .iter()
            .filter_map(|(at, bookmark)| {
                if at < position {
                    Some((*at, bookmark))
                } else {
                    None
                }
            })
            .last()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
}
