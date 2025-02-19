use eframe::{
    egui::{CentralPanel, Context, Widget},
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
    selected: usize,
    search_text: String,
}

impl AppState {
    pub fn new() -> Self {
        let searchables: Vec<_> = (1..100).map(|x| x.to_string()).collect();
        Self {
            searchables,
            selected: 0,
            search_text: String::new(),
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
                    &mut self.search_text,
                ));
                if response.changed() {
                    println!("Selected: {}", self.search_text);
                }

                if ui.button("Focus").clicked() {
                    response.request_focus();
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.columns(2, |columns| {
                    let selectables = ["Dies", "Das", "Ananas", "Foo", "Bar", "Baz"];
                    SegmentedControl::new("segmented-control", &mut self.selected, &selectables)
                        .ui(&mut columns[0]);

                    columns[1].label(selectables[self.selected]);
                })
            })
        });
    }
}
