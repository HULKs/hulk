use eframe::{
    App, Frame,
    egui::{
        Button, CentralPanel, Key, KeyboardShortcut, Modifiers, Popup, PopupCloseBehavior, Ui, vec2,
    },
    run_native,
};
use hulk_widgets::{CompletionEdit, KeybindPreview, SearchableSelector, SegmentedControl};

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
    selector_selected: Option<usize>,
}

impl AppState {
    pub fn new() -> Self {
        let searchables: Vec<_> = (1..100).map(|x| x.to_string()).collect();
        Self {
            searchables,
            selected: 0,
            search_text: String::new(),
            selector_selected: None,
        }
    }
}

impl App for AppState {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        CentralPanel::default().show(ui, |ui| {
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
                ui.label("Searchable selector");
                let popup_id = ui.make_persistent_id("searchable-selector-popup");
                let popup_was_open = Popup::is_id_open(ui.ctx(), popup_id);
                let button_text = self
                    .selector_selected
                    .and_then(|index| self.searchables.get(index))
                    .map_or_else(|| "Choose a number".to_owned(), |item| item.clone());
                let button = ui.button(button_text);
                let reset_on_show = button.clicked() && !popup_was_open;

                let selected = Popup::from_toggle_button_response(&button)
                    .id(popup_id)
                    .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                    .width(220.0)
                    .show(|ui| {
                        SearchableSelector::new(popup_id.with("selector"), &self.searchables)
                            .reset_on_show(reset_on_show)
                            .show(ui, String::as_str, |ui, highlighted, item| {
                                ui.add(
                                    Button::selectable(highlighted, item)
                                        .min_size(vec2(ui.available_width(), 28.0)),
                                )
                            })
                    })
                    .and_then(|response| response.inner);

                if let Some(index) = selected {
                    self.selector_selected = Some(index);
                    Popup::close_id(ui.ctx(), popup_id);
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.columns(2, |columns| {
                    let selectables = ["Dies", "Das", "Ananas", "Foo", "Bar", "Baz"];
                    columns[0].add(SegmentedControl::new(
                        "segmented-control",
                        &mut self.selected,
                        &selectables,
                    ));

                    columns[1].label(selectables[self.selected]);
                })
            });

            ui.group(|ui| {
                ui.add(KeybindPreview(KeyboardShortcut::new(
                    Modifiers::ALT.plus(Modifiers::SHIFT),
                    Key::ArrowDown,
                )));
            });
        });
    }
}
