use bevy::{
    asset::RenderAssetUsages,
    camera::{RenderTarget, ScalingMode, Viewport, visibility::RenderLayers},
    picking::mesh_picking::MeshPickingCamera,
    prelude::*,
    render::render_resource::{TextureDimension, TextureFormat, TextureUsages},
    ui::widget::ViewportNode,
    window::PrimaryWindow,
};

use super::{
    field::FieldDropTarget,
    object::{self, ObjectKind},
    visual::ObjectVisualAssets,
};

const SIDEBAR_WIDTH: f32 = 240.0;
const PALETTE_ITEMS: [ObjectKind; 2] = [ObjectKind::Ball, ObjectKind::Robot];

#[derive(Component)]
struct PaletteItem(ObjectKind);

#[derive(Component)]
struct DropPreview(ObjectKind);

#[derive(Component)]
struct ThumbnailPreview;

#[derive(Component)]
pub struct WorldCamera;

pub struct ObjectPalettePlugin;

impl Plugin for ObjectPalettePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_palette)
            .add_systems(Update, (update_world_viewport, rotate_thumbnails))
            .add_observer(on_drag_start)
            .add_observer(on_drag_end)
            .add_observer(on_drag_over)
            .add_observer(on_drag_leave)
            .add_observer(on_drag_drop);
    }
}

