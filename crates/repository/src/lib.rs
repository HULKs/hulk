//! Tools and utilities for managing development workflows in the repository.
//!
//! This crate simplifies tasks like building for specific targets, handling SDKs, and setting up
//! configurations, making it easier to develop, configure, and deploy for NAO robots.

use std::path::PathBuf;

pub mod cargo;
pub mod communication;
pub mod configuration;
pub mod data_home;
pub mod download;
pub mod find_root;
pub mod image;
pub mod inspect_version;
pub mod location;
pub mod modify_json;
pub mod paths;
pub mod player_number;
pub mod recording;
pub mod sdk;
pub mod symlink;
pub mod team;
pub mod upload;

/// The HULK repository.
pub struct Repository {
    pub root: PathBuf,
}

impl Repository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}
