use communication::messages::Path;
use eframe::egui::{ComboBox, Response, Ui, Widget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisionCycler {
    Top,
    Bottom,
}

impl VisionCycler {
    pub fn as_path(&self) -> Path {
        match self {
            VisionCycler::Top => "VisionTop".to_string(),
            VisionCycler::Bottom => "VisionBottom".to_string(),
        }
    }

    pub fn as_snake_case_path(&self) -> String {
        match self {
            VisionCycler::Top => "vision_top".to_string(),
            VisionCycler::Bottom => "vision_bottom".to_string(),
        }
    }
}

impl TryFrom<&str> for VisionCycler {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "VisionTop" => Ok(VisionCycler::Top),
            "VisionBottom" => Ok(VisionCycler::Bottom),
            _ => Err("Invalid vision cycler"),
        }
    }
}

#[derive(Debug)]
pub struct VisionCyclerSelector<'a> {
    cycler: &'a mut VisionCycler,
}

impl<'a> VisionCyclerSelector<'a> {
    pub fn new(cycler: &'a mut VisionCycler) -> Self {
        Self { cycler }
    }
}

impl<'a> Widget for &mut VisionCyclerSelector<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut selection_changed = false;
        let mut combo_box = ComboBox::from_label("Cycler")
            .selected_text(self.cycler.as_path())
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(self.cycler, VisionCycler::Top, "VisionTop")
                    .clicked()
                {
                    selection_changed = true;
                };
                if ui
                    .selectable_value(self.cycler, VisionCycler::Bottom, "VisionBottom")
                    .clicked()
                {
                    selection_changed = true;
                };
            });
        if selection_changed {
            combo_box.response.mark_changed()
        }
        combo_box.response
    }
}
