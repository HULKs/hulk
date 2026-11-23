//! Coordinate-frame labels for [`nalgebra`] values.
//!
//! [`Framed`] labels a value with the coordinate frame it is expressed in. A [`Pose2`] or
//! [`Pose3`] describes an object's position and orientation in one reference frame. A
//! [`Transform<From, To, _>`](Transform) maps coordinates from `From` to `To`.
//! Composition applies right to left: `b_to_c * a_to_b` maps from `A` to `C` via `B`.
//!
//! A pose's `as_transform` method labels the object's existing local frame without changing
//! the numeric mapping. The resulting isometry maps local coordinates into the pose's
//! reference frame:
//!
//! ```
//! use linear_algebra::{Isometry2, Point2, Pose2, point};
//!
//! struct Robot;
//! #[derive(Debug)]
//! struct World;
//!
//! let robot_in_world = Pose2::<World>::new(point![2.0, 3.0], 0.5);
//! let robot_to_world: Isometry2<Robot, World> = robot_in_world.as_transform();
//! let origin_in_robot = Point2::<Robot>::origin();
//! assert_eq!(robot_to_world * origin_in_robot, robot_in_world.position());
//! ```
//!
//! The same mapping rejects a point already expressed in `World`:
//!
//! ```compile_fail
//! # use linear_algebra::{Isometry2, Point2, Pose2, point};
//! # struct Robot;
//! # struct World;
//! # let robot_in_world = Pose2::<World>::new(point![2.0, 3.0], 0.5);
//! # let robot_to_world: Isometry2<Robot, World> = robot_in_world.as_transform();
//! let origin_in_world = Point2::<World>::origin();
//! let _ = robot_to_world * origin_in_world;
//! ```
//!
//! `as_pose` drops the local frame label from an isometry. The `wrap` methods and the
//! [`IntoFramed`] and [`IntoTransform`] helpers only attach labels; they do not convert
//! coordinates between frames.

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

pub use nalgebra;