fn setup_palette(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    assets: Res<ObjectVisualAssets>,
) {
    let camera = commands
        .spawn((
            Camera2d,
            IsDefaultUiCamera,
            Camera {
                order: 1,
                clear_color: ClearColorConfig::None,
                ..default()
            },
        ))
        .id();

    let thumbnails = PALETTE_ITEMS.map(|kind| {
        let layer = kind as usize + 1;
        let layers = RenderLayers::layer(layer);
        let root = assets.spawn_preview(
            kind,
            &mut commands,
            Transform::IDENTITY,
            false,
            layers.clone(),
        );
        commands.entity(root).insert(ThumbnailPreview);

        let mut image = Image::new_uninit(
            default(),
            TextureDimension::D2,
            TextureFormat::Bgra8UnormSrgb,
            RenderAssetUsages::all(),
        );
        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST
            | TextureUsages::RENDER_ATTACHMENT;
        let image = images.add(image);
        let center = assets.preview_center(kind);
        let camera = commands
            .spawn((
                Camera3d::default(),
                Camera {
                    order: -1,
                    clear_color: ClearColorConfig::Custom(Color::srgb(0.075, 0.09, 0.115)),
                    ..default()
                },
                RenderTarget::Image(image.into()),
                Projection::Orthographic(OrthographicProjection {
                    scaling_mode: ScalingMode::FixedVertical {
                        viewport_height: assets.preview_height(kind),
                    },
                    ..OrthographicProjection::default_3d()
                }),
                Transform::from_translation(center + Vec3::new(2.0, 1.2, 3.0))
                    .looking_at(center, Vec3::Y),
                layers,
            ))
            .id();
        (kind, camera)
    });

    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: false,
            illuminance: 4_000.0,
            ..default()
        },
        Transform::from_xyz(2.0, 4.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::from_layers(&[1, 2]),
    ));

    commands
        .spawn((
            UiTargetCamera(camera),
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(SIDEBAR_WIDTH),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    padding: UiRect::all(px(16)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.055, 0.065, 0.08)),
                Pickable::IGNORE,
            ))
            .with_children(|sidebar| {
                sidebar.spawn((
                    Text::new("OBJECTS"),
                    TextFont::from_font_size(13.0),
                    TextColor(Color::srgb(0.45, 0.55, 0.68)),
                    Node {
                        margin: UiRect::bottom(px(4)),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));

                for (kind, camera) in thumbnails {
                    sidebar
                        .spawn((
                            PaletteItem(kind),
                            Node {
                                width: percent(100),
                                height: px(92),
                                align_items: AlignItems::Center,
                                column_gap: px(10),
                                padding: UiRect::horizontal(px(14)),
                                border: UiRect::all(px(1)),
                                border_radius: BorderRadius::all(px(5)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.10, 0.125, 0.16)),
                            BorderColor::all(Color::srgb(0.16, 0.20, 0.26)),
                        ))
                        .insert(Pickable::default())
                        .with_children(|card| {
                            card.spawn((
                                ViewportNode::new(camera),
                                Node {
                                    width: px(68),
                                    height: px(68),
                                    flex_shrink: 0.0,
                                    border_radius: BorderRadius::all(px(4)),
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ));
                            card.spawn((
                                Text::new(kind.label()),
                                TextFont::from_font_size(17.0),
                                TextColor(Color::srgb(0.88, 0.92, 0.96)),
                                Pickable::IGNORE,
                            ));
                        });
                }

                sidebar.spawn((
                    Text::new("Drag an object onto the field.\n\nClick a ball to select it, then drag to move it. Physics pauses during dragging.\n\nClick the field to deselect."),
                    TextFont::from_font_size(12.0),
                    TextColor(Color::srgb(0.38, 0.44, 0.52)),
                    Node {
                        margin: UiRect::top(px(6)),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            });
        });
}

fn on_drag_start(
    mut event: On<PointerDragStart>,
    items: Query<(), With<PaletteItem>>,
    mut colors: Query<&mut BackgroundColor>,
) {
    let entity = event.original_event_target();
    if !items.contains(entity) {
        return;
    }
    if let Ok(mut color) = colors.get_mut(entity) {
        color.0 = Color::srgb(0.16, 0.25, 0.36);
    }
    event.propagate(false);
}

fn on_drag_end(
    mut event: On<PointerDragEnd>,
    items: Query<(), With<PaletteItem>>,
    mut colors: Query<&mut BackgroundColor>,
    preview: Query<Entity, With<DropPreview>>,
    mut commands: Commands,
) {
    let entity = event.original_event_target();
    if !items.contains(entity) {
        return;
    }
    if let Ok(mut color) = colors.get_mut(entity) {
        color.0 = Color::srgb(0.10, 0.125, 0.16);
    }
    for entity in &preview {
        commands.entity(entity).try_despawn();
    }
    event.propagate(false);
}

fn on_drag_over(
    mut event: On<PointerDragOver>,
    targets: Query<(), With<FieldDropTarget>>,
    items: Query<&PaletteItem>,
    mut preview: Query<(Entity, &DropPreview, &mut Transform)>,
    assets: Res<ObjectVisualAssets>,
    mut commands: Commands,
) {
    if !targets.contains(event.original_event_target()) {
        return;
    }
    let Ok(item) = items.get(event.dragged) else {
        return;
    };
    let Some(mut position) = event.hit.position else {
        return;
    };
    position.y += assets.ground_offset(item.0);

    if let Some((entity, preview, mut transform)) = preview.iter_mut().next() {
        if preview.0 == item.0 {
            transform.translation = position;
            event.propagate(false);
            return;
        }
        commands.entity(entity).try_despawn();
    }

    let preview = assets.spawn_preview(
        item.0,
        &mut commands,
        Transform::from_translation(position),
        true,
        RenderLayers::layer(0),
    );
    commands.entity(preview).insert(DropPreview(item.0));
    event.propagate(false);
}

fn on_drag_leave(
    mut event: On<PointerDragLeave>,
    targets: Query<(), With<FieldDropTarget>>,
    items: Query<(), With<PaletteItem>>,
    preview: Query<Entity, With<DropPreview>>,
    mut commands: Commands,
) {
    if targets.contains(event.original_event_target()) && items.contains(event.dragged) {
        for entity in &preview {
            commands.entity(entity).try_despawn();
        }
        event.propagate(false);
    }
}

fn on_drag_drop(
    mut event: On<PointerDragDrop>,
    targets: Query<(), With<FieldDropTarget>>,
    items: Query<&PaletteItem>,
    preview: Query<Entity, With<DropPreview>>,
    assets: Res<ObjectVisualAssets>,
    mut commands: Commands,
) {
    if !targets.contains(event.original_event_target()) {
        return;
    }
    let Ok(item) = items.get(event.dropped) else {
        return;
    };
    let Some(position) = event.hit.position else {
        return;
    };

    object::spawn(
        item.0,
        &mut commands,
        &assets,
        Transform::from_xyz(position.x, 0.0, position.z),
    );
    for entity in &preview {
        commands.entity(entity).try_despawn();
    }
    event.propagate(false);
}

fn rotate_thumbnails(
    time: Res<Time>,
    mut thumbnails: Query<&mut Transform, With<ThumbnailPreview>>,
) {
    for mut transform in &mut thumbnails {
        transform.rotate_y(0.35 * time.delta_secs());
    }
}

fn update_world_viewport(
    window: Single<&Window, With<PrimaryWindow>>,
    mut camera: Single<&mut Camera, (With<WorldCamera>, With<MeshPickingCamera>)>,
) {
    camera.viewport = Some(world_viewport(
        window.physical_size(),
        window.resolution.scale_factor(),
    ));
}

fn world_viewport(size: UVec2, scale_factor: f32) -> Viewport {
    let sidebar = (SIDEBAR_WIDTH * scale_factor).round() as u32;
    let left = sidebar.min(size.x.saturating_sub(1));
    Viewport {
        physical_position: UVec2::new(left, 0),
        physical_size: UVec2::new(
            size.x
                .saturating_sub(left + (crate::controls::PANEL_WIDTH * scale_factor).round() as u32)
                .max(1),
            size.y.max(1),
        ),
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_reserves_scaled_sidebar_width() {
        let viewport = world_viewport(UVec2::new(1920, 1080), 1.5);

        assert_eq!(viewport.physical_position, UVec2::new(360, 0));
        assert_eq!(viewport.physical_size, UVec2::new(840, 1080));
    }

    #[test]
    fn palette_excludes_goals() {
        assert_eq!(PALETTE_ITEMS, [ObjectKind::Ball, ObjectKind::Robot]);
        assert!(!PALETTE_ITEMS.contains(&ObjectKind::Goal));
    }
}
