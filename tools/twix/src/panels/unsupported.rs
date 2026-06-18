use eframe::egui::{Response, Ui, Widget};
use serde_json::{Value, json};

pub struct UnsupportedPanel {
    panel_type: String,
    saved_value: Value,
    reason: String,
}

impl UnsupportedPanel {
    pub fn new(panel_type: impl Into<String>, saved_value: Option<&Value>) -> Self {
        let panel_type = panel_type.into();
        let saved_value = saved_value
            .cloned()
            .unwrap_or_else(|| json!({ "_panel_type": panel_type.clone() }));
        Self {
            panel_type: panel_type.clone(),
            saved_value,
            reason: format!(
                "'{panel_type}' is unsupported on the read-only ros-z backend in this milestone"
            ),
        }
    }

    pub fn save(&self) -> Value {
        self.saved_value.clone()
    }

    pub fn title(&self) -> &str {
        &self.panel_type
    }
}

impl Widget for &mut UnsupportedPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            ui.heading(self.title());
            ui.label(&self.reason);
            ui.label("Pick Text, Plot, Enum Plot, or Map for native read-only ros-z inspection.");
        })
        .response
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn save_preserves_saved_panel_state_with_original_panel_type() {
        let saved = json!({
            "_panel_type": "Remote",
            "selected": "old-control"
        });
        let panel = UnsupportedPanel::new("Remote", Some(&saved));

        assert_eq!(
            panel.save(),
            json!({
                "_panel_type": "Remote",
                "selected": "old-control"
            })
        );
    }
}
