//! Robot-sized moving occluders shared by headless physics and the tuning viewer.
use bevy::prelude::*;
use mujoco_rs::prelude::{MjSpec, MjtGeom, SpecItem};

use crate::bevy_mujoco::MjcfObject;

pub(crate) const RADIUS: f32 = 0.22;
pub(crate) const HEIGHT: f32 = 0.85;
pub(crate) const COUNT: usize = 2;

pub(crate) fn positions(seconds: f64, seed: u64) -> [[f64; 3]; COUNT] {
    let side = if seed.is_multiple_of(2) { 1.0 } else { -1.0 };
    [
        [
            2.5 + 0.3 * (0.3 * seconds).sin(),
            side * 1.5 * (0.7 * seconds).cos(),
            f64::from(HEIGHT / 2.0),
        ],
        [
            -0.8 + 1.4 * (0.4 * seconds).sin(),
            side * (-1.1 + 0.7 * (0.6 * seconds).sin()),
            f64::from(HEIGHT / 2.0),
        ],
    ]
}

pub(crate) fn object() -> MjcfObject {
    MjcfObject::from_factory(
        || {
            let mut spec = MjSpec::new();
            let body = spec.world_body_mut().add_body();
            body.set_name("opponent")
                .map_err(|error| error.to_string())?;
            body.with_mocap(true);
            body.add_geom()
                .with_type(MjtGeom::mjGEOM_CYLINDER)
                .with_size([f64::from(RADIUS), f64::from(HEIGHT / 2.0), 0.0])
                .with_friction([0.7, 0.005, 0.0001]);
            Ok(spec)
        },
        "opponent",
    )
    .with_mocap_body("opponent")
}

pub(crate) fn transform([x, y, z]: [f64; 3]) -> Transform {
    Transform::from_xyz(x as f32, z as f32, -y as f32)
}
