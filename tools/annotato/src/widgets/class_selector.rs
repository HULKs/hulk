use eframe::egui::{ComboBox, Id, Response, Ui, Widget};
use std::hash::Hash;

use crate::classes::Classes;
pub trait EnumIter {
    fn list() -> Vec<Self>
    where
        Self: Sized;
}

pub struct ClassSelector<'a> {
    id: Id,
    currently_selected: &'a mut Classes,
}

impl<'a> ClassSelector<'a> {
    pub fn new(id_source: impl Hash, currently_selected: &'a mut Classes) -> Self {
        Self {
            id: Id::new(id_source),
            currently_selected,
        }
    }
}

impl<'a> Widget for ClassSelector<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        if let Some(class) =
            ui.input(|i| i.keys_down.iter().find_map(|key| Classes::from_key(*key)))
        {
            *self.currently_selected = class;
        }

        let scroll_y = ui.input(|i| i.scroll_delta.y);
        if scroll_y > 0.0 {
            *self.currently_selected = Classes::next(self.currently_selected)
        } else if scroll_y < 0.0 {
            *self.currently_selected = Classes::previous(self.currently_selected)
        }
        ComboBox::from_id_source(self.id)
            .selected_text(format!("{:?}", self.currently_selected))
            .show_ui(ui, |ui| {
                Classes::list().into_iter().for_each(|class| {
                    ui.selectable_value(self.currently_selected, class, format!("{:?}", class));
                });
            })
            .response
    }
}
