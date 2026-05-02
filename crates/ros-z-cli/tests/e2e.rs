use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use ros_z::{
    Message, ServiceTypeInfo,
    context::ContextBuilder,
    msg::Service,
    parameter::{NodeParameters, NodeParametersExt},
    service::ServiceServer,
};
use serde::{Deserialize as SerdeDec, Serialize as SerdeEnc};
use serde_json::Value;
use tempfile::TempDir;
use zenoh::{Wait, config::WhatAmI};

static NEXT_ROUTER_PORT: AtomicUsize = AtomicUsize::new(0);

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(15);
const EVENTUALLY_TIMEOUT: Duration = Duration::from_secs(8);
const EVENTUALLY_POLL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, SerdeEnc, SerdeDec, PartialEq, ros_z::Message)]
#[message(name = "test_cli::Telemetry")]
struct Telemetry {
    label: String,
    sequence: u32,
    temperatures: Vec<f32>,
}

#[derive(Debug, Clone, SerdeEnc, SerdeDec, PartialEq, ros_z::Message)]
#[message(name = "test_cli::VisionParameters")]
#[serde(deny_unknown_fields)]
struct VisionParameters {
    enabled: bool,
    threshold: f64,
    nested: VisionNestedParameters,
}

#[derive(Debug, Clone, SerdeEnc, SerdeDec, PartialEq, ros_z::Message)]
#[message(name = "test_cli::VisionNestedParameters")]
#[serde(deny_unknown_fields)]
struct VisionNestedParameters {
    count: u32,
}

#[derive(Debug, Clone, SerdeEnc, SerdeDec, Default, PartialEq, ros_z::Message)]
#[message(name = "test_cli::AddRequest")]
struct AddRequest {
    a: i64,
    b: i64,
}

impl ros_z::msg::WireMessage for AddRequest {
    type Codec = ros_z::msg::SerdeCdrCodec<AddRequest>;
}

#[derive(Debug, Clone, SerdeEnc, SerdeDec, Default, PartialEq, ros_z::Message)]
#[message(name = "test_cli::AddResponse")]
struct AddResponse {
    sum: i64,
}

impl ros_z::msg::WireMessage for AddResponse {
    type Codec = ros_z::msg::SerdeCdrCodec<AddResponse>;
}

struct AddTwoInts;

impl ServiceTypeInfo for AddTwoInts {
    fn service_type_info() -> ros_z::entity::TypeInfo {
        ros_z::entity::TypeInfo::new("test_cli::AddTwoInts", None)
    }
}

impl Service for AddTwoInts {
    type Request = AddRequest;
    type Response = AddResponse;
}

struct TestRouter {
    endpoint: String,
    _session: zenoh::Session,
}

impl TestRouter {
    fn new() -> Self {
        let mut last_error = None;
        for _ in 0..20 {
            match Self::try_new(candidate_router_port()) {
                Ok(router) => return router,
                Err(error) => last_error = Some(error),
            }
        }

        panic!(
            "failed to open test router after retries: {:?}",
            last_error.expect("router open error")
        );
    }

