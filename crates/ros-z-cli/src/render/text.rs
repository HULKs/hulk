use std::io::{self, Write};

use ros_z::graph::GraphSnapshot;

use crate::{
    model::{
        doctor::{
            DoctorEndpoint, DoctorFinding, DoctorFindingKind, DoctorQosCompatibility, DoctorReport,
            DoctorSeverity,
        },
        echo::EchoHeader,
        graph::{NodeSummary, ServiceSummary, TopicSummary},
        hz::{HzReport, HzStats},
        info::{EndpointSummary, NamedType, NodeInfo, ServiceInfo, TopicInfo},
        parameter::{
            ParameterMutationView, ParameterSnapshotView, ParameterValueView,
            ParameterWatchEventView,
        },
        schema::{SchemaFieldKindView, SchemaView},
        watch::WatchEvent,
    },
    support::nodes::fully_qualified_node_name,
};

use color_eyre::eyre::Result;

pub fn print_topic_summaries(topics: &[TopicSummary]) {
    let name_width = column_width(topics.iter().map(|topic| topic.name.as_str()));
    let type_width = column_width(topics.iter().map(|topic| topic.type_name.as_str()));

    for topic in topics {
        println!(
            "{:<name_width$}  {:<type_width$}  pubs={} subs={}",
            topic.name, topic.type_name, topic.publishers, topic.subscribers,
        );
    }
}

pub fn print_node_summaries(nodes: &[NodeSummary]) {
    for node in nodes {
        println!("{}", node.fqn);
    }
}

pub fn print_service_summaries(services: &[ServiceSummary]) {
    let name_width = column_width(services.iter().map(|service| service.name.as_str()));
    let type_width = column_width(services.iter().map(|service| service.type_name.as_str()));

    for service in services {
        println!(
            "{:<name_width$}  {:<type_width$}  servers={} clients={}",
            service.name, service.type_name, service.servers, service.clients,
        );
    }
}

pub fn print_graph_snapshot(snapshot: &GraphSnapshot) {
    println!("Topics ({})", snapshot.topics.len());
    let topics: Vec<_> = snapshot
        .topics
        .clone()
        .into_iter()
        .map(TopicSummary::from)
        .collect();
    print_topic_summaries(&topics);
    println!();

    println!("Nodes ({})", snapshot.nodes.len());
    let mut nodes: Vec<_> = snapshot
        .nodes
        .iter()
        .map(|node| NodeSummary::new(node.name.clone(), node.namespace.clone()))
        .collect();
    nodes.sort_by(|left, right| left.fqn.cmp(&right.fqn));
    print_node_summaries(&nodes);
    println!();

    println!("Services ({})", snapshot.services.len());
    let mut services: Vec<_> = snapshot
        .services
        .iter()
        .map(|service| ServiceSummary::new(service.name.clone(), service.type_name.clone(), 0, 0))
        .collect();
    services.sort_by(|left, right| left.name.cmp(&right.name));
    let name_width = column_width(services.iter().map(|service| service.name.as_str()));
    let type_width = column_width(services.iter().map(|service| service.type_name.as_str()));
    for service in services {
        println!(
            "{:<name_width$}  {:<type_width$}",
            service.name, service.type_name,
        );
    }
}

pub fn print_topic_info(info: &TopicInfo) {
    println!("Topic {}", info.name);
    println!("Type: {}", info.type_name);
    println!();
    print_endpoint_section("Publishers", &info.publishers);
    println!();
    print_endpoint_section("Subscribers", &info.subscribers);
}

pub fn print_service_info(info: &ServiceInfo) {
    println!("Service {}", info.name);
    println!("Type: {}", info.type_name);
    println!();
    print_endpoint_section("Servers", &info.servers);
    println!();
    print_endpoint_section("Clients", &info.clients);
}

pub fn print_node_info(info: &NodeInfo) {
    println!("Node {}", info.fqn);
    println!();
    print_named_type_section("Publishers", &info.publishers);
    println!();
    print_named_type_section("Subscribers", &info.subscribers);
    println!();
    print_named_type_section("Services", &info.services);
    println!();
    print_named_type_section("Clients", &info.clients);
}

pub fn print_parameter_snapshot(view: &ParameterSnapshotView) -> Result<()> {
    println!("Node: {}", view.node);
    println!("Parameter Key: {}", view.parameter_key);
    println!("Revision: {}", view.revision);
    println!(
        "Committed At: {}.{:09}",
        view.committed_at.sec, view.committed_at.nanosec
    );
    println!("Effective:");
    println!("{}", serde_json::to_string_pretty(&view.effective)?);
    for (layer, overlay) in view.layers.iter().zip(&view.layer_overlays) {
        print_overlay_summary(&format!("Layer Overlay [{layer}]"), overlay)?;
    }
    Ok(())
}

