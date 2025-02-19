use bevy::{prelude::*, render::camera::Viewport, window::PrimaryWindow};
use bevy_egui::{
    egui::{Rect, TextEdit, Ui, Widget, WidgetText},
    EguiContext, EguiContextSettings, EguiPlugin, EguiPostUpdateSet,
};
use bevy_panorbit_camera::PanOrbitCameraSystemSet;
use egui_dock::{egui::Context, DockArea, DockState, NodeIndex, Style};

use crate::{
    async_runtime::AsyncRuntime,
    nao::{Nao, SpawnRobot},
    MainCamera,
};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .insert_resource(UiState::new())
            .add_systems(
                PostUpdate,
                show_ui
                    .before(EguiPostUpdateSet::ProcessOutput)
                    .before(bevy_egui::end_pass_system)
                    .before(bevy::transform::TransformSystem::TransformPropagate)
                    .before(PanOrbitCameraSystemSet),
            )
            .add_systems(PostUpdate, set_camera_viewport.after(show_ui));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Tab {
    World,
    Connections,
}

struct TabViewer<'a> {
    world: &'a mut World,
    viewport: &'a mut Rect,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        format!("{tab:?}").into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::World => {
                *self.viewport = ui.clip_rect();
            }
            Tab::Connections => {
                let runtime = self
                    .world
                    .get_resource::<AsyncRuntime>()
                    .unwrap()
                    .runtime
                    .handle()
                    .clone();
                let mut naos = self.world.query::<&mut Nao>();
                for mut nao in naos.iter_mut(self.world) {
                    if TextEdit::singleline(&mut nao.address)
                        .hint_text("Address")
                        .ui(ui)
                        .changed()
                    {
                        let communication = nao.client.clone();
                        let address = format!("ws://{}:1337", nao.address);
                        runtime.spawn(async move {
                            communication.set_address(address).await;
                        });
                    };
                    if ui.checkbox(&mut nao.connected, "Connect").changed() {
                        let client = nao.client.clone();
                        let connected = nao.connected;
                        runtime.spawn(async move {
                            if connected {
                                client.connect().await;
                            } else {
                                client.disconnect().await;
                            }
                        });
                    };
                }
                if ui.button("+").clicked() {
                    self.world.send_event(SpawnRobot);
                }
            }
        }
    }

    fn clear_background(&self, window: &Self::Tab) -> bool {
        !matches!(window, Tab::World)
    }

    fn allowed_in_windows(&self, tab: &mut Self::Tab) -> bool {
        !matches!(tab, Tab::World)
    }

    fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
        !matches!(tab, Tab::World | Tab::Connections)
    }
}

#[derive(Resource)]
struct UiState {
    state: DockState<Tab>,
    viewport: Rect,
}

impl UiState {
    fn new() -> Self {
        let mut state = DockState::new(vec![Tab::World]);
        let tree = state.main_surface_mut();
        tree.split_right(NodeIndex::root(), 0.8, vec![Tab::Connections]);

        Self {
            viewport: Rect::NOTHING,
            state,
        }
    }

    fn ui(&mut self, world: &mut World, ctx: &mut Context) {
        let mut tab_viewer = TabViewer {
            world,
            viewport: &mut self.viewport,
        };
        DockArea::new(&mut self.state)
            .show_add_buttons(true)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut tab_viewer);
    }
}

fn show_ui(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    world.resource_scope::<UiState, _>(|world, mut ui_state| {
        ui_state.ui(world, egui_context.get_mut())
    });
}

fn set_camera_viewport(
    ui_state: Res<UiState>,
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    egui_settings: Query<&EguiContextSettings>,
    mut cameras: Query<&mut Camera, With<MainCamera>>,
) {
    let mut cam = cameras.single_mut();

    let Ok(window) = primary_window.get_single() else {
        return;
    };

    let scale_factor = window.scale_factor() * egui_settings.single().scale_factor;

    let viewport_pos = ui_state.viewport.left_top().to_vec2() * scale_factor;
    let viewport_size = ui_state.viewport.size() * scale_factor;

    let physical_position = UVec2::new(viewport_pos.x as u32, viewport_pos.y as u32);
    let physical_size = UVec2::new(viewport_size.x as u32, viewport_size.y as u32);

    // The desired viewport rectangle at its offset in "physical pixel space"
    let rect = physical_position + physical_size;

    let window_size = window.physical_size();
    // wgpu will panic if trying to set a viewport rect which has coordinates extending
    // past the size of the render target, i.e. the physical window in our case.
    // Typically this shouldn't happen- but during init and resizing etc. edge cases might occur.
    // Simply do nothing in those cases.
    if rect.x <= window_size.x && rect.y <= window_size.y {
        cam.viewport = Some(Viewport {
            physical_position,
            physical_size,
            depth: 0.0..1.0,
        });
    }
}
