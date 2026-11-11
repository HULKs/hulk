//! Estimate stationary and rolling ball motion with
//! [`BallMotionFilter`](motion_filter::BallMotionFilter).
//!
//! Supply states, position observations and covariances in the same fixed metric
//! frame. See [`motion_filter`] for state ordering and covariance requirements.

mod dynamics;
pub mod motion_filter;