pub fn print_parameter_value(view: &ParameterValueView) -> Result<()> {
    println!("Node: {}", view.node);
    println!("Path: {}", view.path);
    println!("Revision: {}", view.revision);
    println!("Source Layer: {}", view.effective_source_layer);
    println!("Value: {}", serde_json::to_string(&view.value)?);
    Ok(())
}

pub fn print_parameter_mutation(view: &ParameterMutationView) {
    println!("Node: {}", view.node);
    println!("Operation: {}", view.operation);
    if let Some(path) = &view.path {
        println!("Path: {path}");
    }
    if let Some(layer) = &view.target_layer {
        println!("Target Layer: {layer}");
    }
    println!("Committed Revision: {}", view.committed_revision);
    println!("Successful: {}", view.successful);
    if view.changed_paths.is_empty() {
        println!("Changed Paths: 0");
    } else {
        println!("Changed Paths ({})", view.changed_paths.len());
        for path in &view.changed_paths {
            println!("{path}");
        }
    }
}

pub fn print_parameter_watch_event(view: &ParameterWatchEventView) {
    println!(
        "parameter {} ({}) rev {} -> {} source={} paths={}",
        view.node,
        view.parameter_key,
        view.previous_revision,
        view.revision,
        view.source,
        view.changed_paths.join(", ")
    );
}

pub fn print_echo_header(header: &EchoHeader) {
    println!("Topic: {}", header.topic);
    println!("Type: {}", header.type_name);
    println!("Schema hash: {}", header.schema_hash);
    println!();
}

pub fn print_echo_message(message: &str, count: Option<usize>, seen: usize) {
    print!("{message}");
    if count.is_none_or(|limit| seen < limit) {
        println!();
    }
}

pub fn print_hz_report(report: &HzReport) {
    println!("{}", hz_report_line(&report.topic, &report.receive));

    for source in &report.sources {
        println!("{}", source_hz_report_line(&source.source, &source.stats));
    }
}

fn hz_report_line(topic: &str, stats: &HzStats) -> String {
    match hz_rate_fields(stats) {
        Some((rate_hz, min_seconds, max_seconds, stddev_seconds)) => format!(
            "{}  recv={}  min={}  max={}  stddev={}  intervals={}/{}  samples={}",
            topic,
            format_hz(rate_hz),
            format_seconds(min_seconds),
            format_seconds(max_seconds),
            format_seconds(stddev_seconds),
            stats.intervals,
            stats.window_limit,
            stats.samples,
        ),
        None => format!(
            "{}  not enough samples for rate estimate  intervals={}/{}  samples={}",
            topic, stats.intervals, stats.window_limit, stats.samples
        ),
    }
}

fn source_hz_report_line(source: &str, stats: &HzStats) -> String {
    match hz_rate_fields(stats) {
        Some((rate_hz, min_seconds, max_seconds, stddev_seconds)) => format!(
            "  source={}  rate={}  min={}  max={}  stddev={}  intervals={}/{}  samples={}",
            source,
            format_hz(rate_hz),
            format_seconds(min_seconds),
            format_seconds(max_seconds),
            format_seconds(stddev_seconds),
            stats.intervals,
            stats.window_limit,
            stats.samples,
        ),
        None => format!(
            "  source={}  not enough samples for rate estimate  intervals={}/{}  samples={}",
            source, stats.intervals, stats.window_limit, stats.samples
        ),
    }
}

fn hz_rate_fields(stats: &HzStats) -> Option<(f64, f64, f64, f64)> {
    Some((
        stats.rate_hz?,
        stats.min_seconds?,
        stats.max_seconds?,
        stats.stddev_seconds?,
    ))
}

fn format_hz(rate_hz: f64) -> String {
    format!("{rate_hz:.2}Hz")
}

fn format_seconds(seconds: f64) -> String {
    format!("{seconds:.3}s")
}

pub fn print_doctor_report(report: &DoctorReport) {
    if report.findings.is_empty() {
        println!(
            "rosz doctor found no pub/sub graph issues at graph revision {}",
            format_graph_revision(report.revision)
        );
        return;
    }

    println!(
        "rosz doctor found {} warning(s), {} error(s) at graph revision {}",
        report.warning_count,
        report.error_count,
        format_graph_revision(report.revision)
    );
    println!();

    for (index, finding) in report.findings.iter().enumerate() {
        if index > 0 {
            println!();
        }
        print_doctor_finding(finding);
    }
}

