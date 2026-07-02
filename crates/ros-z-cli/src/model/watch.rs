use ros_z::graph::GraphRevision;
use serde::Serialize;

use crate::model::graph::GraphSummary;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum WatchEvent {
    InitialState {
        snapshot: GraphSummary,
    },
    TopicDiscovered {
        revision: GraphRevision,
        name: String,
        type_name: String,
    },
    TopicRemoved {
        revision: GraphRevision,
        name: String,
    },
    NodeDiscovered {
        revision: GraphRevision,
        namespace: String,
        name: String,
    },
    NodeRemoved {
        revision: GraphRevision,
        namespace: String,
        name: String,
    },
    ServiceDiscovered {
        revision: GraphRevision,
        name: String,
        type_name: String,
    },
    ServiceRemoved {
        revision: GraphRevision,
        name: String,
    },
}
