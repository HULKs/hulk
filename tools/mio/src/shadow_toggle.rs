use bevy::prelude::*;

pub struct ShadowTogglePlugin;

#[derive(Resource)]
pub struct ShadowState {
    pub enabled: bool,
}

impl Plugin for ShadowTogglePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, toggle_shadows)
            .insert_resource(ShadowState { enabled: true });
    }
}

fn toggle_shadows(
    shadow_state: Res<ShadowState>,
    mut directional_lights: Query<&mut DirectionalLight>,
    mut point_lights: Query<&mut PointLight>,
) {
    if !shadow_state.is_changed() {
        return;
    }

    for mut light in directional_lights.iter_mut() {
        light.shadows_enabled = shadow_state.enabled;
    }

    for mut light in point_lights.iter_mut() {
        light.shadows_enabled = shadow_state.enabled;
    }
}
