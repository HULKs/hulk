use std::collections::BTreeMap;

use ros_z::{
    entity::{EndpointEntity, EndpointKind},
    graph::{Graph, GraphData, GraphRevision},
    qos::{QosCompatibility, QosProfile},
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorFindingKind {
    DanglingPublisher,
    DanglingSubscriber,
    TypeMismatch,
    QosIncompatibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorQosCompatibility {
    IncompatibleReliability,
    IncompatibleDurability,
}

impl TryFrom<QosCompatibility> for DoctorQosCompatibility {
    type Error = ();

    fn try_from(compatibility: QosCompatibility) -> Result<Self, Self::Error> {
        match compatibility {
            QosCompatibility::IncompatibleReliability => Ok(Self::IncompatibleReliability),
            QosCompatibility::IncompatibleDurability => Ok(Self::IncompatibleDurability),
            QosCompatibility::Compatible => Err(()),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorEndpoint {
    pub node: String,
    pub kind: String,
    pub topic: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub schema_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorFinding {
    pub severity: DoctorSeverity,
    pub kind: DoctorFindingKind,
    pub topic: String,
    pub endpoints: Vec<DoctorEndpoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qos_compatibility: Option<DoctorQosCompatibility>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub revision: GraphRevision,
    pub warning_count: usize,
    pub error_count: usize,
    pub findings: Vec<DoctorFinding>,
}

impl DoctorReport {
    pub fn new(revision: GraphRevision, mut findings: Vec<DoctorFinding>) -> Self {
        for finding in &mut findings {
            sort_doctor_endpoints(&mut finding.endpoints);
        }
        findings.sort_by_key(finding_sort_key);
        let warning_count = findings
            .iter()
            .filter(|finding| finding.severity == DoctorSeverity::Warning)
            .count();
        let error_count = findings
            .iter()
            .filter(|finding| finding.severity == DoctorSeverity::Error)
            .count();

        Self {
            revision,
            warning_count,
            error_count,
            findings,
        }
    }

    pub fn from_graph(graph: &Graph) -> Self {
        let data = graph.lock().clone();
        Self::from_graph_data(&data)
    }

    pub fn from_graph_data(data: &GraphData) -> Self {
        let revision = data.revision();
        let endpoints = data
            .endpoints()
            .filter(|endpoint| {
                matches!(
                    endpoint.kind,
                    EndpointKind::Publisher | EndpointKind::Subscription
                )
            })
            .cloned()
            .collect();
        Self::from_endpoints(revision, endpoints)
    }

    fn from_endpoints(revision: GraphRevision, endpoints: Vec<EndpointEntity>) -> Self {
        let mut by_topic: BTreeMap<String, TopicEndpoints> = BTreeMap::new();
        for endpoint in endpoints {
            let entry = by_topic.entry(endpoint.topic.clone()).or_default();
            match endpoint.kind {
                EndpointKind::Publisher => entry.publishers.push(endpoint),
                EndpointKind::Subscription => entry.subscribers.push(endpoint),
                EndpointKind::Service | EndpointKind::Client => {}
            }
        }

        let mut findings = Vec::new();
        for (topic, endpoints) in &by_topic {
            collect_dangling_topic_findings(&mut findings, topic, endpoints);
            collect_type_mismatch_findings(&mut findings, topic, endpoints);
            collect_qos_incompatibility_findings(&mut findings, topic, endpoints);
        }

        Self::new(revision, findings)
    }

    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }
}

#[derive(Default)]
struct TopicEndpoints {
    publishers: Vec<EndpointEntity>,
    subscribers: Vec<EndpointEntity>,
}

fn collect_dangling_topic_findings(
    findings: &mut Vec<DoctorFinding>,
    topic: &str,
    endpoints: &TopicEndpoints,
) {
    if endpoints.subscribers.is_empty() && !endpoints.publishers.is_empty() {
        findings.push(DoctorFinding {
            severity: DoctorSeverity::Warning,
            kind: DoctorFindingKind::DanglingPublisher,
            topic: topic.to_string(),
            endpoints: endpoints
                .publishers
                .iter()
                .map(DoctorEndpoint::from)
                .collect(),
            qos_compatibility: None,
        });
    }

    if endpoints.publishers.is_empty() && !endpoints.subscribers.is_empty() {
        findings.push(DoctorFinding {
            severity: DoctorSeverity::Warning,
            kind: DoctorFindingKind::DanglingSubscriber,
            topic: topic.to_string(),
            endpoints: endpoints
                .subscribers
                .iter()
                .map(DoctorEndpoint::from)
                .collect(),
            qos_compatibility: None,
        });
    }
}

fn collect_type_mismatch_findings(
    findings: &mut Vec<DoctorFinding>,
    topic: &str,
    endpoints: &TopicEndpoints,
) {
    for publisher in &endpoints.publishers {
        for subscriber in &endpoints.subscribers {
            if publisher.type_info != subscriber.type_info {
                findings.push(DoctorFinding {
                    severity: DoctorSeverity::Error,
                    kind: DoctorFindingKind::TypeMismatch,
                    topic: topic.to_string(),
                    endpoints: vec![
                        DoctorEndpoint::from(publisher),
                        DoctorEndpoint::from(subscriber),
                    ],
                    qos_compatibility: None,
                });
            }
        }
    }
}

fn collect_qos_incompatibility_findings(
    findings: &mut Vec<DoctorFinding>,
    topic: &str,
    endpoints: &TopicEndpoints,
) {
    for publisher in &endpoints.publishers {
        let Ok(offered) = QosProfile::try_from(publisher.qos) else {
            continue;
        };

        for subscriber in &endpoints.subscribers {
            let Ok(requested) = QosProfile::try_from(subscriber.qos) else {
                continue;
            };
            let compatibility = requested.compatibility_with_offered(&offered);
            let Ok(qos_compatibility) = DoctorQosCompatibility::try_from(compatibility) else {
                continue;
            };

            findings.push(DoctorFinding {
                severity: DoctorSeverity::Error,
                kind: DoctorFindingKind::QosIncompatibility,
                topic: topic.to_string(),
                endpoints: vec![
                    DoctorEndpoint::from(publisher),
                    DoctorEndpoint::from(subscriber),
                ],
                qos_compatibility: Some(qos_compatibility),
            });
        }
    }
}

impl From<&EndpointEntity> for DoctorEndpoint {
    fn from(endpoint: &EndpointEntity) -> Self {
        Self {
            node: endpoint.node.fully_qualified_name(),
            kind: endpoint_kind_name(endpoint.kind).to_string(),
            topic: endpoint.topic.clone(),
            type_name: endpoint.type_info.name.clone(),
            schema_hash: endpoint.type_info.hash.to_hash_string(),
        }
    }
}

fn endpoint_kind_name(kind: EndpointKind) -> &'static str {
    match kind {
        EndpointKind::Publisher => "publisher",
        EndpointKind::Subscription => "subscriber",
        EndpointKind::Service => "service",
        EndpointKind::Client => "client",
    }
}

fn sort_doctor_endpoints(endpoints: &mut [DoctorEndpoint]) {
    endpoints.sort_by(|left, right| endpoint_sort_key(left).cmp(&endpoint_sort_key(right)));
}

type EndpointSortKey<'a> = (&'a str, &'a str, &'a str, &'a str, &'a str);
type OwnedEndpointSortKey = (String, String, String, String, String);
type FindingSortKey = (
    DoctorSeverity,
    DoctorFindingKind,
    String,
    Vec<OwnedEndpointSortKey>,
    Option<DoctorQosCompatibility>,
);

fn endpoint_sort_key(endpoint: &DoctorEndpoint) -> EndpointSortKey<'_> {
    (
        endpoint.node.as_str(),
        endpoint.type_name.as_str(),
        endpoint.schema_hash.as_str(),
        endpoint.kind.as_str(),
        endpoint.topic.as_str(),
    )
}

fn finding_sort_key(finding: &DoctorFinding) -> FindingSortKey {
    (
        finding.severity,
        finding.kind,
        finding.topic.clone(),
        finding
            .endpoints
            .iter()
            .map(|endpoint| {
                (
                    endpoint.node.clone(),
                    endpoint.type_name.clone(),
                    endpoint.schema_hash.clone(),
                    endpoint.kind.clone(),
                    endpoint.topic.clone(),
                )
            })
            .collect(),
        finding.qos_compatibility,
    )
}

#[cfg(test)]
mod tests {
    use ros_z::entity::{EndpointEntity, EndpointKind, NodeEntity, SchemaHash, TypeInfo};
    use ros_z::qos::{QosCompatibility, QosHistory, QosProfile, QosReliability};
    use serde_json::json;

    use super::{DoctorFindingKind, DoctorReport, DoctorSeverity};

    fn endpoint_with_hash(
        id: usize,
        kind: EndpointKind,
        topic: &str,
        type_name: &str,
        schema_hash: SchemaHash,
    ) -> EndpointEntity {
        EndpointEntity {
            id,
            node: NodeEntity {
                z_id: Default::default(),
                id,
                name: format!("node_{id}"),
                namespace: "/doctor_test".to_string(),
            },
            kind,
            topic: topic.to_string(),
            type_info: TypeInfo::new(type_name, schema_hash),
            qos: Default::default(),
        }
    }

    fn endpoint(id: usize, kind: EndpointKind, topic: &str, type_name: &str) -> EndpointEntity {
        endpoint_with_hash(id, kind, topic, type_name, SchemaHash::zero())
    }

    #[test]
    fn warning_only_report_has_no_errors() {
        let report = DoctorReport::from_endpoints(
            ros_z::graph::GraphRevision::INITIAL,
            vec![endpoint(
                1,
                EndpointKind::Publisher,
                "/topic",
                "test_msgs::A",
            )],
        );

        assert_eq!(report.warning_count, 1);
        assert_eq!(report.error_count, 0);
        assert!(!report.has_errors());
    }

    #[test]
    fn dangling_subscriber_report_is_warning_only() {
        let report = DoctorReport::from_endpoints(
            ros_z::graph::GraphRevision::INITIAL,
            vec![endpoint(
                1,
                EndpointKind::Subscription,
                "/topic",
                "test_msgs::A",
            )],
        );

        assert_eq!(report.warning_count, 1);
        assert_eq!(report.error_count, 0);
        assert!(!report.has_errors());
        assert_eq!(report.findings[0].severity, DoctorSeverity::Warning);
        assert_eq!(
            report.findings[0].kind,
            DoctorFindingKind::DanglingSubscriber
        );
    }

    #[test]
    fn from_endpoints_reports_type_mismatches() {
        let report = DoctorReport::from_endpoints(
            ros_z::graph::GraphRevision::INITIAL,
            vec![
                endpoint(1, EndpointKind::Publisher, "/topic", "test_msgs::A"),
                endpoint(2, EndpointKind::Subscription, "/topic", "test_msgs::B"),
            ],
        );

        assert_eq!(report.warning_count, 0);
        assert_eq!(report.error_count, 1);
        assert!(report.has_errors());
        assert_eq!(report.findings[0].severity, DoctorSeverity::Error);
        assert_eq!(report.findings[0].kind, DoctorFindingKind::TypeMismatch);
        assert_eq!(report.findings[0].topic, "/topic");
        assert_eq!(report.findings[0].endpoints.len(), 2);
    }

    #[test]
    fn from_endpoints_reports_qos_incompatibilities() {
        let mut publisher = endpoint(1, EndpointKind::Publisher, "/topic", "test_msgs::A");
        publisher.qos = QosProfile {
            reliability: QosReliability::BestEffort,
            history: QosHistory::KeepLast(std::num::NonZeroUsize::new(10).unwrap()),
            ..Default::default()
        }
        .to_protocol_qos();

        let mut subscriber = endpoint(2, EndpointKind::Subscription, "/topic", "test_msgs::A");
        subscriber.qos = QosProfile {
            reliability: QosReliability::Reliable,
            history: QosHistory::KeepLast(std::num::NonZeroUsize::new(10).unwrap()),
            ..Default::default()
        }
        .to_protocol_qos();

        let report = DoctorReport::from_endpoints(
            ros_z::graph::GraphRevision::INITIAL,
            vec![publisher, subscriber],
        );

        assert_eq!(report.warning_count, 0);
        assert_eq!(report.error_count, 1);
        assert!(report.has_errors());
        assert_eq!(report.findings[0].severity, DoctorSeverity::Error);
        assert_eq!(
            report.findings[0].kind,
            DoctorFindingKind::QosIncompatibility
        );
        assert_eq!(
            report.findings[0].qos_compatibility,
            Some(super::DoctorQosCompatibility::IncompatibleReliability)
        );
    }

    #[test]
    fn qos_compatibility_serializes_as_snake_case() {
        let finding = super::DoctorFinding {
            severity: DoctorSeverity::Error,
            kind: DoctorFindingKind::QosIncompatibility,
            topic: "/topic".to_string(),
            endpoints: Vec::new(),
            qos_compatibility: Some(
                super::DoctorQosCompatibility::try_from(QosCompatibility::IncompatibleReliability)
                    .expect("incompatible reliability should be representable"),
            ),
        };

        let value = serde_json::to_value(&finding).expect("doctor finding should serialize");

        assert_eq!(
            value,
            json!({
                "severity": "error",
                "kind": "qos_incompatibility",
                "topic": "/topic",
                "endpoints": [],
                "qos_compatibility": "incompatible_reliability"
            })
        );
    }

    #[test]
    fn finding_order_is_deterministic() {
        let report = DoctorReport::from_endpoints(
            ros_z::graph::GraphRevision::INITIAL,
            vec![
                endpoint(2, EndpointKind::Publisher, "/z_topic", "test_msgs::A"),
                endpoint(1, EndpointKind::Publisher, "/a_topic", "test_msgs::A"),
            ],
        );

        assert_eq!(report.findings.len(), 2);
        assert_eq!(report.findings[0].topic, "/a_topic");
        assert_eq!(report.findings[1].topic, "/z_topic");
    }

    #[test]
    fn dangling_finding_endpoint_order_is_deterministic() {
        let report = DoctorReport::from_endpoints(
            ros_z::graph::GraphRevision::INITIAL,
            vec![
                endpoint(3, EndpointKind::Publisher, "/topic", "test_msgs::C"),
                endpoint(1, EndpointKind::Publisher, "/topic", "test_msgs::A"),
                endpoint(2, EndpointKind::Publisher, "/topic", "test_msgs::B"),
            ],
        );

        assert_eq!(report.findings.len(), 1);
        assert_eq!(
            report.findings[0]
                .endpoints
                .iter()
                .map(|endpoint| (endpoint.node.as_str(), endpoint.type_name.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("/doctor_test/node_1", "test_msgs::A"),
                ("/doctor_test/node_2", "test_msgs::B"),
                ("/doctor_test/node_3", "test_msgs::C"),
            ]
        );
    }
}
