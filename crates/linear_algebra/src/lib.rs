//! # linear_algebra
//!
//! A crate for frame-safe linear algebra, wrapping [`nalgebra`] types with coordinate system tags.
//!
//! ## Motivation
//!
//! Geometric computations often involve multiple coordinate systems. Mixing up these systems can
//! lead to subtle and hard-to-find bugs. By encoding coordinate systems in types, this crate
//! ensures that only compatible operations are allowed, catching errors at compile time.
//!
//! ## Features
//!
//! - Enforces coordinate system correctness at compile time using Rust's type system.
//! - Separates Vectors from Points, Isometries from Poses, etc.
//! - Wraps commonly used parts of the [`nalgebra`] API to provide frame-safe abstractions.
//! - Supports 2D and 3D geometry with extensible coordinate system tagging.
//! - Provides clear and explicit geometric transformations.
//!
//! ## Example
//!
//! ```rust
//! use linear_algebra::{point, vector, Isometry3, Orientation3, Point3};
//!
//! struct Camera;
//! struct Ground;
//!
//! fn ball_to_ground(
//!     position: Point3<Camera>,
//!     camera_to_ground: Isometry3<Camera, Ground>,
//! ) -> Point3<Ground> {
//!     camera_to_ground * position
//! }
//!
//! let position_in_camera: Point3<Camera> = point![1.0, 2.0, 3.0];
//! let camera_to_ground =
//!     Isometry3::<Camera, Ground>::from_parts(vector![0.0, 0.0, 1.0], Orientation3::identity());
//! let position_in_ground: Point3<Ground> = ball_to_ground(position_in_camera, camera_to_ground);
//! ```
//!
//! ## Philosophy
//!
//! This crate is a thin, zero-cost wrapper around [`nalgebra`], adding type-level tags for
//! coordinate systems using [`Framed`] and [`Transform`]. It does not reimplement linear algebra,
//! but provides a safer API for geometric programming.
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
pub use point::{center, distance, distance_squared, Point, Point2, Point3};
pub use pose::{Pose2, Pose3};
pub use rotation::{Rotation2, Rotation3};
pub use transform::Transform;
pub use vector::{Vector, Vector2, Vector3};
