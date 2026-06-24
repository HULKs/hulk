use std::sync::Arc;

use eframe::egui::{Id, Response, Ui, Widget};

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
        let mut response = ui
            .push_id(self.id, |ui| ui.text_edit_singleline(self.selector))
            .inner;

        if response.has_focus() && !self.selector.is_empty() {
            let needle = self.selector.to_lowercase();
            for selector in selectors
                .iter()
                .filter(|selector| selector.to_lowercase().contains(&needle))
                .take(8)
            {
                if ui.button(selector).clicked() {
                    *self.selector = selector.clone();
                    response.mark_changed();
                }
            }
        }

        response
    }
}
