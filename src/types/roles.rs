use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Role {
    DefenderLeft,
    DefenderRight,
    DefenderFront,
    Keeper,
    Striker,
}

impl Default for Role {
    fn default() -> Self {
        Role::Striker
    }
}
