use eframe::egui::{Grid, Key, KeyboardShortcut, Modifiers, Response, Ui, Widget};
use hulk_widgets::KeybindPreview;

pub struct Controls {
    pub create_bookmark: KeyboardShortcut,
    pub delete_bookmark: KeyboardShortcut,
    pub play_pause: KeyboardShortcut,

    pub bookmark: ForwardBackward,
    pub jump_large: ForwardBackward,
    pub jump_small: ForwardBackward,
    pub step: ForwardBackward,
}

pub struct ForwardBackward {
    pub forward: KeyboardShortcut,
    pub backward: KeyboardShortcut,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            create_bookmark: KeyboardShortcut::new(Modifiers::NONE, Key::B),
            delete_bookmark: KeyboardShortcut::new(Modifiers::CTRL, Key::B),
            play_pause: KeyboardShortcut::new(Modifiers::NONE, Key::Space),
            bookmark: ForwardBackward {
                forward: KeyboardShortcut::new(Modifiers::NONE, Key::PageDown),
                backward: KeyboardShortcut::new(Modifiers::NONE, Key::PageUp),
            },
            jump_large: ForwardBackward {
                forward: KeyboardShortcut::new(Modifiers::NONE, Key::L),
                backward: KeyboardShortcut::new(Modifiers::NONE, Key::J),
            },
            jump_small: ForwardBackward {
                forward: KeyboardShortcut::new(Modifiers::NONE, Key::ArrowRight),
                backward: KeyboardShortcut::new(Modifiers::NONE, Key::ArrowLeft),
            },
            step: ForwardBackward {
                forward: KeyboardShortcut::new(Modifiers::NONE, Key::Period),
                backward: KeyboardShortcut::new(Modifiers::NONE, Key::Comma),
            },
        }
    }
}

fn create_row(ui: &mut Ui, name: &str, shortcut: KeyboardShortcut) {
    ui.add(KeybindPreview(shortcut));
    ui.label(name);
    ui.end_row();
}

impl Widget for &Controls {
    fn ui(self, ui: &mut Ui) -> Response {
        Grid::new("shortcuts")
            .num_columns(2)
            .show(ui, |ui| {
                create_row(ui, "Play/Pause", self.play_pause);
                create_row(ui, "Create bookmark", self.create_bookmark);
                create_row(ui, "Delete bookmark", self.delete_bookmark);
                create_row(ui, "Next bookmark", self.bookmark.forward);
                create_row(ui, "Previous bookmark", self.bookmark.backward);
                create_row(ui, "Step forward", self.step.forward);
                create_row(ui, "Step backward", self.step.backward);
                create_row(ui, "Jump forward", self.jump_small.forward);
                create_row(ui, "Jump backward", self.jump_small.backward);
                create_row(ui, "Large jump forward", self.jump_large.forward);
                create_row(ui, "Large jump backward", self.jump_large.backward);
            })
            .response
    }
}
