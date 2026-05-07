# Twix Ros-Z Reintegration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Twix build and provide generic Ros-Z connection, discovery, dynamic subscriptions, and remote parameter editing on the current Ros-Z stack.

**Architecture:** Keep Twix UI code synchronous by preserving `Robot` as the facade over a Tokio runtime and the active Ros-Z `Context`/`Node`. Use current Ros-Z dynamic subscribers for generic topic values and current `RemoteParameterClient` for parameters. Keep legacy typed logical paths explicitly unsupported for this pass.

**Tech Stack:** Rust 2024, `eframe`/`egui`, `tokio`, `ros-z`, `serde_json`, Cargo workspace tooling.

---

## File Structure

- Modify `tools/twix/Cargo.toml`: remove stale dependencies and keep only current workspace crates.
- Modify `tools/twix/src/robot.rs`: update the backend facade to current Ros-Z APIs.
- Modify `tools/twix/src/dynamic_json.rs`: convert current `DynamicPayload`/`DynamicValue` to JSON.
- Modify `tools/twix/src/backend.rs`: keep generic backend types and add any missing parameter descriptors.
- Modify `tools/twix/src/panels/parameter.rs`: replace `ros-z-config` types with current parameter responses and JSON-derived path metadata.
- Modify `tools/twix/src/panels/synthetic_pose.rs`: either port demo message to current `ros_z::Message` or remove it from selectable panels if not needed for generic-first.
- Modify `tools/twix/examples/ros_z_*.rs`: update examples to current Ros-Z APIs or remove stale examples that are not useful.
- Modify `tools/twix/src/main.rs` and `tools/twix/src/panels/mod.rs`: remove stale demo panel wiring if the demo panel is deferred.

### Task 1: Restore Twix Manifest Load

**Files:**
- Modify: `tools/twix/Cargo.toml`

- [ ] **Step 1: Reproduce manifest failure**

Run:

```bash
cargo check -p twix --examples
```

Expected: FAIL with `dependency.ros-z-config was not found in workspace.dependencies`.

- [ ] **Step 2: Remove stale dependencies**

In `tools/twix/Cargo.toml`, remove these dependency lines:

```toml
ros-z-config = { workspace = true }
ros-z-msgs = { workspace = true, features = ["sensor_msgs"] }
zenoh-buffers = "1.7.2"
```

Keep:

```toml
ros-z = { workspace = true }
```

- [ ] **Step 3: Run package check to expose Rust API failures**

Run:

```bash
cargo check -p twix --examples
```

Expected: FAIL with unresolved imports or type errors in Twix source, not manifest parsing errors.

### Task 2: Port Dynamic JSON Conversion

**Files:**
- Modify: `tools/twix/src/dynamic_json.rs`

- [ ] **Step 1: Replace stale imports**

Use current dynamic types:

```rust
use ros_z::dynamic::{
    DynamicNamedValue, DynamicPayload, DynamicStruct, DynamicValue, EnumPayloadValue, EnumValue,
};
use serde_json::{Map, Number, Value};
```

- [ ] **Step 2: Replace the public conversion entry point**

Replace `dynamic_message_to_json` with:

```rust
pub fn dynamic_payload_to_json(payload: &DynamicPayload) -> Value {
    dynamic_value_to_json(&payload.value)
}

fn dynamic_struct_to_json(message: &DynamicStruct) -> Value {
    let mut fields = Map::new();
    for (name, value) in message.iter() {
        fields.insert(name.to_string(), dynamic_value_to_json(value));
    }
    Value::Object(fields)
}
```

- [ ] **Step 3: Update enum and sequence handling**

Use current variants:

```rust
fn dynamic_value_to_json(value: &DynamicValue) -> Value {
    match value {
        DynamicValue::Bool(value) => Value::Bool(*value),
        DynamicValue::Int8(value) => Value::Number((*value).into()),
        DynamicValue::Int16(value) => Value::Number((*value).into()),
        DynamicValue::Int32(value) => Value::Number((*value).into()),
        DynamicValue::Int64(value) => Value::Number((*value).into()),
        DynamicValue::Uint8(value) => Value::Number((*value).into()),
        DynamicValue::Uint16(value) => Value::Number((*value).into()),
        DynamicValue::Uint32(value) => Value::Number((*value).into()),
        DynamicValue::Uint64(value) => Value::Number((*value).into()),
        DynamicValue::Float32(value) => Number::from_f64(*value as f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        DynamicValue::Float64(value) => Number::from_f64(*value)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        DynamicValue::String(value) => Value::String(value.clone()),
        DynamicValue::Bytes(value) => Value::Array(
            value.iter().map(|byte| Value::Number((*byte).into())).collect(),
        ),
        DynamicValue::Struct(value) => dynamic_struct_to_json(value),
        DynamicValue::Optional(None) => Value::Null,
        DynamicValue::Optional(Some(value)) => dynamic_value_to_json(value),
        DynamicValue::Enum(value) => enum_value_to_json(value),
        DynamicValue::Sequence(values) => Value::Array(values.iter().map(dynamic_value_to_json).collect()),
        DynamicValue::Map(entries) => Value::Array(
            entries
                .iter()
                .map(|(key, value)| {
                    let mut entry = Map::new();
                    entry.insert("key".to_string(), dynamic_value_to_json(key));
                    entry.insert("value".to_string(), dynamic_value_to_json(value));
                    Value::Object(entry)
                })
                .collect(),
        ),
    }
}
```

