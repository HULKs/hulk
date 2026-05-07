# Twix Ros-Z Reintegration Design

## Goal

Bring Twix back to a working state on the current `ros-z` stack with generic tooling first: connect to Ros-Z, discover topics and parameter-capable nodes, subscribe dynamically to topic data, support parameter inspection and mutation, and render camera images from Ros-Z `sensor_msgs/Image` topics. Other legacy robot-specific typed panels can stay unavailable until explicit Ros-Z topic/type mappings exist.

## Current State

The `twix-ros-z` worktree started from an old Twix migration against removed Ros-Z APIs. The reintegration removes stale dependencies such as `ros-z-config` and `ros-z-msgs` and replaces stale API names such as `ZContextBuilder`, `ZNode`, `DynamicMessage`, `MessageTypeInfo`, and `ZMessage`.

The current Ros-Z stack exposes `ContextBuilder`, `Context`, `Node`, `Message`, `DynamicPayload`, dynamic subscriber builders, graph discovery, and `ros_z::parameter::RemoteParameterClient`.

## Architecture

Twix keeps a single backend facade in `tools/twix/src/robot.rs`. The facade owns a Tokio runtime, current endpoint, connection state, an optional Ros-Z `Context` and `Node`, and change callbacks used to repaint egui. It exposes synchronous UI-friendly methods while doing Ros-Z work on the runtime.

Generic dynamic subscriptions use `Node::dynamic_subscriber_auto(topic, timeout).await?.build().await?`, receive `DynamicPayload` values with metadata, convert payloads into `serde_json::Value`, and feed Twix `Buffer` or `ChangeBuffer` handles. Topic completion uses the Ros-Z graph snapshot.

Remote parameters use `RemoteParameterClient` from `ros_z::parameter`. Parameter node discovery is based on graph services named `/node/parameter/get_snapshot`. The parameter panel reads snapshots and individual values, derives path completion from snapshot JSON, and writes with `set_json`/`reset`.

## Image Topic Support

Twix restores image rendering through Ros-Z topics, not legacy logical paths. The Image panel subscribes to a selected `sensor_msgs/Image` topic through `Robot::subscribe_topic_value::<ros2::sensor_msgs::image::Image>`. The panel keeps topic completion, renders the latest raw image with the existing `Ros2Image` to `RgbImage` conversion path, and saves the current frame as a PNG.

The Image panel removes legacy source selection, JPEG logical path selection, YCbCr logical path selection, and the unavailable topic placeholder. Saved image panel config keeps only the selected topic and overlay config. Old saved image panels that contain `image_path`, `is_jpeg`, or `source` fields ignore those legacy fields and use the saved topic or the default image topic.

`Robot::subscribe_topic_value<T>` becomes a real typed Ros-Z subscription path for current Ros-Z message types. It follows the same backend watch and repaint callback model as dynamic JSON subscriptions. On disconnect, endpoint changes, or panel drop, subscription loops stop without retaining stale backend state.

## Deferred Scope

The first reintegration does not restore the legacy websocket logical path model. `subscribe_value<T>(logical_path)` remains unsupported. Panels that depend on old logical paths or generic value writes should show unavailable states instead of failing at runtime. The exception is the Image panel, which now uses a real typed `sensor_msgs/Image` Ros-Z topic subscription.

## Testing

The reintegration is considered ready when these pass in `.worktrees/twix-ros-z`:

```bash
cargo fmt --check
cargo test -p twix
cargo check -p twix
cargo check -p twix --examples
cargo clippy -p twix --all-targets -- -D warnings
```

Focused tests should cover image panel config migration to topic-only behavior and typed topic subscription buffering. Manual smoke testing should cover running a Zenoh router, a small Ros-Z dynamic publisher, Twix connection to `tcp/127.0.0.1:7447`, topic discovery, Text/Plot/Enum dynamic subscriptions, an image publisher on a `sensor_msgs/Image` topic, Image panel rendering, and parameter snapshot/get/set/reset against a Ros-Z node exposing parameters.
