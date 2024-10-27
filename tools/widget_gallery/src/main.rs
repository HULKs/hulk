use eframe::{
    egui::{CentralPanel, Context},
    run_native, App, Frame,
};
use hulk_widgets::{CompletionEdit, SegmentedControl};

fn main() -> eframe::Result {
    run_native(
        "Gallery",
        Default::default(),
        Box::new(|_cc| Ok(Box::new(AppState::new()))),
    )
}

#[derive(Debug, Clone)]
struct AppState {
    searchables: Vec<String>,
    selected: String,
}

impl AppState {
    pub fn new() -> Self {
        let searchables: Vec<_> = (1..100).map(|x| x.to_string()).collect();
        Self {
            searchables,
            selected: String::new(),
        }
    }
}

impl App for AppState {
    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(context, |ui| {
            ui.horizontal(|ui| {
                let response = ui.add(CompletionEdit::new(
                    "completion-edit",
                    &self.searchables,
                    &mut self.selected,
                ));
                if response.changed() {
                    println!("Selected: {}", self.selected);
                }

                if ui.button("Focus").clicked() {
                    response.request_focus();
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.columns(2, |columns| {
                    let selectables = ["Dies", "Das", "Ananas", "Foo", "Bar", "Baz"];
                    let selected = SegmentedControl::new("segmented-control", &selectables)
                        .ui(&mut columns[0])
                        .inner;
                    columns[1].label(*selected);
                })
            })
        });
    }
}
