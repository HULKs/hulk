use std::sync::Arc;

use eframe::egui::{Id, Response, Ui, Widget};
use hulk_widgets::CompletionEdit;

use crate::backend::catalog::TopicCatalog;

pub struct TopicCompletionEdit<'ui> {
    id: Id,
    catalog: Arc<TopicCatalog>,
    selector: &'ui mut String,
    show_all_topics: bool,
}

impl<'ui> TopicCompletionEdit<'ui> {
    pub fn namespace_topics(
        id_salt: impl Into<Id>,
        catalog: Arc<TopicCatalog>,
        selector: &'ui mut String,
    ) -> Self {
        Self {
            id: id_salt.into(),
            catalog,
            selector,
            show_all_topics: false,
        }
    }

    pub fn all_topics(
        id_salt: impl Into<Id>,
        catalog: Arc<TopicCatalog>,
        selector: &'ui mut String,
    ) -> Self {
        Self {
            id: id_salt.into(),
            catalog,
            selector,
            show_all_topics: true,
        }
    }

    fn selectors(&self) -> Vec<String> {
        let topics = if self.show_all_topics {
            self.catalog.all_topics().collect::<Vec<_>>()
        } else {
            self.catalog.namespace_topics().collect::<Vec<_>>()
        };
        topics
            .into_iter()
            .map(|topic| topic.selector.clone())
            .collect()
    }
}

impl Widget for TopicCompletionEdit<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let selectors = self.selectors();
        ui.add(CompletionEdit::new(self.id, &selectors, self.selector))
    }
}
