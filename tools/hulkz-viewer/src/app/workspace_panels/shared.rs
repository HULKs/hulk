use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NamespaceSelection {
    #[default]
    FollowDefault,
    Override(String),
}
