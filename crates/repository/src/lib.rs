//! Tools and utilities for managing development workflows in the repository.
//!
//! This crate simplifies tasks like building for specific targets, handling SDKs, and setting up
//! configurations, making it easier to develop, configure, and deploy our robots.

use std::path::PathBuf;

pub mod cargo;
pub mod configuration;
pub mod data_home;
pub mod download;
pub mod find_root;
pub mod inspect_version;
pub mod location;
pub mod paths;
pub mod player_number;
pub mod sdk;
pub mod symlink;
pub mod team;
pub mod upload;

pub use player_number::PlayerNumber;

/// The HULK repository.
#[derive(Debug, Clone)]
pub struct Repository {
    pub root: PathBuf,
}

impl Repository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}