fn print_doctor_finding(finding: &DoctorFinding) {
    let severity = doctor_severity_name(finding.severity);
    let kind = doctor_finding_kind_name(finding.kind);
    println!("{severity} {kind} topic {}", finding.topic);
    if let Some(compatibility) = finding.qos_compatibility {
        println!(
            "  compatibility: {}",
            doctor_qos_compatibility_name(compatibility)
        );
    }
    for endpoint in &finding.endpoints {
        print_doctor_endpoint(endpoint);
    }
}

fn print_doctor_endpoint(endpoint: &DoctorEndpoint) {
    println!(
        "  {}: {} type={} schema_hash={}",
        endpoint.kind, endpoint.node, endpoint.type_name, endpoint.schema_hash
    );
}

fn doctor_severity_name(severity: DoctorSeverity) -> &'static str {
    match severity {
        DoctorSeverity::Warning => "warning",
        DoctorSeverity::Error => "error",
    }
}

fn doctor_finding_kind_name(kind: DoctorFindingKind) -> &'static str {
    match kind {
        DoctorFindingKind::DanglingPublisher => "dangling publisher",
        DoctorFindingKind::DanglingSubscriber => "dangling subscriber",
        DoctorFindingKind::TypeMismatch => "type mismatch",
        DoctorFindingKind::QosIncompatibility => "qos incompatibility",
    }
}

fn doctor_qos_compatibility_name(compatibility: DoctorQosCompatibility) -> &'static str {
    match compatibility {
        DoctorQosCompatibility::IncompatibleReliability => "incompatible reliability",
        DoctorQosCompatibility::IncompatibleDurability => "incompatible durability",
    }
}

fn format_graph_revision(revision: ros_z::graph::GraphRevision) -> String {
    serde_json::to_string(&revision).unwrap_or_else(|_| format!("{revision:?}"))
}

pub fn print_schema(view: &SchemaView) {
    if let Err(error) = write_schema(&mut io::stdout(), view) {
        eprintln!("failed to write schema: {error}");
    }
}

fn write_schema(mut writer: impl Write, view: &SchemaView) -> io::Result<()> {
    writeln!(writer, "Node: {}", view.node)?;
    writeln!(writer, "Type: {}", view.type_name)?;
    writeln!(writer, "Root type: {}", view.root.type_name)?;
    writeln!(
        writer,
        "Root kind: {}",
        schema_field_kind_name(view.root.kind)
    )?;
    if !view.root.enum_variants.is_empty() {
        writeln!(
            writer,
            "Root variants: {}",
            view.root.enum_variants.join(", ")
        )?;
    }
    for variant_field in &view.root.enum_variant_fields {
        writeln!(
            writer,
            "Root variant {}.{}  type={}",
            variant_field.variant, variant_field.path, variant_field.type_name
        )?;
    }
    writeln!(writer, "Schema hash: {}", view.schema_hash)?;
    if view.fields.is_empty() {
        writeln!(writer, "Fields: 0")?;
        return Ok(());
    }

    writeln!(writer, "Fields ({})", view.fields.len())?;
    for field in &view.fields {
        writeln!(
            writer,
            "{}  type={} kind={}",
            field.path,
            field.type_name,
            schema_field_kind_name(field.kind)
        )?;
        if !field.enum_variants.is_empty() {
            writeln!(writer, "  variants: {}", field.enum_variants.join(", "))?;
        }
        for variant_field in &field.enum_variant_fields {
            writeln!(
                writer,
                "  variant {}.{}  type={}",
                variant_field.variant, variant_field.path, variant_field.type_name
            )?;
        }
    }

    Ok(())
}

pub fn print_watch_event(event: &WatchEvent) {
    match event {
        WatchEvent::InitialState { snapshot } => print_graph_snapshot(snapshot),
        WatchEvent::TopicDiscovered {
            name, type_name, ..
        } => {
            println!("topic + {name} ({type_name})");
        }
        WatchEvent::TopicRemoved { name, .. } => {
            println!("topic - {name}");
        }
        WatchEvent::NodeDiscovered {
            namespace, name, ..
        } => {
            println!("node + {}", fully_qualified_node_name(namespace, name));
        }
        WatchEvent::NodeRemoved {
            namespace, name, ..
        } => {
            println!("node - {}", fully_qualified_node_name(namespace, name));
        }
        WatchEvent::ServiceDiscovered {
            name, type_name, ..
        } => {
            println!("service + {name} ({type_name})");
        }
        WatchEvent::ServiceRemoved { name, .. } => {
            println!("service - {name}");
        }
    }
}

