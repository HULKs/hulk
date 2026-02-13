#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscoveredPublisher {
    pub namespace: String,
    pub node: String,
    pub path_expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscoveredSession {
    pub namespace: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscoveredParameter {
    pub namespace: String,
    pub node: String,
    pub path_expression: String,
}
