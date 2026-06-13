use std::f32::consts::PI;

use coordinate_systems::{Field, Ground, World};
use linear_algebra::{Isometry2, Point2, Vector2};
use types::field_dimensions::GlobalFieldSide;

pub(crate) fn world_to_field_transform(
    global_field_side: GlobalFieldSide,
) -> Isometry2<World, Field> {
    match global_field_side {
        GlobalFieldSide::Home => Isometry2::identity(),
        GlobalFieldSide::Away => Isometry2::from_parts(Vector2::zeros(), PI),
    }
}

pub(crate) fn ground_to_field_from_world(
    ground_to_world: Isometry2<Ground, World>,
    global_field_side: GlobalFieldSide,
) -> Isometry2<Ground, Field> {
    world_to_field_transform(global_field_side) * ground_to_world
}

pub(crate) fn point_world_to_field(
    point: Point2<World>,
    global_field_side: GlobalFieldSide,
) -> Point2<Field> {
    world_to_field_transform(global_field_side) * point
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use linear_algebra::point;

    #[test]
    fn world_to_field_is_identity_for_home_side() {
        let point_in_field = point_world_to_field(point![1.0, -0.5], GlobalFieldSide::Home);

        assert_relative_eq!(point_in_field.x(), 1.0);
        assert_relative_eq!(point_in_field.y(), -0.5);
    }

    #[test]
    fn world_to_field_flips_for_away_side() {
        let point_in_field = point_world_to_field(point![1.0, -0.5], GlobalFieldSide::Away);

        assert_relative_eq!(point_in_field.x(), -1.0);
        assert_relative_eq!(point_in_field.y(), 0.5);
    }
}
