use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::coordinate_systems::AbsoluteTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayUserData {
    pub latest: AbsoluteTime,
    pub bookmarks: BookmarkCollection,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BookmarkCollection(pub BTreeMap<AbsoluteTime, Bookmark>);

impl BookmarkCollection {
    pub fn add(&mut self, at: AbsoluteTime) {
        self.0.insert(
            at,
            Bookmark {
                name: format!("#{}", self.0.len() + 1),
            },
        );
    }

    pub fn remove_if_exists(&mut self, position: &AbsoluteTime) -> Option<Bookmark> {
        self.0.remove(position)
    }

    pub fn next_after(&self, position: &AbsoluteTime) -> Option<(AbsoluteTime, &Bookmark)> {
        self.0
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
        self.0
            .iter()
            .filter_map(|(at, bookmark)| {
                if at < position {
                    Some((*at, bookmark))
                } else {
                    None
                }
            })
            .next_back()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
}
