use bevy::{
    picking::{
        backend::ray::{RayId, RayMap},
        pointer::PointerId,
    },
    prelude::*,
};

use super::{ball::Ball, field::FieldDropTarget, palette::WorldCamera, visual::ObjectVisualAssets};
use crate::bevy_mujoco::{MujocoWorld, SimulationMode};

#[derive(Default, Resource)]
pub struct BallSelection {
    selected: Option<Entity>,
    drag: Option<BallDrag>,
}

struct BallDrag {
    entity: Entity,
    pointer: PointerId,
    camera: Entity,
    transform: Transform,
    offset: Vec3,
    previous_mode: SimulationMode,
}

impl BallSelection {
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    fn finish_drag(&mut self, mode: &mut SimulationMode) {
        if let Some(drag) = self.drag.take() {
            *mode = drag.previous_mode;
        }
    }
}

pub struct BallInteractionPlugin;

impl Plugin for BallInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BallSelection>()
            .add_observer(select_ball)
            .add_observer(start_drag)
            .add_observer(drag_ball)
            .add_observer(end_drag)
            .add_observer(cancel_drag)
            .add_systems(Update, (release_missing_drag, highlight_selection).chain());
    }
}

fn select_ball(
    event: On<PointerPress>,
    balls: Query<(), With<Ball>>,
    field: Query<(), With<FieldDropTarget>>,
    mut selection: ResMut<BallSelection>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let entity = event.original_event_target();
    if balls.contains(entity) {
        selection.selected = Some(entity);
    } else if field.contains(entity) {
        selection.selected = None;
    }
}

fn start_drag(
    mut event: On<PointerDragStart>,
    balls: Query<&Transform, With<Ball>>,
    cameras: Query<(), With<WorldCamera>>,
    rays: Res<RayMap>,
    mut selection: ResMut<BallSelection>,
    mut mode: ResMut<SimulationMode>,
) {
    if event.button != PointerButton::Primary || selection.is_dragging() {
        return;
    }
    let entity = event.original_event_target();
    let Ok(transform) = balls.get(entity) else {
        return;
    };
    if !cameras.contains(event.hit.camera) {
        return;
    }
    let Some(ray) = rays
        .map
        .get(&RayId::new(event.hit.camera, event.pointer.id))
    else {
        return;
    };
    let Some(point) = horizontal_intersection(*ray, transform.translation.y) else {
        return;
    };
    selection.selected = Some(entity);
    selection.drag = Some(BallDrag {
        entity,
        pointer: event.pointer.id,
        camera: event.hit.camera,
        transform: *transform,
        offset: transform.translation - point,
        previous_mode: *mode,
    });
    *mode = SimulationMode::Paused;
    event.propagate(false);
}

fn drag_ball(
    mut event: On<PointerDrag>,
    rays: Res<RayMap>,
    selection: Res<BallSelection>,
    mut physics: ResMut<MujocoWorld>,
) {
    let Some(drag) = &selection.drag else {
        return;
    };
    if event.button != PointerButton::Primary
        || event.pointer.id != drag.pointer
        || event.original_event_target() != drag.entity
    {
        return;
    }
    // RayMap only contains rays inside the scene viewport, so moving across either
    // sidebar holds the last position instead of teleporting the ball behind the UI.
    if let Some(ray) = rays.map.get(&RayId::new(drag.camera, drag.pointer))
        && let Some(point) = horizontal_intersection(*ray, drag.transform.translation.y)
    {
        let mut transform = drag.transform;
        transform.translation = point + drag.offset;
        if let Err(error) = physics.set_object_pose(drag.entity, transform) {
            warn!("Could not move ball: {error}");
        }
    }
    event.propagate(false);
}

fn end_drag(
    event: On<PointerDragEnd>,
    mut selection: ResMut<BallSelection>,
    mut mode: ResMut<SimulationMode>,
) {
    if event.button == PointerButton::Primary
        && selection
            .drag
            .as_ref()
            .is_some_and(|drag| drag.pointer == event.pointer.id)
    {
        selection.finish_drag(&mut mode);
    }
}

fn cancel_drag(
    event: On<PointerCancel>,
    mut selection: ResMut<BallSelection>,
    mut mode: ResMut<SimulationMode>,
) {
    if selection
        .drag
        .as_ref()
        .is_some_and(|drag| drag.pointer == event.pointer.id)
    {
        selection.finish_drag(&mut mode);
    }
}

fn release_missing_drag(
    balls: Query<(), With<Ball>>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut selection: ResMut<BallSelection>,
    mut mode: ResMut<SimulationMode>,
) {
    if selection
        .selected
        .is_some_and(|entity| !balls.contains(entity))
    {
        selection.selected = None;
    }
    if selection.drag.as_ref().is_some_and(|drag| {
        !balls.contains(drag.entity)
            || (drag.pointer == PointerId::Mouse && !buttons.pressed(MouseButton::Left))
            || windows.iter().all(|window| !window.focused)
    }) {
        selection.finish_drag(&mut mode);
    }
}

fn highlight_selection(
    selection: Res<BallSelection>,
    assets: Res<ObjectVisualAssets>,
    mut balls: Query<(Entity, &mut MeshMaterial3d<StandardMaterial>), With<Ball>>,
) {
    for (entity, mut material) in &mut balls {
        let next = assets.ball.material(selection.selected == Some(entity));
        if material.0 != next {
            material.0 = next;
        }
    }
}

fn horizontal_intersection(ray: Ray3d, height: f32) -> Option<Vec3> {
    let distance = ray.intersect_plane(Vec3::Y * height, InfinitePlane3d::new(Vec3::Y))?;
    let point = ray.get_point(distance);
    point.is_finite().then_some(point)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragging_preserves_height_and_grab_offset() {
        let center = Vec3::new(1.0, 0.105, 2.0);
        let first = Ray3d::new(
            Vec3::new(0.0, 3.0, 8.0),
            Dir3::new(center - Vec3::new(0.1, 3.0, 8.0)).unwrap(),
        );
        let offset = center - horizontal_intersection(first, center.y).unwrap();
        let next = Ray3d::new(first.origin + Vec3::new(2.0, 0.0, -1.0), first.direction);
        let moved = horizontal_intersection(next, center.y).unwrap() + offset;
        assert!((moved - (center + Vec3::new(2.0, 0.0, -1.0))).length() < 1e-5);
        assert!(horizontal_intersection(Ray3d::new(Vec3::Y, Dir3::X), center.y).is_none());
        assert!(horizontal_intersection(Ray3d::new(Vec3::Y, Dir3::Y), center.y).is_none());
    }
}