- [ ] **Step 4: Run focused check**

Run:

```bash
cargo check -p twix
```

Expected: Remaining errors are in `robot.rs`, parameter panel, or stale examples.

### Task 3: Port `Robot` to Current Ros-Z Runtime APIs

**Files:**
- Modify: `tools/twix/src/robot.rs`

- [ ] **Step 1: Replace stale imports**

Use current imports:

```rust
use std::{
    collections::{BTreeSet, VecDeque},
    sync::atomic::{AtomicU64, Ordering},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use color_eyre::eyre::eyre;
use log::error;
use ros_z::{
    context::{Context, ContextBuilder},
    dynamic::DynamicPayload,
    graph::Graph,
    node::Node,
    parameter::{
        GetNodeParameterTypeInfoResponse, GetNodeParameterValueResponse,
        GetNodeParametersSnapshotResponse, RemoteParameterClient, ResetNodeParameterResponse,
        SetNodeParameterResponse,
    },
    pubsub::Received,
    time::Time,
};
use serde_json::Value;
use tokio::{runtime::{Builder as RuntimeBuilder, Runtime}, sync::watch};
```

- [ ] **Step 2: Update `ConnectedBackend` types**

Use:

```rust
struct ConnectedBackend {
    generation: u64,
    context: Arc<Context>,
    node: Arc<Node>,
}
```

Update `context_graph`:

```rust
impl ConnectedBackend {
    fn context_graph(&self) -> &Graph {
        self.context.graph().as_ref()
    }
}
```

- [ ] **Step 3: Port connection creation**

Replace `connect_backend` with an async function:

```rust
async fn connect_backend(endpoint: &str, generation: u64) -> color_eyre::Result<ConnectedBackend> {
    let context = Arc::new(
        ContextBuilder::default()
            .with_mode("client")
            .with_connect_endpoints([endpoint])
            .build()
            .await
            .map_err(|error| eyre!(error.to_string()))?,
    );
    let node = Arc::new(
        context
            .create_node("twix")
            .with_namespace("tools")
            .build()
            .await
            .map_err(|error| eyre!(error.to_string()))?,
    );
    Ok(ConnectedBackend { generation, context, node })
}
```

Update the caller in `connect` to call `connect_backend(&endpoint, generation).await`.

- [ ] **Step 4: Port dynamic subscriber construction**

Replace `.create_dyn_sub_auto` with:

```rust
backend
    .node
    .dynamic_subscriber_auto(&topic, DYNAMIC_SCHEMA_DISCOVERY_TIMEOUT)
    .await
```

Build with `.build().await`.

- [ ] **Step 5: Convert received dynamic payloads**

Update helpers:

```rust
fn dynamic_received_to_datum(received: Received<DynamicPayload>) -> Datum<Value> {
    Datum {
        timestamp: received_timestamp(&received),
        source_timestamp: received.source_time.map(twix_time),
        value: dynamic_payload_to_json(&received.message),
    }
}

fn dynamic_received_to_change(received: Received<DynamicPayload>) -> Change<Value> {
    Change {
        timestamp: received_timestamp(&received),
        source_timestamp: received.source_time.map(twix_time),
        value: dynamic_payload_to_json(&received.message),
    }
}

fn received_timestamp<T>(received: &Received<T>) -> TwixTime {
    received
        .transport_time
        .or(received.source_time)
        .map(twix_time)
        .or_else(|| TwixTime::from_system_time(SystemTime::now()))
        .unwrap_or_else(|| TwixTime::from_duration(Duration::ZERO))
}
```

- [ ] **Step 6: Defer typed subscriptions**

Replace the body of `subscribe_topic_value<T>` with a compatibility error for now:

