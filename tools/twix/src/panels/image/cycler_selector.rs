use communication::client::Cycler;
use eframe::egui::{ComboBox, Response, Ui, Widget};

pub struct VisionCyclerSelector {
    cycler: Cycler,
}

impl VisionCyclerSelector {
    pub fn selected_cycler(&self) -> Cycler {
        self.cycler
    }
}

impl Default for VisionCyclerSelector {
    fn default() -> Self {
        Self {
            cycler: Cycler::VisionTop,
        }
    }
}

impl VisionCyclerSelector {
    pub fn new(cycler: Cycler) -> Self {
        Self { cycler }
    }
}

impl Widget for &mut VisionCyclerSelector {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut camera_selection_changed = false;
        let mut combo_box = ComboBox::from_label("Cycler")
            .selected_text(format!("{:?}", self.cycler))
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(&mut self.cycler, Cycler::VisionTop, "VisionTop")
                    .clicked()
                {
                    camera_selection_changed = true;
                };
                if ui
                    .selectable_value(&mut self.cycler, Cycler::VisionBottom, "VisionBottom")
                    .clicked()
                {
                    camera_selection_changed = true;
                };
            });
        if camera_selection_changed {
            combo_box.response.mark_changed()
        }
        combo_box.response
    }
}
