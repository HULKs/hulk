use crate::CompletionEdit;

use communication::client::PathsEvent;
use egui::{Id, Response, Ui, Widget};

pub enum PathFilter {
    Readable,
    Writable,
}

pub struct NaoPathCompletionEdit<'ui> {
    id: Id,
    paths_events: PathsEvent,
    path: &'ui mut String,
    filter: PathFilter,
}

impl<'ui> NaoPathCompletionEdit<'ui> {
    pub fn new(
        id_salt: impl Into<Id>,
        paths_events: PathsEvent,
        path: &'ui mut String,
        filter: PathFilter,
    ) -> Self {
        Self {
            id: id_salt.into(),
            paths_events,
            path,
            filter,
        }
    }

    fn list_paths(&self) -> Vec<String> {
        match self.paths_events.as_ref() {
            Some(Ok(paths)) => paths
                .iter()
                .filter_map(|(path, entry)| match self.filter {
                    PathFilter::Readable if entry.is_readable => Some(path.clone()),
                    PathFilter::Writable if entry.is_writable => Some(path.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

impl<'ui> Widget for NaoPathCompletionEdit<'ui> {
    fn ui(self, ui: &mut Ui) -> Response {
        let paths = self.list_paths();
        ui.add(CompletionEdit::new(self.id, &paths, self.path))
    }
}