```rust
pub fn subscribe_topic_value<T>(&self, topic: impl Into<String>, _history: Duration) -> BufferHandle<T>
where
    T: Clone + Send + Sync + 'static,
{
    let topic = topic.into();
    let (buffer, handle) = Buffer::new(Duration::ZERO);
    buffer.push_error(eyre!(BackendError::UnsupportedCapability {
        operation: "typed topic subscription"
    }));
    error!("typed topic subscription is deferred for topic {topic}");
    handle
}
```

- [ ] **Step 7: Run focused check**

Run:

```bash
cargo check -p twix
```

Expected: Remaining errors are from parameter panel types, stale demo panel, examples, or trait bounds.

### Task 4: Port Remote Parameter Access

**Files:**
- Modify: `tools/twix/src/robot.rs`
- Modify: `tools/twix/src/backend.rs`
- Modify: `tools/twix/src/panels/parameter.rs`

- [ ] **Step 1: Replace config service naming with parameter service naming**

In `robot.rs`, replace suffix constants with current parameter names:

```rust
const PARAMETER_SNAPSHOT_SUFFIX: &str = "/parameter/get_snapshot";
```

Make `config_nodes_from_services` detect `PARAMETER_SNAPSHOT_SUFFIX`. Keep the existing public method names if that minimizes UI churn, but the implementation should use parameter services.

- [ ] **Step 2: Use `RemoteParameterClient`**

Replace `config_client` internals with:

```rust
fn parameter_client(&self, selector: &str) -> BackendResult<RemoteParameterClient> {
    let backend = self.current_backend.lock().unwrap().clone().ok_or(BackendError::NotConnected)?;
    let services = service_names(backend.context_graph());
    let node_fqn = resolve_config_node_selector(&services, selector)?;
    RemoteParameterClient::new(backend.node.clone(), node_fqn).map_err(|error| BackendError::Operation {
        operation: "parameter.client",
        message: error.to_string(),
    })
}
```

- [ ] **Step 3: Update response types**

Change methods to return current types:

```rust
pub fn get_config_snapshot(&self, selector: &str) -> BackendResult<GetNodeParametersSnapshotResponse>
pub fn get_config_value(&self, selector: &str, path: &str) -> BackendResult<GetNodeParameterValueResponse>
pub fn get_config_metadata(&self, selector: &str) -> BackendResult<GetNodeParameterTypeInfoResponse>
pub fn set_config_json(&self, selector: &str, path: &str, value: &Value, layer: String, expected_revision: Option<u64>) -> BackendResult<SetNodeParameterResponse>
pub fn reset_config(&self, selector: &str, path: &str, layer: String, expected_revision: Option<u64>) -> BackendResult<ResetNodeParameterResponse>
```

Implement each by calling the same-named current remote parameter client methods.

- [ ] **Step 4: Add JSON path flattening in parameter panel**

In `parameter.rs`, remove `ros_z_config::NodeConfigFieldMetadataWire`. Add:

```rust
#[derive(Clone, Debug)]
struct ParameterFieldMetadata {
    type_name: String,
    writable: bool,
    effective_source_layer: String,
}

fn flatten_json_paths(value: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    flatten_json_paths_inner(value, "", &mut paths);
    paths
}

fn flatten_json_paths_inner(value: &Value, prefix: &str, paths: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let path = if prefix.is_empty() { key.clone() } else { format!("{prefix}.{key}") };
                paths.push(path.clone());
                flatten_json_paths_inner(child, &path, paths);
            }
        }
        _ => {}
    }
}
```

- [ ] **Step 5: Use snapshot fields for panel context**

Update `refresh_node_context` to read current snapshot fields:

```rust
match self.robot.get_config_snapshot(&self.node_selector) {
    Ok(response) if response.success => {
        self.available_layers = response.layers;
        self.config_key = Some(response.parameter_key);
        self.current_revision = Some(response.revision);
        self.field_paths = serde_json::from_str::<Value>(&response.value_json)
            .map(|value| flatten_json_paths(&value))
            .unwrap_or_default();
        if self.target_layer.is_empty() || !self.available_layers.iter().any(|layer| layer == &self.target_layer) {
            self.target_layer = self.available_layers.last().cloned().unwrap_or_default();
        }
        self.metadata_available = !self.field_paths.is_empty();
    }
    Ok(response) => self.status_message = Some(response.message),
    Err(error) => self.status_message = Some(error.to_string()),
}
```

- [ ] **Step 6: Simplify metadata display**

Set `field_metadata` from value responses:

```rust
self.field_metadata = Some(ParameterFieldMetadata {
    type_name: "json".to_string(),
    writable: true,
    effective_source_layer: response.effective_source_layer.clone(),
});
```

Render only `type_name`, `writable`, and `effective_source_layer`.

