# Twix Ros-Z Reintegration Design

## Goal

Bring Twix back to a working state on the current `ros-z` stack with generic tooling first: connect to Ros-Z, discover topics and parameter-capable nodes, subscribe dynamically to topic data, and support parameter inspection and mutation. Legacy robot-specific typed panels can stay unavailable until explicit Ros-Z topic/type mappings exist.

## Current State

The `twix-ros-z` worktree contains an old Twix migration against removed Ros-Z APIs. Twix currently fails before compilation because `tools/twix/Cargo.toml` references `ros-z-config` and `ros-z-msgs`, which no longer exist in workspace dependencies. The migration also uses stale API names such as `ZContextBuilder`, `ZNode`, `DynamicMessage`, `MessageTypeInfo`, and `ZMessage`.

The current Ros-Z stack exposes `ContextBuilder`, `Context`, `Node`, `Message`, `DynamicPayload`, dynamic subscriber builders, graph discovery, and `ros_z::parameter::RemoteParameterClient`.

## Architecture

Twix keeps a single backend facade in `tools/twix/src/robot.rs`. The facade owns a Tokio runtime, current endpoint, connection state, an optional Ros-Z `Context` and `Node`, and change callbacks used to repaint egui. It exposes synchronous UI-friendly methods while doing Ros-Z work on the runtime.

Generic dynamic subscriptions use `Node::dynamic_subscriber_auto(topic, timeout).await?.build().await?`, receive `DynamicPayload` values with metadata, convert payloads into `serde_json::Value`, and feed Twix `Buffer` or `ChangeBuffer` handles. Topic completion uses the Ros-Z graph snapshot.

Remote parameters use `RemoteParameterClient` from `ros_z::parameter`. Parameter node discovery is based on graph services named `/node/parameter/get_snapshot`. The parameter panel reads snapshots and individual values, derives path completion from snapshot JSON, and writes with `set_json`/`reset`.

## Deferred Scope

The first reintegration does not restore the legacy websocket logical path model. `subscribe_value<T>(logical_path)` remains unsupported unless the caller subscribes to a real typed Ros-Z topic through a current `ros_z::Message` type. Panels that depend on old logical paths or generic value writes should show unavailable states instead of failing at runtime.

## Testing

The reintegration is considered ready when these pass in `.worktrees/twix-ros-z`:

```bash
cargo fmt --check
cargo check -p twix
cargo check -p twix --examples
cargo clippy -p twix --all-targets -- -D warnings
```

Manual smoke testing should cover running a Zenoh router, a small Ros-Z dynamic publisher, Twix connection to `tcp/127.0.0.1:7447`, topic discovery, Text/Plot/Enum dynamic subscriptions, and parameter snapshot/get/set/reset against a Ros-Z node exposing parameters.
