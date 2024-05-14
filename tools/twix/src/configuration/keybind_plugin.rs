use std::sync::Arc;

use eframe::egui::{Context, Id, InputState};

use super::keys::{KeybindAction, Keybinds};

type ActionList = Arc<Vec<KeybindAction>>;

pub fn register(ctx: &Context) {
    ctx.on_begin_frame("keybinds", Arc::new(begin_frame))
}

fn begin_frame(ctx: &Context) {
    if let Some(keybinds) = ctx.data(|data| data.get_temp::<Arc<Keybinds>>(Id::NULL)) {
        let actions = ctx.input_mut(|input| read_actions(keybinds, input));

        ctx.data_mut(|data| data.insert_temp::<ActionList>(Id::NULL, Arc::new(actions)))
    }
}

fn read_actions(keybinds: Arc<Keybinds>, input: &mut InputState) -> Vec<KeybindAction> {
    let mut actions = Vec::new();

    input.events.retain(|event| {
        if let eframe::egui::Event::Key {
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

pub fn set_keybinds(ctx: &Context, keybinds: Arc<Keybinds>) {
    ctx.data_mut(|data| data.insert_temp(Id::NULL, keybinds));
}

pub fn keybind_pressed(ctx: &Context, action: KeybindAction) -> bool {
    ctx.data(|data| {
        data.get_temp::<ActionList>(Id::NULL)
            .is_some_and(|actions| actions.contains(&action))
    })
}