fn print_endpoint_section(label: &str, endpoints: &[EndpointSummary]) {
    println!("{label} ({})", endpoints.len());
    if endpoints.is_empty() {
        println!("none");
        return;
    }

    for endpoint in endpoints {
        let node = &endpoint.node;
        let schema_hash = &endpoint.schema_hash;
        println!("{node} [{schema_hash}]");
    }
}

fn print_named_type_section(label: &str, entries: &[NamedType]) {
    println!("{label} ({})", entries.len());
    if entries.is_empty() {
        println!("none");
        return;
    }

    let name_width = column_width(entries.iter().map(|entry| entry.name.as_str()));
    for entry in entries {
        println!("{:<name_width$}  {}", entry.name, entry.type_name);
    }
}

fn column_width<'a>(values: impl Iterator<Item = &'a str>) -> usize {
    values.map(str::len).max().unwrap_or(0)
}

fn print_overlay_summary(label: &str, value: &serde_json::Value) -> Result<()> {
    let pretty = serde_json::to_string_pretty(value)?;
    let line_count = pretty.lines().count();
    if pretty.len() <= 400 && line_count <= 12 {
        println!("{label}:");
        println!("{pretty}");
    } else {
        println!(
            "{label}: large JSON overlay ({} bytes, {} lines, use --json for full content)",
            pretty.len(),
            line_count
        );
    }
    Ok(())
}

fn schema_field_kind_name(kind: SchemaFieldKindView) -> &'static str {
    match kind {
        SchemaFieldKindView::Primitive => "primitive",
        SchemaFieldKindView::String => "string",
        SchemaFieldKindView::Struct => "struct",
        SchemaFieldKindView::Optional => "optional",
        SchemaFieldKindView::Enum => "enum",
        SchemaFieldKindView::Array => "array",
        SchemaFieldKindView::Sequence => "sequence",
        SchemaFieldKindView::Map => "map",
    }
}

#[cfg(test)]
mod hz_tests {
    use super::*;

    fn no_rate_stats() -> HzStats {
        HzStats {
            rate_hz: None,
            min_seconds: None,
            max_seconds: None,
            stddev_seconds: None,
            intervals: 0,
            window_limit: 10,
            samples: 1,
        }
    }

    fn rate_stats() -> HzStats {
        HzStats {
            rate_hz: Some(20.0),
            min_seconds: Some(0.04),
            max_seconds: Some(0.06),
            stddev_seconds: Some(0.01),
            intervals: 2,
            window_limit: 10,
            samples: 3,
        }
    }

    #[test]
    fn hz_report_line_includes_samples_without_rate() {
        assert_eq!(
            hz_report_line("/chatter", &no_rate_stats()),
            "/chatter  not enough samples for rate estimate  intervals=0/10  samples=1"
        );
    }

    #[test]
    fn source_hz_report_line_includes_samples_without_rate() {
        assert_eq!(
            source_hz_report_line("0101", &no_rate_stats()),
            "  source=0101  not enough samples for rate estimate  intervals=0/10  samples=1"
        );
    }

    #[test]
    fn hz_report_line_keeps_compact_rate_format() {
        assert_eq!(
            hz_report_line("/chatter", &rate_stats()),
            "/chatter  recv=20.00Hz  min=0.040s  max=0.060s  stddev=0.010s  intervals=2/10  samples=3"
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::model::schema::{
        SchemaEnumVariantFieldView, SchemaFieldKindView, SchemaRootView, SchemaView,
    };

    use super::write_schema;

    #[test]
    fn renders_root_schema_details() {
        let view = SchemaView {
            node: "/tools/rosz".to_string(),
            type_name: "custom_msgs::Mode".to_string(),
            schema_hash: "RZHS02_deadbeef".to_string(),
            root: SchemaRootView {
                type_name: "enum custom_msgs::Mode".to_string(),
                kind: SchemaFieldKindView::Enum,
                enum_variants: vec!["Idle".to_string(), "Manual".to_string()],
                enum_variant_fields: vec![SchemaEnumVariantFieldView {
                    variant: "Manual".to_string(),
                    path: "speed_limit".to_string(),
                    type_name: "uint32".to_string(),
                }],
            },
            fields: Vec::new(),
        };

        let mut output = Vec::new();
        write_schema(&mut output, &view).unwrap();
        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("Root type: enum custom_msgs::Mode"));
        assert!(output.contains("Root variants: Idle, Manual"));
        assert!(output.contains("Root variant Manual.speed_limit  type=uint32"));
    }
}
