use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use ros_z::{
    Message, SchemaHash,
    context::ContextBuilder,
    parameter::{
        GetNodeParameterTypeInfoSrv, GetNodeParameterValueRequest, GetNodeParameterValueSrv,
        GetNodeParametersSnapshotSrv, NodeParameterChangeSource, NodeParameterEvent,
        NodeParameterWriteJson, ParameterError, ParameterJsonWrite, RemoteParameterClient,
        SetNodeParameterRequest, SetNodeParameterSrv,
    },
};
use ros_z_schema::{FieldDef, SchemaBundle, TypeDef, TypeDefinition};
use serde::{Deserialize, Serialize};

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn schema_type_name(schema: &SchemaBundle) -> &str {
    let TypeDef::Named(name) = &schema.root else {
        panic!("expected named schema root, got {:?}", schema.root);
    };
    name.as_str()
}

fn schema_field<'a>(schema: &'a SchemaBundle, name: &str) -> Option<&'a FieldDef> {
    let TypeDef::Named(type_name) = &schema.root else {
        return None;
    };
    let Some(TypeDefinition::Struct(definition)) = schema.definitions.get(type_name) else {
        return None;
    };
    definition.fields.iter().find(|field| field.name == name)
}

fn next_unique_id() -> usize {
    let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    (std::process::id() as usize) * 100 + sequence
}

#[derive(Debug, Clone, Serialize, Deserialize, ros_z::Message)]
#[message(name = "test_parameters::VisionParameters")]
#[serde(deny_unknown_fields)]
struct VisionParameters {
    enabled: bool,
    threshold: f64,
    nested: NestedParameters,
}

#[derive(Debug, Clone, Serialize, Deserialize, ros_z::Message)]
#[message(name = "test_parameters::NestedParameters")]
#[serde(deny_unknown_fields)]
struct NestedParameters {
    count: u32,
}

struct TestLayers {
    base: PathBuf,
    location: PathBuf,
    robot: PathBuf,
}

