use std::{collections::BTreeSet, sync::Arc, time::Duration};

use color_eyre::eyre::Result;
use ros_z::entity::{Entity, EntityKind};

use crate::app::AppContext;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EndpointBrief {
    node: String,
    type_name: String,
    schema_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum DoctorFinding {
    DanglingSubscriber {
        topic: String,
        subscribers: Vec<EndpointBrief>,
    },
    DanglingPublisher {
        topic: String,
        publishers: Vec<EndpointBrief>,
    },
    TypeMismatch {
        topic: String,
        publisher: EndpointBrief,
        subscriber: EndpointBrief,
    },
}

pub async fn run(app: &AppContext, settle_timeout_seconds: f64) -> Result<bool> {
    let settle_timeout = Duration::from_secs_f64(settle_timeout_seconds);
    app.wait_for_graph_settle_with_timeout(settle_timeout).await;

    let findings = collect_findings(app);
    print_findings(&findings);
    Ok(!findings.is_empty())
}

fn collect_findings(app: &AppContext) -> Vec<DoctorFinding> {
    let graph = app.graph();
    let mut findings = BTreeSet::new();

    for topic in app.snapshot().topics {
        let publishers =
            endpoint_briefs(graph.get_entities_by_topic(EntityKind::Publisher, &topic.name));
        let subscribers =
            endpoint_briefs(graph.get_entities_by_topic(EntityKind::Subscription, &topic.name));

        if publishers.is_empty() && !subscribers.is_empty() {
            findings.insert(DoctorFinding::DanglingSubscriber {
                topic: topic.name.clone(),
                subscribers: subscribers.clone(),
            });
        }

        if subscribers.is_empty() && !publishers.is_empty() {
            findings.insert(DoctorFinding::DanglingPublisher {
                topic: topic.name.clone(),
                publishers: publishers.clone(),
            });
        }

        for publisher in &publishers {
            for subscriber in &subscribers {
                if publisher.type_name != subscriber.type_name
                    || publisher.schema_hash != subscriber.schema_hash
                {
                    findings.insert(DoctorFinding::TypeMismatch {
                        topic: topic.name.clone(),
                        publisher: publisher.clone(),
                        subscriber: subscriber.clone(),
                    });
                }
            }
        }
    }

    findings.into_iter().collect()
}

fn endpoint_briefs(entities: Vec<Arc<Entity>>) -> Vec<EndpointBrief> {
    let mut endpoints = BTreeSet::new();

    for entity in entities {
        let Entity::Endpoint(endpoint) = &*entity else {
            continue;
        };
        endpoints.insert(EndpointBrief {
            node: endpoint.node.fully_qualified_name(),
            type_name: endpoint.type_info.name.clone(),
            schema_hash: endpoint.type_info.hash.to_string(),
        });
    }

    endpoints.into_iter().collect()
}

fn print_findings(findings: &[DoctorFinding]) {
    if findings.is_empty() {
        println!("No pub/sub graph inconsistencies found.");
        return;
    }

    println!(
        "rosz doctor found {} pub/sub graph issue(s):",
        findings.len()
    );
    for finding in findings {
        match finding {
            DoctorFinding::DanglingSubscriber { topic, subscribers } => {
                println!("warning dangling subscriber topic {topic}");
                print_endpoints("subscriber", subscribers);
            }
            DoctorFinding::DanglingPublisher { topic, publishers } => {
                println!("warning dangling publisher topic {topic}");
                print_endpoints("publisher", publishers);
            }
            DoctorFinding::TypeMismatch {
                topic,
                publisher,
                subscriber,
            } => {
                println!("error type mismatch on topic {topic}");
                print_endpoint("publisher", publisher);
                print_endpoint("subscriber", subscriber);
            }
        }
    }
}

fn print_endpoints(label: &str, endpoints: &[EndpointBrief]) {
    for endpoint in endpoints {
        print_endpoint(label, endpoint);
    }
}

fn print_endpoint(label: &str, endpoint: &EndpointBrief) {
    println!(
        "  {label}: {} type={} schema_hash={}",
        endpoint.node, endpoint.type_name, endpoint.schema_hash
    );
}
