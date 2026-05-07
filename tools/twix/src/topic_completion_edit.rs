use eframe::egui::{Id, Response, Ui, Widget};
use hulk_widgets::CompletionEdit;

use crate::backend::TopicListState;

pub struct TopicCompletionEdit<'a> {
    id: Id,
    topics: &'a TopicListState,
    selected: &'a mut String,
}

impl<'a> TopicCompletionEdit<'a> {
    pub fn new(
        id_salt: impl Into<Id>,
        topics: &'a TopicListState,
        selected: &'a mut String,
    ) -> Self {
        Self {
            id: id_salt.into(),
            topics,
            selected,
        }
    }
}

impl Widget for TopicCompletionEdit<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let topic_names = self
            .topics
            .topics
            .iter()
            .map(|topic| topic.name.clone())
            .collect::<Vec<_>>();

        ui.horizontal(|ui| {
            let response = ui.add(CompletionEdit::new(self.id, &topic_names, self.selected));
            if self.topics.discovering {
                ui.label("discovering topics...");
            }
            response
        })
        .inner
    }
}