- [ ] **Step 7: Run focused check**

Run:

```bash
cargo check -p twix
```

Expected: Parameter compile errors are gone or reduced to field-name mismatches visible in the compiler output.

### Task 5: Remove or Defer Stale Demo Typed Panel

**Files:**
- Modify: `tools/twix/src/panels/synthetic_pose.rs`
- Modify: `tools/twix/src/panels/mod.rs`
- Modify: `tools/twix/src/main.rs`

- [ ] **Step 1: Remove stale Ros-Z message API usage**

Either port `RobotPose` to the current `ros_z::Message` contract or remove the panel from selectable panels. For generic-first, remove `SyntheticPosePanel` from `impl_selectable_panel!` in `main.rs` and from exports/imports in `panels/mod.rs`.

- [ ] **Step 2: Keep the file only if it compiles**

If keeping `synthetic_pose.rs`, use:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ros_z::Message)]
#[message(name = "twix_demo::RobotPose")]
struct RobotPose {
    x: f64,
    y: f64,
    theta: f64,
    confidence: f64,
    state: String,
}
```

Do not add it back to selectable panels until typed subscriptions are intentionally restored.

- [ ] **Step 3: Run focused check**

Run:

```bash
cargo check -p twix
```

Expected: No `MessageTypeInfo`, `ZMessage`, or `SerdeCdrSerdes` errors remain.

### Task 6: Port or Remove Stale Examples

**Files:**
- Modify: `tools/twix/examples/ros_z_dynamic_spike.rs`
- Modify: `tools/twix/examples/ros_z_twix_demo_dynamic_pub.rs`
- Modify: `tools/twix/examples/ros_z_twix_demo_pose_pub.rs`
- Modify: `tools/twix/examples/ros_z_twix_demo_config_node.rs`

- [ ] **Step 1: Port dynamic spike to current API**

Use current imports and async builders:

```rust
use ros_z::{context::ContextBuilder, dynamic::{DynamicPayload, DynamicValue, EnumPayloadValue, EnumValue}};
```

Replace `ZContextBuilder` with `ContextBuilder`; replace `create_dyn_sub_auto` with `dynamic_subscriber_auto`; replace `async_recv_with_metadata` with `recv_with_metadata`.

- [ ] **Step 2: Port publishers to current derive**

For demo message structs use:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default, ros_z::Message)]
#[message(name = "twix_demo::DynamicStatus")]
```

Publish with:

```rust
let publisher = node.publisher::<DynamicStatus>(&args.topic).build().await?;
publisher.publish(&message).await?;
```

- [ ] **Step 3: Port parameter demo to current parameter API**

Use `ContextBuilder::default().with_router_endpoint(args.endpoint)?` and `node.bind_parameter_as::<TwixDemoConfig>("twix_demo")?`. Replace `ConfigMetadata` derive with `ros_z::Message` derive if metadata is not available in current Ros-Z.

- [ ] **Step 4: Run examples check**

Run:

```bash
cargo check -p twix --examples
```

Expected: Examples compile or only fail on consciously removed example files.

### Task 7: Full Formatting, Clippy, and Smoke Verification

**Files:**
- Modify files touched by formatter only if `cargo fmt` changes them.

- [ ] **Step 1: Format**

Run:

```bash
cargo fmt
cargo fmt --check
```

Expected: `cargo fmt --check` exits successfully.

- [ ] **Step 2: Check Twix**

Run:

```bash
cargo check -p twix
cargo check -p twix --examples
```

Expected: Both commands exit successfully.

- [ ] **Step 3: Clippy Twix**

Run:

```bash
cargo clippy -p twix --all-targets -- -D warnings
```

Expected: Command exits successfully.

- [ ] **Step 4: Manual smoke path**

Run a router and demo publisher in separate terminals:

```bash
cargo run -p ros-z --example zenoh_router
cargo run -p twix --example ros_z_twix_demo_dynamic_pub
cargo run -p twix -- tcp/127.0.0.1:7447
```

Expected: Twix connects, topic completion shows `/twix_demo/status`, and Text/Plot/Enum panels can subscribe to the demo topic as dynamic JSON.

## Self-Review

- Spec coverage: Tasks cover manifest load, current Ros-Z connection, dynamic subscriptions, topic discovery, parameters, deferred typed panels, examples, formatting, check, clippy, and smoke verification.
- Placeholder scan: No task depends on unspecified code; deferred behavior is explicit and scoped.
- Type consistency: Plan uses current `ContextBuilder`, `Node`, `DynamicPayload`, `DynamicValue`, `RemoteParameterClient`, and `ros_z::Message` APIs consistently.
