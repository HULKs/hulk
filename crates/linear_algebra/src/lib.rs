//! # linear_algebra
//!
//! A crate for frame-safe linear algebra built on top of [`nalgebra`].
//!
//! ## Motivation
//!
//! Geometric computations often involve multiple coordinate systems. Mixing up these systems can
//! lead to subtle and hard-to-find bugs. By encoding coordinate systems in types, this crate
//! ensures that only compatible operations are allowed, catching errors at compile time.
//!
//! [`nalgebra`] still provides the underlying point, vector, rotation, and isometry math. This
//! crate adds frame-safe wrappers and transforms that encode coordinate systems in Rust types so
//! incompatible spaces cannot be mixed by accident.
//!
//! A coordinate system can be any marker type chosen by the user. A [`Pose2`] or [`Pose3`]
//! represents an object's position and orientation inside a single coordinate system, while an
//! [`Isometry2`] or [`Isometry3`] transforms values from one coordinate system into another.
//!
//! ## Features
//!
//! - Reuses nalgebra's point, vector, rotation, and isometry math.
//! - Adds frame-safe wrappers and transforms that encode coordinate systems in types.
//! - Separates vectors from points, and poses from transforms, so conversions are explicit.
//! - Re-exports [`nalgebra`] so `point!` and `vector!` work without a direct dependency.
//!
//! ## Example
//!
//! ```rust
//! use linear_algebra::{point, vector, Isometry3, Orientation3, Point3};
//!
//! struct Camera;
//! struct Ground;
//!
//! fn ball_in_ground(
//!     position: Point3<Camera>,
//!     camera_to_ground: Isometry3<Camera, Ground>,
//! ) -> Point3<Ground> {
//!     camera_to_ground * position
//! }
//!
//! let position_in_camera: Point3<Camera> = point![1.0, 2.0, 3.0];
//! let camera_to_ground =
//!     Isometry3::<Camera, Ground>::from_parts(vector![0.0, 0.0, 1.0], Orientation3::identity());
//! let position_in_ground: Point3<Ground> = ball_in_ground(position_in_camera, camera_to_ground);
//! ```
//!
//! ## Philosophy
//!
//! This crate is a thin, zero-cost wrapper around [`nalgebra`]. nalgebra defines the core math and
//! data structures, while this crate adds type-level coordinate system tags through [`Framed`] and
//! [`Transform`] and a small set of frame-aware convenience APIs.
//!
//! [`nalgebra`]: https://docs.rs/nalgebra

mod framed;
mod into;
mod isometry;
mod orientation;
mod point;
mod pose;
mod rotation;
mod transform;
mod vector;

pub use framed::Framed;
pub use into::{IntoFramed, IntoTransform};
pub use isometry::{Isometry, Isometry2, Isometry3};
pub use orientation::{Orientation2, Orientation3};
pub use point::{Point, Point2, Point3, center, distance, distance_squared};
pub use pose::{Pose2, Pose3};
pub use rotation::{Rotation2, Rotation3};
pub use transform::Transform;
pub use vector::{Vector, Vector2, Vector3};

/// Re-exported so crate macros and examples can use `nalgebra` without requiring a direct
/// dependency in downstream crates.
pub use nalgebra;