    fn try_new(port: usize) -> zenoh::Result<Self> {
        let endpoint = format!("tcp/127.0.0.1:{port}");
        let mut config = zenoh::Config::default();
        config.set_mode(Some(WhatAmI::Router)).expect("router mode");
        config
            .insert_json5("listen/endpoints", &format!("[\"{endpoint}\"]"))
            .expect("router listen endpoint");
        config
            .insert_json5("scouting/multicast/enabled", "false")
            .expect("disable multicast scouting");
        let session = zenoh::open(config).wait()?;

        Ok(Self {
            endpoint,
            _session: session,
        })
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

fn candidate_router_port() -> usize {
    20_000
        + (((std::process::id() as usize % 200) * 100)
            + NEXT_ROUTER_PORT.fetch_add(1, Ordering::Relaxed))
            % 20_000
}

struct TestEnv {
    router: TestRouter,
    temp: TempDir,
}

impl TestEnv {
    fn new() -> Self {
        Self {
            router: TestRouter::new(),
            temp: tempfile::tempdir().expect("tempdir"),
        }
    }

    fn temp_path(&self, name: &str) -> PathBuf {
        self.temp.path().join(name)
    }

    async fn create_context(&self) -> ros_z::Result<ros_z::context::Context> {
        ContextBuilder::default()
            .with_mode("client")
            .with_connect_endpoints([self.router.endpoint()])
            .disable_multicast_scouting()
            .build()
            .await
    }

    async fn create_context_with_parameter_layers(
        &self,
        layers: &[PathBuf],
    ) -> ros_z::Result<ros_z::context::Context> {
        ContextBuilder::default()
            .with_mode("client")
            .with_connect_endpoints([self.router.endpoint()])
            .disable_multicast_scouting()
            .with_parameter_layers(layers.iter().cloned())
            .build()
            .await
    }

    fn rosz(&self) -> RoszCommand {
        RoszCommand::new(self.router.endpoint())
    }
}

fn eventually_json<F>(env: &TestEnv, command: &[&str], mut predicate: F) -> Value
where
    F: FnMut(&Value) -> bool,
{
    let started = Instant::now();

    loop {
        let value = env.rosz().json_command(command).run_json();
        if predicate(&value) {
            return value;
        }

        if started.elapsed() >= EVENTUALLY_TIMEOUT {
            let graph = env.rosz().json_command(["graph"]).run_json();
            panic!(
                "condition was not met for rosz {command:?}\nlast output:\n{}\nlast graph:\n{}",
                serde_json::to_string_pretty(&value).expect("last json"),
                serde_json::to_string_pretty(&graph).expect("graph json")
            );
        }

        std::thread::sleep(EVENTUALLY_POLL);
    }
}

fn json_array_contains_field(value: &Value, field: &str, expected: &str) -> bool {
    value.as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item.get(field).and_then(Value::as_str) == Some(expected))
    })
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("test path must be UTF-8")
}

fn write_parameter_file(layer: &Path, parameter_key: &str, contents: &str) {
    std::fs::create_dir_all(layer).expect("create parameter layer");
    std::fs::write(layer.join(format!("{parameter_key}.json5")), contents)
        .expect("write parameter file");
}

struct RoszCommand {
    args: Vec<OsString>,
}

impl RoszCommand {
    fn new(router: &str) -> Self {
        Self {
            args: vec!["--router".into(), router.into()],
        }
    }

    fn json_command<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args.push("--json".into());
        self.args
            .extend(args.into_iter().map(|arg| arg.as_ref().to_os_string()));
        self
    }

    fn run(self) -> CommandOutput {
        let binary = env!("CARGO_BIN_EXE_rosz");
        let mut child = Command::new(binary)
            .args(&self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("failed to spawn rosz {:?}: {error}", self.args));

        let started = Instant::now();
        loop {
            if child
                .try_wait()
                .unwrap_or_else(|error| panic!("failed to poll rosz {:?}: {error}", self.args))
                .is_some()
            {
                let output = child.wait_with_output().unwrap_or_else(|error| {
                    panic!("failed to collect rosz {:?}: {error}", self.args)
                });
                return CommandOutput {
                    args: self.args,
                    status: output.status,
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                };
            }

            if started.elapsed() >= COMMAND_TIMEOUT {
                let _ = child.kill();
                let output = child.wait_with_output().unwrap_or_else(|error| {
                    panic!("failed to collect timed out rosz {:?}: {error}", self.args)
                });
                panic!(
                    "rosz timed out after {:?}\nargs: {:?}\nstdout:\n{}\nstderr:\n{}",
                    COMMAND_TIMEOUT,
                    self.args,
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }

            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn run_json(self) -> Value {
        let output = self.run_success();
        serde_json::from_str(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "failed to parse rosz JSON: {error}\nargs: {:?}\nstdout:\n{}\nstderr:\n{}",
                output.args, output.stdout, output.stderr
            )
        })
    }

    fn run_json_lines(self) -> Vec<Value> {
        let output = self.run_success();
        output
            .stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                serde_json::from_str(line).unwrap_or_else(|error| {
                    panic!(
                        "failed to parse rosz JSON line: {error}\nline: {line}\nargs: {:?}\nstdout:\n{}\nstderr:\n{}",
                        output.args, output.stdout, output.stderr
                    )
                })
            })
            .collect()
    }

    fn run_success(self) -> CommandOutput {
        let output = self.run();
        output.assert_success();
        output
    }
}

