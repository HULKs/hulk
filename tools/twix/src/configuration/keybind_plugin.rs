use std::sync::Arc;

use eframe::egui::{Context, Event, Id, InputState, Ui};

use super::keys::{KeybindAction, Keybinds};

type ActionList = Arc<Vec<KeybindAction>>;

pub fn register(ctx: &Context) {
    ctx.on_begin_pass("keybinds", Arc::new(begin_frame))
}

fn begin_frame(ui: &mut Ui) {
    let ctx = ui.ctx();

    if let Some(keybinds) = ctx.data(|data| data.get_temp::<Arc<Keybinds>>(Id::NULL)) {
        let text_edit_focused = ctx.text_edit_focused();
        let actions =
            ctx.input_mut(|input| consume_actions(&keybinds, input, false, text_edit_focused));

        ctx.data_mut(|data| data.insert_temp::<ActionList>(Id::NULL, Arc::new(actions)));
    }
}

fn consume_actions(
    keybinds: &Keybinds,
    input: &mut InputState,
    timeline: bool,
    text_edit_focused: bool,
) -> Vec<KeybindAction> {
    let mut actions = Vec::new();

    input.events.retain(|event| {
        if let Event::Key {
            key,
            pressed: true,
            modifiers,
            repeat,
            ..
        } = event
        {
            for (trigger, action) in keybinds.iter() {
                if action.is_timeline() == timeline
                    && !(text_edit_focused && trigger.conflicts_with_text_editing())
                    && trigger.logical_key == *key
                    && modifiers.matches_exact(trigger.modifiers)
                {
                    if !*repeat || *action != KeybindAction::TimelineTogglePlayback {
                        actions.push(*action);
                    }
                    return false;
                }
            }
        }
        true
    });

    actions
}

pub trait KeybindSystem {
    fn keybind_pressed(&self, action: KeybindAction) -> bool;
    fn set_keybinds(&self, keybinds: Arc<Keybinds>);
    fn consume_timeline_actions(&self) -> Vec<KeybindAction>;
    fn keybind_label(&self, action: KeybindAction) -> String;
}

impl KeybindSystem for Context {
    fn consume_timeline_actions(&self) -> Vec<KeybindAction> {
        let keybinds = self.data(|data| data.get_temp::<Arc<Keybinds>>(Id::NULL));
        keybinds.map_or_else(Vec::new, |keybinds| {
            let text_edit_focused = self.text_edit_focused();
            self.input_mut(|input| consume_actions(&keybinds, input, true, text_edit_focused))
        })
    }

    fn keybind_label(&self, action: KeybindAction) -> String {
        let keybinds = self.data(|data| data.get_temp::<Arc<Keybinds>>(Id::NULL));
        let Some(keybinds) = keybinds else {
            return String::new();
        };
        let mut labels: Vec<_> = keybinds
            .iter()
            .filter(|(_, configured)| **configured == action)
            .map(|(shortcut, _)| self.format_shortcut(shortcut))
            .collect();
        labels.sort();
        labels.join(", ")
    }

    fn set_keybinds(&self, keybinds: Arc<Keybinds>) {
        self.data_mut(|data| data.insert_temp(Id::NULL, keybinds));
    }

    fn keybind_pressed(&self, action: KeybindAction) -> bool {
        self.data(|data| {
            data.get_temp::<ActionList>(Id::NULL)
                .is_some_and(|actions| actions.contains(&action))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Key, Modifiers};

    #[test]
    fn global_dispatch_leaves_scoped_keys_for_one_local_consumer() {
        let keybinds: Keybinds = toml::from_str(
            r#"
            C-t = "open_split"
            Space = "timeline_toggle_playback"
        "#,
        )
        .unwrap();
        let mut input = InputState::default();
        input.events = [(Key::T, Modifiers::COMMAND), (Key::Space, Modifiers::NONE)]
            .into_iter()
            .map(|(key, modifiers)| Event::Key {
                key,
                modifiers,
                physical_key: None,
                pressed: true,
                repeat: false,
            })
            .collect();
        assert_eq!(
            consume_actions(&keybinds, &mut input, false, false),
            [KeybindAction::OpenSplit]
        );
        assert_eq!(
            input.events.len(),
            1,
            "unfocused widgets must still receive Space"
        );
        assert_eq!(
            consume_actions(&keybinds, &mut input, true, false),
            [KeybindAction::TimelineTogglePlayback]
        );
        assert!(consume_actions(&keybinds, &mut input, true, false).is_empty());
    }

    #[test]
    fn focused_text_edit_keeps_editing_keys_but_allows_nonconflicting_shortcuts() {
        let keybinds: Keybinds = toml::from_str(
            r#"
            C-t = "open_split"
            A-ArrowLeft = "focus_left"
            Space = "timeline_toggle_playback"
        "#,
        )
        .unwrap();
        let mut input = InputState::default();
        input.events = [
            (Key::T, Modifiers::COMMAND),
            (Key::ArrowLeft, Modifiers::ALT),
            (Key::Space, Modifiers::NONE),
        ]
        .into_iter()
        .map(|(key, modifiers)| Event::Key {
            key,
            modifiers,
            physical_key: None,
            pressed: true,
            repeat: false,
        })
        .collect();

        assert_eq!(
            consume_actions(&keybinds, &mut input, false, true),
            [KeybindAction::OpenSplit]
        );
        assert!(consume_actions(&keybinds, &mut input, true, true).is_empty());
        assert_eq!(input.events.len(), 2);
        assert!(matches!(
            input.events[0],
            Event::Key {
                key: Key::ArrowLeft,
                ..
            }
        ));
        assert!(matches!(
            input.events[1],
            Event::Key {
                key: Key::Space,
                ..
            }
        ));
    }
}