fn temp_parameter_root() -> PathBuf {
    let id = next_unique_id();
    let root = std::env::temp_dir().join(format!("ros_z_parameter_test_{id}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create temp parameter root");
    root
}

fn test_layers(root: &Path, suffix: &str) -> TestLayers {
    TestLayers {
        base: root.join("base"),
        location: root.join(format!("location-lab-{suffix}")),
        robot: root.join(format!("robot-{suffix}")),
    }
}

fn layer_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn write_layer_file(layer: &Path, parameter_key: &str, contents: &str) {
    fs::create_dir_all(layer).expect("create layer dir");
    fs::write(layer.join(format!("{parameter_key}.json5")), contents)
        .expect("write parameter file");
}

async fn build_ctx(layers: &TestLayers) -> ros_z::Result<ros_z::context::Context> {
    ContextBuilder::default()
        .with_mode("peer")
        .disable_multicast_scouting()
        .with_parameter_layers([
            layers.base.clone(),
            layers.location.clone(),
            layers.robot.clone(),
        ])
        .build()
        .await
}

async fn build_empty_layers_ctx() -> ros_z::Result<ros_z::context::Context> {
    ContextBuilder::default()
        .with_mode("peer")
        .disable_multicast_scouting()
        .build()
        .await
}

async fn wait_for_service(
    node: &ros_z::node::Node,
    service: &str,
    expected_count: usize,
) -> TestResult {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(5);
    loop {
        if node.graph().view().services_named(service).len() >= expected_count {
            return Ok(());
        }
        if start.elapsed() >= timeout {
            return Err(format!("timed out waiting for service {service}").into());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn local_atomic_write_returns_commit_outcome() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "atomic-outcome");
    write_layer_file(
        &layers.base,
        "ball_detector",
        r#"{ enabled: true, threshold: 0.5, nested: { count: 1 } }"#,
    );

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;
    let parameters = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await?;
    let revision = parameters.snapshot().revision;

    let outcome = parameters
        .set_json_atomically(
            vec![ParameterJsonWrite {
                path: "threshold".into(),
                value: serde_json::json!(0.9),
                target_layer: layer_string(&layers.robot),
            }],
            Some(revision),
        )
        .await?;

    assert_eq!(outcome.committed_revision, revision + 1);
    assert_eq!(outcome.changed_paths, vec!["threshold".to_string()]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn local_atomic_write_does_not_partially_persist_layers_on_error() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "atomic-persist-error");
    write_layer_file(
        &layers.base,
        "ball_detector",
        r#"{ enabled: true, threshold: 0.5, nested: { count: 1 } }"#,
    );
    write_layer_file(&layers.location, "ball_detector", r#"{ threshold: 0.8 }"#);

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;
    let parameters = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await?;
    let revision = parameters.snapshot().revision;
    let base_file = layers.base.join("ball_detector.json5");
    let location_file = layers.location.join("ball_detector.json5");
    let original_base = fs::read_to_string(&base_file)?;

    fs::remove_file(&location_file)?;
    fs::create_dir(&location_file)?;

    let result = parameters
        .set_json_atomically(
            vec![
                ParameterJsonWrite {
                    path: "enabled".into(),
                    value: serde_json::json!(false),
                    target_layer: layer_string(&layers.base),
                },
                ParameterJsonWrite {
                    path: "threshold".into(),
                    value: serde_json::json!(0.9),
                    target_layer: layer_string(&layers.location),
                },
            ],
            Some(revision),
        )
        .await;

    assert!(result.is_err());
    assert_eq!(parameters.snapshot().revision, revision);
    assert_eq!(fs::read_to_string(&base_file)?, original_base);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn bind_merge_set_and_subscribe_work() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "a");
    write_layer_file(
        &layers.base,
        "ball_detector",
        r#"{
            enabled: true,
            threshold: 0.5,
            nested: { count: 1 }
        }"#,
    );
    write_layer_file(&layers.location, "ball_detector", r#"{ threshold: 0.8 }"#);

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;

    let parameters = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await?;
    let snapshot = parameters.snapshot();
    assert!(snapshot.typed().enabled);
    assert_eq!(snapshot.typed().threshold, 0.8);

    let mut rx = parameters.subscribe();
    parameters
        .set_json(
            "nested.count",
            serde_json::json!(7),
            layer_string(&layers.robot),
        )
        .await?;
    rx.changed().await.expect("watch update");
    let updated = rx.borrow().clone();
    assert_eq!(updated.typed().nested.count, 7);

    let robot_file = fs::read_to_string(layers.robot.join("ball_detector.json5"))?;
    let reparsed: serde_json::Value = json5::from_str(&robot_file)?;
    assert_eq!(reparsed["nested"]["count"], 7);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn second_bind_fails() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "b");
    write_layer_file(
        &layers.base,
        "ball_detector",
        r#"{ enabled: true, threshold: 0.5, nested: { count: 1 } }"#,
    );

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;

    let _parameters = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await?;
    let err = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await
        .expect_err("second bind must fail");
    assert!(err.to_string().contains("already bound"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn dropping_all_parameter_handles_allows_rebind() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "rebind-after-drop");
    write_layer_file(
        &layers.base,
        "ball_detector",
        r#"{ enabled: true, threshold: 0.5, nested: { count: 1 } }"#,
    );

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;

    let parameters = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await?;
    let clone = parameters.clone();
    drop(parameters);

    let err = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await
        .expect_err("clone should keep the binding active");
    assert!(matches!(err, ParameterError::AlreadyBound { .. }));

    drop(clone);

    let rebound = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            match node
                .bind_parameter_as::<VisionParameters>("ball_detector")
                .await
            {
                Ok(parameters) => {
                    return Ok::<_, Box<dyn std::error::Error + Send + Sync>>(parameters);
                }
                Err(ParameterError::AlreadyBound { .. }) => {
                    tokio::task::yield_now().await;
                }
                Err(err) => return Err(err.into()),
            }
        }
    })
    .await
    .map_err(|_| "timed out waiting for parameter binding to release")??;
    assert_eq!(rebound.snapshot().typed().threshold, 0.5);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn bind_rejects_empty_layer_list() -> TestResult {
    let context = build_empty_layers_ctx().await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;

    let err = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await
        .expect_err("bind must reject empty parameter layer list");
    assert!(matches!(err, ParameterError::EmptyLayerList));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn async_binding_reports_deserialization_errors() -> TestResult {
    #[derive(Debug, Clone, Serialize, Deserialize, ros_z::Message)]
    #[message(name = "test_parameters::RequiredOnlyParameters")]
    struct RequiredOnlyParameters {
        required: f64,
    }

    let root = temp_parameter_root();
    let layers = test_layers(&root, "async-bind-error");
    write_layer_file(&layers.base, "ball_detector", r#"{ threshold: 0.5 }"#);

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;

    let err = node
        .bind_parameter_as::<RequiredOnlyParameters>("ball_detector")
        .await
        .expect_err("binding should return typed parameter errors");

    assert!(matches!(err, ParameterError::DeserializationError { .. }));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn bind_rejects_invalid_parameter_key() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "invalid-key");
    write_layer_file(
        &layers.base,
        "ball_detector",
        r#"{ enabled: true, threshold: 0.5, nested: { count: 1 } }"#,
    );

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;

    let err = node
        .bind_parameter_as::<VisionParameters>("ball detector")
        .await
        .expect_err("bind must reject spaces in parameter key");
    assert!(matches!(err, ParameterError::InvalidParameterKey { .. }));

    let err = node
        .bind_parameter_as::<VisionParameters>("vision/ball_detector")
        .await
        .expect_err("bind must reject path separators in parameter key");
    assert!(matches!(err, ParameterError::InvalidParameterKey { .. }));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn late_validation_hook_validates_current_snapshot() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "c");
    write_layer_file(
        &layers.base,
        "ball_detector",
        r#"{ enabled: true, threshold: 2.5, nested: { count: 1 } }"#,
    );

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;

    let parameters = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await?;
    let err = parameters
        .add_validation_hook(|cfg: &VisionParameters| {
            if cfg.threshold > 1.0 {
                Err("threshold too high".into())
            } else {
                Ok(())
            }
        })
        .await
        .expect_err("late hook must validate current snapshot");
    assert!(err.to_string().contains("threshold too high"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn remote_v1_services_work() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "d");
    write_layer_file(
        &layers.base,
        "walk_publisher",
        r#"{ enabled: true, threshold: 0.5, nested: { count: 1 } }"#,
    );

    let context = build_ctx(&layers).await?;
    let server_node = context
        .create_node("walk_publisher")
        .with_namespace("motion")
        .build()
        .await?;
    let _parameters = server_node
        .bind_parameter_as::<VisionParameters>("walk_publisher")
        .await?;

    let client_node = context
        .create_node("tester")
        .with_namespace("tools")
        .build()
        .await?;

    let snapshot_client = client_node
        .create_service_client::<GetNodeParametersSnapshotSrv>(
            "/motion/walk_publisher/parameter/get_snapshot",
        )?
        .build()
        .await?;
    let snapshot = snapshot_client.call_async(&Default::default()).await?;
    assert!(snapshot.success);
    assert_eq!(snapshot.parameter_key, "walk_publisher");
    assert!(snapshot.value_json.contains("threshold"));

    let set_client = client_node
        .create_service_client::<SetNodeParameterSrv>("/motion/walk_publisher/parameter/set")?
        .build()
        .await?;
    let set_response = set_client
        .call_async(&SetNodeParameterRequest {
            path: "threshold".into(),
            value_json: "0.9".into(),
            target_layer: layer_string(&layers.robot),
            expected_revision: None,
        })
        .await?;
    assert!(set_response.success);

    let value_client = client_node
        .create_service_client::<GetNodeParameterValueSrv>(
            "/motion/walk_publisher/parameter/get_value",
        )?
        .build()
        .await?;
    let value_response = value_client
        .call_async(&GetNodeParameterValueRequest {
            path: "threshold".into(),
        })
        .await?;
    assert!(value_response.success);
    assert_eq!(value_response.value_json, "0.9");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn bound_parameters_expose_type_info_and_schema_lookup() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "type-info");
    write_layer_file(
        &layers.base,
        "walk_publisher",
        r#"{ enabled: true, threshold: 0.5, nested: { count: 1 } }"#,
    );

    let context = build_ctx(&layers).await?;
    let server_node = context
        .create_node("walk_publisher")
        .with_namespace("motion")
        .build()
        .await?;
    let _parameters = server_node
        .bind_parameter_as::<VisionParameters>("walk_publisher")
        .await?;

    let client_node = context
        .create_node("tester")
        .with_namespace("tools")
        .build()
        .await?;
    wait_for_service(
        &client_node,
        "/motion/walk_publisher/parameter/get_type_info",
        1,
    )
    .await?;
    let type_info_client = client_node
        .create_service_client::<GetNodeParameterTypeInfoSrv>(
            "/motion/walk_publisher/parameter/get_type_info",
        )?
        .build()
        .await?;
    let type_info = type_info_client.call_async(&Default::default()).await?;
    assert!(type_info.success);
    assert_eq!(type_info.type_name, VisionParameters::type_info().name);
    let expected_hash = VisionParameters::schema_hash();
    assert_eq!(type_info.schema_hash, expected_hash.to_hash_string());

    let schema = server_node
        .schema_service()
        .expect("schema service")
        .get_schema(
            &type_info.type_name,
            &SchemaHash::from_hash_string(&type_info.schema_hash)
                .expect("parameter type info hash must be valid"),
        )?
        .expect("registered bound schema");
    assert_eq!(schema.schema_hash, expected_hash);
    assert_eq!(schema_type_name(&schema.schema), type_info.type_name);
    assert!(schema_field(&schema.schema, "threshold").is_some());

    let remote_client_node = std::sync::Arc::new(
        context
            .create_node("tester_remote_client")
            .with_namespace("tools")
            .build()
            .await?,
    );
    let remote_client = RemoteParameterClient::new(remote_client_node, "/motion/walk_publisher")?;
    let remote_type_info = remote_client.get_type_info().await?;
    assert!(remote_type_info.success);
    assert_eq!(remote_type_info.type_name, type_info.type_name);
    assert_eq!(remote_type_info.schema_hash, type_info.schema_hash);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn remote_client_round_trips_and_receives_events() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "remote-client");
    write_layer_file(
        &layers.base,
        "walk_publisher",
        r#"{ enabled: true, threshold: 0.5, nested: { count: 1 } }"#,
    );

    let context = build_ctx(&layers).await?;
    let server_node = context
        .create_node("walk_publisher")
        .with_namespace("motion")
        .build()
        .await?;
    let _parameters = server_node
        .bind_parameter_as::<VisionParameters>("walk_publisher")
        .await?;

    let client_node = std::sync::Arc::new(
        context
            .create_node("tester")
            .with_namespace("tools")
            .build()
            .await?,
    );
    let client = RemoteParameterClient::new(client_node, "/motion/walk_publisher")?;

    let snapshot = client.get_snapshot().await?;
    assert!(snapshot.success);
    assert_eq!(snapshot.parameter_key, "walk_publisher");
    assert!(snapshot.value_json.contains("threshold"));

    let value = client.get_value("threshold").await?;
    assert!(value.success);
    assert_eq!(value.value_json, "0.5");

    let events = client.subscribe_events().await?;
    assert!(events.wait_for_publishers(1, Duration::from_secs(5)).await);

    let set = client
        .set_json(
            "threshold",
            &serde_json::json!(0.9),
            layer_string(&layers.robot),
            None,
        )
        .await?;
    assert!(set.success);
    assert_eq!(set.committed_revision, snapshot.revision + 1);
    assert!(set.changed_paths.contains(&"threshold".to_string()));

    let event: NodeParameterEvent = events.recv().await?;
    assert_eq!(event.node_fqn, "/motion/walk_publisher");
    assert_eq!(event.parameter_key, "walk_publisher");
    assert_eq!(event.source, NodeParameterChangeSource::RemoteWrite);
    assert!(event.changed_paths.contains(&"threshold".to_string()));

    let atomic = client
        .set_json_atomically(
            vec![
                NodeParameterWriteJson {
                    path: "threshold".into(),
                    value_json: "0.95".into(),
                    target_layer: layer_string(&layers.robot),
                },
                NodeParameterWriteJson {
                    path: "nested.count".into(),
                    value_json: "7".into(),
                    target_layer: layer_string(&layers.robot),
                },
            ],
            Some(set.committed_revision),
        )
        .await?;
    assert!(atomic.success);
    assert_eq!(atomic.committed_revision, set.committed_revision + 1);
    assert!(atomic.changed_paths.contains(&"threshold".to_string()));
    assert!(atomic.changed_paths.contains(&"nested.count".to_string()));

    let event: NodeParameterEvent = events.recv().await?;
    assert_eq!(event.node_fqn, "/motion/walk_publisher");
    assert_eq!(event.parameter_key, "walk_publisher");
    assert_eq!(event.source, NodeParameterChangeSource::RemoteWrite);
    assert!(event.changed_paths.contains(&"threshold".to_string()));
    assert!(event.changed_paths.contains(&"nested.count".to_string()));

    let value = client.get_value("nested.count").await?;
    assert!(value.success);
    assert_eq!(value.value_json, "7");

    let reset = client
        .reset(
            "threshold",
            layer_string(&layers.robot),
            Some(atomic.committed_revision),
        )
        .await?;
    assert!(reset.success);
    assert_eq!(reset.committed_revision, atomic.committed_revision + 1);
    assert_eq!(reset.changed_paths, vec!["threshold".to_string()]);

    let event: NodeParameterEvent = events.recv().await?;
    assert_eq!(event.node_fqn, "/motion/walk_publisher");
    assert_eq!(event.parameter_key, "walk_publisher");
    assert_eq!(event.source, NodeParameterChangeSource::RemoteWrite);
    assert_eq!(event.changed_paths, vec!["threshold".to_string()]);

    let value = client.get_value("threshold").await?;
    assert!(value.success);
    assert_eq!(value.value_json, "0.5");

    fs::write(
        layers.base.join("walk_publisher.json5"),
        r#"{ enabled: false, threshold: 0.8, nested: { count: 1 } }"#,
    )?;

    let reload = client.reload().await?;
    assert!(reload.success);
    assert_eq!(reload.committed_revision, reset.committed_revision + 1);
    assert!(reload.changed_paths.contains(&"enabled".to_string()));
    assert!(reload.changed_paths.contains(&"threshold".to_string()));

    let event: NodeParameterEvent = events.recv().await?;
    assert_eq!(event.node_fqn, "/motion/walk_publisher");
    assert_eq!(event.parameter_key, "walk_publisher");
    assert_eq!(event.source, NodeParameterChangeSource::Reload);
    assert!(event.changed_paths.contains(&"enabled".to_string()));
    assert!(event.changed_paths.contains(&"threshold".to_string()));

    let snapshot = client.get_snapshot().await?;
    assert!(snapshot.success);
    assert!(snapshot.value_json.contains(r#""enabled":false"#));
    assert!(snapshot.value_json.contains(r#""threshold":0.8"#));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn reset_exposes_lower_scope_and_noop_succeeds() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "f");
    write_layer_file(
        &layers.base,
        "ball_detector",
        r#"{ enabled: true, threshold: 0.5, nested: { count: 1 } }"#,
    );
    write_layer_file(&layers.robot, "ball_detector", r#"{ threshold: 0.9 }"#);

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;
    let parameters = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await?;
    assert_eq!(parameters.snapshot().typed().threshold, 0.9);

    parameters
        .reset("threshold", layer_string(&layers.robot))
        .await?;
    assert_eq!(parameters.snapshot().typed().threshold, 0.5);

    parameters
        .reset("threshold", layer_string(&layers.robot))
        .await?;
    assert_eq!(parameters.snapshot().typed().threshold, 0.5);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn writes_reject_inactive_target_layer() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "inactive-layer");
    write_layer_file(
        &layers.base,
        "ball_detector",
        r#"{ enabled: true, threshold: 0.5, nested: { count: 1 } }"#,
    );

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;
    let parameters = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await?;

    let inactive_layer = "/not/an/active/layer".to_string();
    let err = parameters
        .set_json("threshold", serde_json::json!(0.9), inactive_layer.clone())
        .await
        .expect_err("set_json must reject inactive layer");
    assert!(matches!(
        err,
        ParameterError::LayerNotActive { layer } if layer == inactive_layer
    ));

    let err = parameters
        .reset("threshold", inactive_layer.clone())
        .await
        .expect_err("reset must reject inactive layer");
    assert!(matches!(
        err,
        ParameterError::LayerNotActive { layer } if layer == inactive_layer
    ));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn reset_rejects_invalid_config() -> TestResult {
    #[derive(Debug, Clone, Serialize, Deserialize, ros_z::Message)]
    #[message(name = "test_parameters::RequiredOnlyParameters")]
    #[serde(deny_unknown_fields)]
    struct RequiredOnlyParameters {
        threshold: f64,
    }

    let root = temp_parameter_root();
    let layers = test_layers(&root, "g");
    write_layer_file(&layers.robot, "ball_detector", r#"{ threshold: 0.9 }"#);

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;
    let parameters = node
        .bind_parameter_as::<RequiredOnlyParameters>("ball_detector")
        .await?;

    let err = parameters
        .reset("threshold", layer_string(&layers.robot))
        .await
        .expect_err("reset should fail when it removes required parameters");
    assert!(
        err.to_string().contains("deserialization") || err.to_string().contains("missing field")
    );
    assert_eq!(parameters.snapshot().typed().threshold, 0.9);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn reload_updates_snapshot_and_preserves_last_good_on_failure() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "h");
    let path = layers.base.join("ball_detector.json5");
    write_layer_file(
        &layers.base,
        "ball_detector",
        r#"{ enabled: true, threshold: 0.5, nested: { count: 1 } }"#,
    );

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;
    let parameters = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await?;

    fs::write(
        &path,
        r#"{ enabled: true, threshold: 0.8, nested: { count: 3 } }"#,
    )?;
    parameters.reload().await?;
    assert_eq!(parameters.snapshot().typed().threshold, 0.8);
    assert_eq!(parameters.snapshot().typed().nested.count, 3);

    fs::write(&path, r#"{ threshold: "bad" }"#)?;
    let err = parameters
        .reload()
        .await
        .expect_err("reload must reject invalid parameters");
    assert!(err.to_string().contains("deserialization"));
    assert_eq!(parameters.snapshot().typed().threshold, 0.8);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn revision_mismatch_rejects_local_atomic_write() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "i");
    write_layer_file(
        &layers.base,
        "ball_detector",
        r#"{ enabled: true, threshold: 0.5, nested: { count: 1 } }"#,
    );

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;
    let parameters = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await?;

    let err = parameters
        .set_json_atomically(
            vec![ParameterJsonWrite {
                path: "threshold".into(),
                value: serde_json::json!(0.9),
                target_layer: layer_string(&layers.robot),
            }],
            Some(999),
        )
        .await
        .expect_err("revision mismatch must fail");
    assert!(err.to_string().contains("revision mismatch"));
    assert_eq!(parameters.snapshot().typed().threshold, 0.5);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn local_atomic_write_updates_multiple_paths() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "j");
    write_layer_file(
        &layers.base,
        "ball_detector",
        r#"{ enabled: true, threshold: 0.5, nested: { count: 1 } }"#,
    );

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;
    let parameters = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await?;
    let revision = parameters.snapshot().revision;

    parameters
        .set_json_atomically(
            vec![
                ParameterJsonWrite {
                    path: "threshold".into(),
                    value: serde_json::json!(0.9),
                    target_layer: layer_string(&layers.robot),
                },
                ParameterJsonWrite {
                    path: "nested.count".into(),
                    value: serde_json::json!(42),
                    target_layer: layer_string(&layers.robot),
                },
            ],
            Some(revision),
        )
        .await?;

    let snapshot = parameters.snapshot();
    assert_eq!(snapshot.typed().threshold, 0.9);
    assert_eq!(snapshot.typed().nested.count, 42);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_readers_and_writers_do_not_lose_updates() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "k");
    write_layer_file(
        &layers.base,
        "ball_detector",
        r#"{ enabled: true, threshold: 0.5, nested: { count: 0 } }"#,
    );

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;
    let parameters = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await?;
    let robot_layer = layer_string(&layers.robot);

    let writer_a = {
        let parameters = parameters.clone();
        let robot_layer = robot_layer.clone();
        tokio::spawn(async move {
            for value in 1..=10u32 {
                parameters
                    .set_json(
                        "nested.count",
                        serde_json::json!(value),
                        robot_layer.clone(),
                    )
                    .await?;
            }
            Ok::<_, ros_z::parameter::ParameterError>(())
        })
    };

    let writer_b = {
        let parameters = parameters.clone();
        let robot_layer = robot_layer.clone();
        tokio::spawn(async move {
            for value in 1..=10u32 {
                parameters
                    .set_json(
                        "threshold",
                        serde_json::json!(0.5 + (value as f64 / 10.0)),
                        robot_layer.clone(),
                    )
                    .await?;
            }
            Ok::<_, ros_z::parameter::ParameterError>(())
        })
    };

    let reader = {
        let parameters = parameters.clone();
        tokio::spawn(async move {
            for _ in 0..100 {
                let snapshot = parameters.snapshot();
                let _ = snapshot.typed().threshold;
                let _ = snapshot.typed().nested.count;
                tokio::task::yield_now().await;
            }
        })
    };

    writer_a.await??;
    writer_b.await??;
    reader.await?;

    let snapshot = parameters.snapshot();
    assert_eq!(snapshot.typed().nested.count, 10);
    assert_eq!(snapshot.typed().threshold, 1.5);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn persistence_round_trip_pretty_json_is_valid_json5() -> TestResult {
    let root = temp_parameter_root();
    let layers = test_layers(&root, "l");
    write_layer_file(
        &layers.base,
        "ball_detector",
        r#"{ enabled: true, threshold: 0.5, nested: { count: 1 } }"#,
    );

    let context = build_ctx(&layers).await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;
    let parameters = node
        .bind_parameter_as::<VisionParameters>("ball_detector")
        .await?;

    parameters
        .set_json(
            "threshold",
            serde_json::json!(0.75),
            layer_string(&layers.robot),
        )
        .await?;
    let written = fs::read_to_string(layers.robot.join("ball_detector.json5"))?;
    let reparsed: serde_json::Value = json5::from_str(&written)?;
    assert_eq!(reparsed["threshold"], 0.75);
    Ok(())
}