struct CommandOutput {
    args: Vec<OsString>,
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

impl CommandOutput {
    fn assert_success(&self) {
        assert!(
            self.status.success(),
            "rosz failed\nargs: {:?}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            self.args,
            self.status,
            self.stdout,
            self.stderr
        );
    }
}

struct GraphFixture {
    topic: String,
    service: String,
    node_fqn: String,
    _service: ServiceServer<AddTwoInts>,
    _publisher: ros_z::pubsub::Publisher<Telemetry>,
    _node: ros_z::node::Node,
    _context: ros_z::context::Context,
}

impl GraphFixture {
    async fn new(env: &TestEnv) -> TestResult<Self> {
        let context = env.create_context().await?;
        let node = context
            .create_node("fixture")
            .with_namespace("/cli_e2e")
            .build()
            .await?;
        let topic = "/cli_e2e/telemetry".to_string();
        let service = "/cli_e2e/add_two_ints".to_string();
        let publisher = node.publisher::<Telemetry>(&topic).build().await?;
        let service_server = node
            .create_service_server::<AddTwoInts>(&service)
            .build()
            .await?;

        Ok(Self {
            topic,
            service,
            node_fqn: "/cli_e2e/fixture".to_string(),
            _service: service_server,
            _publisher: publisher,
            _node: node,
            _context: context,
        })
    }
}

struct PublishingFixture {
    topic: String,
    node_fqn: String,
    publisher: Arc<ros_z::pubsub::Publisher<Telemetry>>,
    publish_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    _node: ros_z::node::Node,
    _context: ros_z::context::Context,
}

impl PublishingFixture {
    async fn new(env: &TestEnv, topic: &str) -> TestResult<Self> {
        let context = env.create_context().await?;
        let node = context
            .create_node("schema_fixture")
            .with_namespace("/cli_e2e")
            .build()
            .await?;
        let publisher = node.publisher::<Telemetry>(topic).build().await?;

        Ok(Self {
            topic: topic.to_string(),
            node_fqn: "/cli_e2e/schema_fixture".to_string(),
            publisher: Arc::new(publisher),
            publish_task: Mutex::new(None),
            _node: node,
            _context: context,
        })
    }

    fn start_publishing(&self) {
        let publisher = self.publisher.clone();
        let handle = tokio::spawn(async move {
            let message = Telemetry {
                label: "robot-7".to_string(),
                sequence: 7,
                temperatures: vec![20.0, 20.5],
            };

            loop {
                if publisher.publish(&message).await.is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });

        if let Some(previous) = self
            .publish_task
            .lock()
            .expect("publish task mutex")
            .replace(handle)
        {
            previous.abort();
        }
    }
}

impl Drop for PublishingFixture {
    fn drop(&mut self) {
        if let Some(task) = self
            .publish_task
            .get_mut()
            .expect("publish task mutex")
            .take()
        {
            task.abort();
        }
    }
}

struct ParameterFixture {
    node_fqn: String,
    robot_layer: PathBuf,
    _parameters: NodeParameters<VisionParameters>,
    _node: ros_z::node::Node,
    _context: ros_z::context::Context,
}

impl ParameterFixture {
    async fn new(env: &TestEnv) -> TestResult<Self> {
        let base_layer = env.temp_path("parameters/base");
        let robot_layer = env.temp_path("parameters/robot");
        write_parameter_file(
            &base_layer,
            "parameter_fixture",
            r#"{
                enabled: false,
                threshold: 0.5,
                nested: { count: 1 }
            }"#,
        );
        write_parameter_file(
            &robot_layer,
            "parameter_fixture",
            r#"{
                enabled: true,
                threshold: 0.8,
                nested: { count: 3 }
            }"#,
        );

        let layers = vec![base_layer, robot_layer.clone()];
        let context = env.create_context_with_parameter_layers(&layers).await?;
        let node = context
            .create_node("parameter_fixture")
            .with_namespace("/cli_e2e")
            .build()
            .await?;
        let parameters = node.bind_parameter_as::<VisionParameters>("parameter_fixture")?;

