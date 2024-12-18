use std::sync::Arc;

use eframe::egui::{Context, Event, Id, InputState};

use super::keys::{KeybindAction, Keybinds};

type ActionList = Arc<Vec<KeybindAction>>;

pub fn register(ctx: &Context) {
    ctx.on_begin_pass("keybinds", Arc::new(begin_frame))
}

fn begin_frame(ctx: &Context) {
    if let Some(keybinds) = ctx.data(|data| data.get_temp::<Arc<Keybinds>>(Id::NULL)) {
        let actions = ctx.input_mut(|input| consume_actions(keybinds, input));

        ctx.data_mut(|data| data.insert_temp::<ActionList>(Id::NULL, Arc::new(actions)))
    }
}

fn consume_actions(keybinds: Arc<Keybinds>, input: &mut InputState) -> Vec<KeybindAction> {
    let mut actions = Vec::new();

    input.events.retain(|event| {
        if let Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } = event
        {
            for (trigger, action) in keybinds.iter() {
                if trigger.key == *key && modifiers.matches_exact(trigger.modifiers) {
                    actions.push(*action);
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
}

impl KeybindSystem for Context {
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
