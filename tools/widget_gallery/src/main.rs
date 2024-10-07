use eframe::{
    egui::{CentralPanel, Context, Id},
    run_native, run_simple_native, Frame, NativeOptions,
};
use hulk_widgets::completion_edit::CompletionEdit;

fn main() -> eframe::Result {
    run_simple_native("Gallery", Default::default(), show)
}

#[derive(Debug, Clone)]
struct AppState {
    searchables: Vec<String>,
}

impl AppState {
    pub fn new() -> Self {
        let searchables: Vec<_> = (1..100).map(|x| x.to_string()).collect();
        Self { searchables }
    }
}

fn show(context: &Context, _frame: &mut Frame) {
    let app_state = match context.data(|reader| reader.get_temp::<AppState>(Id::NULL)) {
        Some(app_state) => app_state,
        None => {
            let app_state = AppState::new();
            context.data_mut(|writer| writer.insert_temp(Id::NULL, app_state.clone()));
            app_state
        }
    };

    CentralPanel::default().show(context, |ui| {
        let mut selected = None;
        ui.horizontal(|ui| {
            ui.add(CompletionEdit::new(
                "completion-edit",
                &app_state.searchables,
                &mut selected,
            ));
            if let Some(selected) = selected {
                ui.label(format!("Selected: {}", selected));
            }
        });
    });
}