        Ok(Self {
            node_fqn: "/cli_e2e/parameter_fixture".to_string(),
            robot_layer,
            _parameters: parameters,
            _node: node,
            _context: context,
        })
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graph_json_omits_removed_graph_identifier() -> TestResult {
    let env = TestEnv::new();
    let graph = env.rosz().json_command(["graph"]).run_json();

    assert!(graph.get("domain_id").is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graph_list_and_info_report_fixture_entities() -> TestResult {
    let env = TestEnv::new();
    let fixture = GraphFixture::new(&env).await?;

    eventually_json(&env, &["graph"], |graph| {
        graph["topics"]
            .as_array()
            .is_some_and(|topics| topics.iter().any(|topic| topic["name"] == fixture.topic))
            && graph["services"].as_array().is_some_and(|services| {
                services
                    .iter()
                    .any(|service| service["name"] == fixture.service)
            })
    });

    let topics = env.rosz().json_command(["list", "topics"]).run_json();
    assert!(json_array_contains_field(&topics, "name", &fixture.topic));

    let nodes = env.rosz().json_command(["list", "nodes"]).run_json();
    assert!(json_array_contains_field(&nodes, "fqn", &fixture.node_fqn));

    let services = env.rosz().json_command(["list", "services"]).run_json();
    assert!(json_array_contains_field(
        &services,
        "name",
        &fixture.service
    ));

    let topic_info = env
        .rosz()
        .json_command(["info", "topic", fixture.topic.as_str()])
        .run_json();
    assert_eq!(topic_info["name"], fixture.topic);
    assert_eq!(topic_info["type"], Telemetry::type_name());
    assert!(
        topic_info["publishers"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let service_info = env
        .rosz()
        .json_command(["info", "service", fixture.service.as_str()])
        .run_json();
    assert_eq!(service_info["name"], fixture.service);
    assert_eq!(service_info["type"], "test_cli::AddTwoInts");
    assert!(
        service_info["servers"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let node_info = env
        .rosz()
        .json_command(["info", "node", fixture.node_fqn.as_str()])
        .run_json();
    assert_eq!(node_info["fqn"], fixture.node_fqn);
    assert!(
        node_info["publishers"]
            .as_array()
            .is_some_and(|items| { items.iter().any(|item| item["name"] == fixture.topic) }),
        "node info did not include publisher:\n{}",
        serde_json::to_string_pretty(&node_info).expect("node info json")
    );
    assert!(
        node_info["services"]
            .as_array()
            .is_some_and(|items| { items.iter().any(|item| item["name"] == fixture.service) }),
        "node info did not include service:\n{}",
        serde_json::to_string_pretty(&node_info).expect("node info json")
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn echo_receives_dynamic_message_from_fixture_publisher() -> TestResult {
    let env = TestEnv::new();
    let fixture = PublishingFixture::new(&env, "/cli_e2e/echo_telemetry").await?;
    fixture.start_publishing();

    eventually_json(&env, &["list", "topics"], |topics| {
        json_array_contains_field(topics, "name", &fixture.topic)
    });

    let lines = env
        .rosz()
        .json_command([
            "echo",
            fixture.topic.as_str(),
            "--count",
            "1",
            "--timeout",
            "5",
        ])
        .run_json_lines();

    let message = lines.first().expect("one echo message");
    assert_eq!(message["topic"], fixture.topic);
    assert_eq!(message["type"], Telemetry::type_name());
    assert_eq!(
        message["schema_hash"],
        Telemetry::schema_hash().to_hash_string()
    );
    assert_eq!(message["data"]["label"], "robot-7");
    assert_eq!(message["data"]["sequence"].as_u64(), Some(7));
    let temperatures = message["data"]["temperatures"]
        .as_array()
        .expect("temperatures array")
        .iter()
        .map(|value| value.as_f64().expect("temperature number"))
        .collect::<Vec<_>>();
    assert_eq!(temperatures, [20.0, 20.5]);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schema_resolves_fixture_message_schema() -> TestResult {
    let env = TestEnv::new();
    let fixture = PublishingFixture::new(&env, "/cli_e2e/schema_telemetry").await?;

    eventually_json(&env, &["list", "services"], |services| {
        json_array_contains_field(services, "name", "/cli_e2e/schema_fixture/get_schema")
    });

    let schema_hash = Telemetry::schema_hash().to_hash_string();
    let schema = env
        .rosz()
        .json_command([
            "schema",
            Telemetry::type_name(),
            "--node",
            fixture.node_fqn.as_str(),
            "--schema-hash",
            schema_hash.as_str(),
        ])
        .run_json();

    assert_eq!(schema["node"], fixture.node_fqn);
    assert_eq!(schema["type_name"], Telemetry::type_name());
    assert_eq!(
        schema["schema_hash"],
        Telemetry::schema_hash().to_hash_string()
    );
    assert!(schema["fields"].as_array().is_some_and(|fields| {
        fields.iter().any(|field| field["path"] == "label")
            && fields.iter().any(|field| field["path"] == "sequence")
            && fields.iter().any(|field| field["path"] == "temperatures")
    }));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parameter_commands_read_mutate_reset_and_reload_fixture_node() -> TestResult {
    let env = TestEnv::new();
    let fixture = ParameterFixture::new(&env).await?;

    eventually_json(&env, &["list", "services"], |services| {
        json_array_contains_field(
            services,
            "name",
            "/cli_e2e/parameter_fixture/parameter/get_snapshot",
        )
    });

    let snapshot = env
        .rosz()
        .json_command(["parameter", "snapshot", "--node", fixture.node_fqn.as_str()])
        .run_json();
    assert_eq!(snapshot["node"], fixture.node_fqn);
    assert_eq!(snapshot["parameter_key"], "parameter_fixture");
    assert_eq!(snapshot["effective"]["enabled"].as_bool(), Some(true));
    assert_eq!(snapshot["effective"]["threshold"].as_f64(), Some(0.8));

    let initial_revision = snapshot["revision"].as_u64().expect("initial revision");
    let value = env
        .rosz()
        .json_command([
            "parameter",
            "get",
            "nested.count",
            "--node",
            fixture.node_fqn.as_str(),
        ])
        .run_json();
    assert_eq!(value["path"], "nested.count");
    assert_eq!(value["value"].as_u64(), Some(3));

    let set = env
        .rosz()
        .json_command([
            "parameter",
            "set",
            "threshold",
            "0.95",
            "--node",
            fixture.node_fqn.as_str(),
            "--layer",
            path_str(&fixture.robot_layer),
        ])
        .run_json();
    assert_eq!(set["operation"], "set");
    assert_eq!(set["successful"].as_bool(), Some(true));
    assert!(set["committed_revision"].as_u64().unwrap_or(0) > initial_revision);
    assert!(
        set["changed_paths"]
            .as_array()
            .is_some_and(|paths| { paths.iter().any(|path| path == "threshold") })
    );

    let updated = env
        .rosz()
        .json_command([
            "parameter",
            "get",
            "threshold",
            "--node",
            fixture.node_fqn.as_str(),
        ])
        .run_json();
    assert_eq!(updated["value"].as_f64(), Some(0.95));

    let reset = env
        .rosz()
        .json_command([
            "parameter",
            "reset",
            "threshold",
            "--node",
            fixture.node_fqn.as_str(),
            "--layer",
            path_str(&fixture.robot_layer),
        ])
        .run_json();
    assert_eq!(reset["operation"], "reset");
    assert_eq!(reset["successful"].as_bool(), Some(true));

    let reset_value = env
        .rosz()
        .json_command([
            "parameter",
            "get",
            "threshold",
            "--node",
            fixture.node_fqn.as_str(),
        ])
        .run_json();
    assert_eq!(reset_value["value"].as_f64(), Some(0.5));

    write_parameter_file(
        &fixture.robot_layer,
        "parameter_fixture",
        r#"{
            enabled: true,
            threshold: 0.7,
            nested: { count: 3 }
        }"#,
    );

    let reloaded = env
        .rosz()
        .json_command(["parameter", "reload", "--node", fixture.node_fqn.as_str()])
        .run_json();
    assert_eq!(reloaded["operation"], "reload");
    assert_eq!(reloaded["successful"].as_bool(), Some(true));

    let reloaded_value = env
        .rosz()
        .json_command([
            "parameter",
            "get",
            "threshold",
            "--node",
            fixture.node_fqn.as_str(),
        ])
        .run_json();
    assert_eq!(reloaded_value["value"].as_f64(), Some(0.7));

    Ok(())
}
