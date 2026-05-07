# Twix Topic Image Subscriptions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Twix image legacy logical-path support with Ros-Z topic subscriptions for `ros2::sensor_msgs::image::Image`.

**Architecture:** Add Ros-Z message metadata to the `ros2` image/header/time types, make `Robot::subscribe_topic_value<T>` run a real typed subscriber loop, then simplify `ImagePanel` to one topic-only raw-image path. Keep the existing image decoding/rendering/saving code for `Ros2Image`.

**Tech Stack:** Rust, `ros-z`, `ros2`, Tokio, egui, Twix `Buffer`, `cargo test`, `cargo clippy`.

---

## File Map

- Modify `crates/ros2/Cargo.toml`: add `ros-z` dependency so local ROS2 message structs can implement Ros-Z message traits.
- Modify `crates/ros2/src/builtin_interfaces/time.rs`: derive `ros_z::Message`, set ROS type name, and implement `WireMessage`.
- Modify `crates/ros2/src/std_msgs/header.rs`: derive `ros_z::Message`, set ROS type name, and implement `WireMessage`.
- Modify `crates/ros2/src/sensor_msgs/image.rs`: derive `ros_z::Message`, set ROS type name, implement `WireMessage`, and add focused message metadata tests.
- Modify `tools/twix/src/robot.rs`: replace the `UnsupportedCapability` typed subscription stub with a backend-aware typed subscriber loop.
- Modify `tools/twix/src/panels/image/mod.rs`: remove legacy image source modes and subscribe only to a selected `sensor_msgs/Image` topic.
- Create `tools/twix/examples/ros_z_twix_demo_image_pub.rs`: publish a small changing RGB image for smoke tests.

Do not create commits during execution unless the user explicitly asks for them.

## Task 1: Make ROS2 Image Types Ros-Z Messages

**Files:**
- Modify: `crates/ros2/Cargo.toml`
- Modify: `crates/ros2/src/builtin_interfaces/time.rs`
- Modify: `crates/ros2/src/std_msgs/header.rs`
- Modify: `crates/ros2/src/sensor_msgs/image.rs`

- [ ] **Step 1: Write failing metadata tests**

Add this test module to the bottom of `crates/ros2/src/sensor_msgs/image.rs`:

```rust
#[cfg(test)]
mod tests {
    use ros_z::Message as _;

    use super::Image;
    use crate::{builtin_interfaces::time::Time, std_msgs::header::Header};

    #[test]
    fn image_advertises_sensor_msgs_image_type_name() {
        assert_eq!(Image::type_name(), "sensor_msgs/msg/Image");
    }

    #[test]
    fn header_and_time_advertise_ros_type_names() {
        assert_eq!(Header::type_name(), "std_msgs/msg/Header");
        assert_eq!(Time::type_name(), "builtin_interfaces/msg/Time");
    }

    #[test]
    fn image_schema_builds() {
        Image::schema().expect("image schema");
    }
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p ros2 sensor_msgs::image::tests`

Expected: FAIL because `ros_z` is not a dependency and `Image`, `Header`, and `Time` do not implement `ros_z::Message` yet.

- [ ] **Step 3: Add Ros-Z dependency to ros2**

In `crates/ros2/Cargo.toml`, add this under `[dependencies]`:

```toml
ros-z = { workspace = true }
```

- [ ] **Step 4: Implement Ros-Z message traits for Time**

In `crates/ros2/src/builtin_interfaces/time.rs`, change the derive block and add the `WireMessage` impl:

```rust
#[derive(
    Clone,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PathIntrospect,
    PathSerialize,
    PathDeserialize,
    ros_z::Message,
)]
#[message(name = "builtin_interfaces/msg/Time")]
pub struct Time {
    pub sec: i32,
    pub nanosec: u32,
}

impl ros_z::msg::WireMessage for Time {
    type Codec = ros_z::msg::SerdeCdrCodec<Time>;
}
```

- [ ] **Step 5: Implement Ros-Z message traits for Header**

In `crates/ros2/src/std_msgs/header.rs`, change the derive block and add the `WireMessage` impl:

```rust
#[derive(
    Clone,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PathIntrospect,
    PathSerialize,
    PathDeserialize,
    ros_z::Message,
)]
#[message(name = "std_msgs/msg/Header")]
pub struct Header {
    pub stamp: Time,
    pub frame_id: String,
}

impl ros_z::msg::WireMessage for Header {
    type Codec = ros_z::msg::SerdeCdrCodec<Header>;
}
```

- [ ] **Step 6: Implement Ros-Z message traits for Image**

In `crates/ros2/src/sensor_msgs/image.rs`, change the derive block and add the `WireMessage` impl after the struct:

```rust
#[derive(
    Clone,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PathIntrospect,
    PathSerialize,
    PathDeserialize,
    ros_z::Message,
)]
#[message(name = "sensor_msgs/msg/Image")]
pub struct Image {
    pub header: Header,
    pub height: u32,
    pub width: u32,
    pub encoding: String,
    pub is_bigendian: u8,
    pub step: u32,
    pub data: Vec<u8>,
}

impl ros_z::msg::WireMessage for Image {
    type Codec = ros_z::msg::SerdeCdrCodec<Image>;
}
```

- [ ] **Step 7: Run tests to verify GREEN**

Run: `cargo test -p ros2 sensor_msgs::image::tests`

Expected: PASS with the three new tests passing.

## Task 2: Implement Typed Topic Buffer Subscriptions in Robot

**Files:**
- Modify: `tools/twix/src/robot.rs`

- [ ] **Step 1: Write failing typed subscription unit test**

Add this test to the existing `#[cfg(test)] mod tests` in `tools/twix/src/robot.rs`:

```rust
#[test]
fn typed_received_to_datum_preserves_message_and_metadata() {
    let message = String::from("frame");
    let transport_time = Time::from_nanos(12_000_000_034);
    let source_time = Time::from_nanos(10_000_000_020);
    let received = Received {
        message: message.clone(),
        transport_time: Some(transport_time),
        source_time: Some(source_time),
        sequence_number: Some(7),
        source_global_id: None,
    };

    let datum = typed_received_to_datum(received).expect("typed datum");

    assert_eq!(datum.value, message);
    assert_eq!(datum.timestamp, twix_time(transport_time));
    assert_eq!(datum.source_timestamp, Some(twix_time(source_time)));
}
```

- [ ] **Step 2: Run test to verify RED**

Run: `cargo test -p twix robot::tests::typed_received_to_datum_preserves_message_and_metadata`

Expected: FAIL because `typed_received_to_datum` does not exist.

- [ ] **Step 3: Replace `subscribe_topic_value` stub**

Replace the current `subscribe_topic_value<T>` implementation in `tools/twix/src/robot.rs` with:

```rust
pub fn subscribe_topic_value<T>(&self, topic: impl Into<String>, history: Duration) -> BufferHandle<T>
where
    T: ros_z::Message + ros_z::msg::WireMessage + Clone + Send + Sync + 'static,
    for<'a> T::Codec: ros_z::msg::WireDecoder<Input<'a> = &'a [u8], Output = T>,
{
    let topic = topic.into();
    let (buffer, handle) = Buffer::new(history);
    let mut backend_rx = self.backend_tx.subscribe();
    let callbacks = self.callbacks.clone();

    self.runtime.spawn(async move {
        subscribe_typed_loop::<T>(topic, buffer, &mut backend_rx, callbacks).await;
    });

    handle
}
```

- [ ] **Step 4: Add typed subscriber loop**

Add this function near `subscribe_dynamic_json_loop` in `tools/twix/src/robot.rs`:

```rust
async fn subscribe_typed_loop<T>(
    topic: String,
    buffer: Buffer<T, color_eyre::Report>,
    backend_rx: &mut watch::Receiver<Option<Arc<ConnectedBackend>>>,
    callbacks: Arc<Mutex<Vec<ChangeCallback>>>,
) where
    T: ros_z::Message + ros_z::msg::WireMessage + Clone + Send + Sync + 'static,
    for<'a> T::Codec: ros_z::msg::WireDecoder<Input<'a> = &'a [u8], Output = T>,
{
    loop {
        if buffer.is_closed() {
            return;
        }
        let backend = tokio::select! {
            backend = wait_for_backend(backend_rx) => backend,
            () = buffer.closed() => return,
        };
        let Some(backend) = backend else {
            return;
        };
        let generation = backend.generation;
        let subscriber_result = tokio::select! {
            subscriber = backend.node.subscriber::<T>(&topic).build() => {
                subscriber.map_err(|error| BackendError::Operation {
                    operation: "typed.subscribe",
                    message: error.to_string(),
                })
            }
            () = buffer.closed() => return,
        };
        if !is_current_backend(backend_rx, generation) {
            continue;
        }
        let subscriber = match subscriber_result {
            Ok(subscriber) => subscriber,
            Err(error) => {
                buffer.push_error(eyre!(error));
                tokio::select! {
                    () = tokio::time::sleep(Duration::from_secs(1)) => {}
                    () = buffer.closed() => return,
                    changed = backend_rx.changed() => {
                        if changed.is_err() {
                            return;
                        }
                    }
                }
                continue;
            }
        };

        loop {
            tokio::select! {
                () = buffer.closed() => return,
                changed = backend_rx.changed() => {
                    if changed.is_err() {
                        return;
                    }
                    if backend_rx.borrow().as_ref().map(|backend| backend.generation) != Some(generation) {
                        break;
                    }
                }
                result = subscriber.recv_with_metadata() => {
                    if !is_current_backend(backend_rx, generation) {
                        break;
                    }
                    match result {
                        Ok(received) => {
                            if let Some(datum) = typed_received_to_datum(received) {
                                buffer.push(datum).await;
                                trigger_callbacks(&callbacks);
                            }
                        }
                        Err(error) => {
                            buffer.push_error(eyre!(BackendError::Operation {
                                operation: "typed.recv",
                                message: error.to_string(),
                            }));
                            break;
                        }
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 5: Add typed metadata conversion helper**

Add this near `dynamic_received_to_datum`:

```rust
fn typed_received_to_datum<T>(received: Received<T>) -> Option<Datum<T>> {
    Some(Datum {
        timestamp: received_timestamp(received.transport_time, received.source_time),
        source_timestamp: received.source_time.map(twix_time),
        value: received.message,
    })
}
```

- [ ] **Step 6: Run test to verify GREEN**

Run: `cargo test -p twix robot::tests::typed_received_to_datum_preserves_message_and_metadata`

Expected: PASS.

## Task 3: Convert ImagePanel to Topic-Only Raw Image Mode

**Files:**
- Modify: `tools/twix/src/panels/image/mod.rs`

- [ ] **Step 1: Replace image panel tests with topic-only expectations**

Replace the existing image tests in `tools/twix/src/panels/image/mod.rs` with:

```rust
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn image_topic_from_value_uses_default_without_saved_topic() {
        assert_eq!(image_topic_from_value(None), DEFAULT_TOPIC);
    }

    #[test]
    fn image_topic_from_value_uses_saved_topic() {
        let value = json!({ "topic": "/camera/front/image" });

        assert_eq!(image_topic_from_value(Some(&value)), "/camera/front/image");
    }

    #[test]
    fn image_save_keeps_topic_and_overlays_only() {
        let saved = saved_image_config("/camera/front/image", json!([]));

        assert_eq!(saved["topic"], "/camera/front/image");
        assert!(saved.get("source").is_none());
        assert!(saved.get("image_path").is_none());
        assert!(saved.get("is_jpeg").is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test -p twix panels::image::tests`

Expected: FAIL because `image_topic_from_value` and `saved_image_config` do not exist, and old image source tests still expect legacy behavior until replaced.

- [ ] **Step 3: Simplify image buffer and remove legacy mode types**

In `tools/twix/src/panels/image/mod.rs`:

Remove imports that only support legacy/JPEG/YCbCr mode:

```rust
use eframe::egui::{ColorImage, Context, Response, TextureId, TextureOptions, Ui, Widget};
use types::jpeg::JpegImage;
use types::ycbcr422_image::YCbCr422Image;
```

Keep this egui import shape instead:

```rust
use eframe::egui::{ColorImage, Context, Response, TextureId, TextureOptions, Ui, Widget};
```

Replace `ImageBuffer` and remove `ImageSourceMode`:

```rust
enum ImageBuffer {
    TopicRaw(BufferHandle<Ros2Image>),
}
```

Remove these constants:

```rust
const DEFAULT_IMAGE_PATH: &str = "Vision.main_outputs.image";
const LEGACY_YCBCR_PATH: &str = "Vision.main_outputs.ycbcr422_image";
```

Keep only:

```rust
const DEFAULT_TOPIC: &str = "/alpha/robot_hw/left_image";
```

- [ ] **Step 4: Add topic config helpers**

Add these helper functions near `DEFAULT_TOPIC`:

```rust
fn image_topic_from_value(value: Option<&Value>) -> &str {
    value
        .and_then(|value| value.get("topic"))
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_TOPIC)
}

fn saved_image_config(topic: &str, overlays: Value) -> Value {
    json!({
        "topic": topic,
        "overlays": overlays,
    })
}

fn subscribe_image(robot: &Arc<Robot>, topic: &str) -> ImageBuffer {
    ImageBuffer::TopicRaw(robot.subscribe_topic_value(topic, Duration::ZERO))
}
```

Add `Duration` back to the `std` import if it was removed:

```rust
use std::{env::temp_dir, fs::create_dir_all, path::PathBuf, sync::Arc, time::Duration};
```

- [ ] **Step 5: Simplify ImagePanel fields and constructor**

Update `ImagePanel` fields:

```rust
pub struct ImagePanel {
    robot: Arc<Robot>,
    image_buffer: ImageBuffer,
    overlays: Overlays,
    zoom_and_pan: ZoomAndPanTransform,
    current_topic: String,
}
```

Update `Panel::new`:

```rust
fn new(context: PanelCreationContext) -> Self {
    let current_topic = image_topic_from_value(context.value).to_string();
    let image_buffer = subscribe_image(&context.robot, &current_topic);

    let overlays = Overlays::new(
        context.robot.clone(),
        context.value.and_then(|value| value.get("overlays")),
    );
    Self {
        robot: context.robot,
        image_buffer,
        overlays,
        zoom_and_pan: ZoomAndPanTransform::default(),
        current_topic,
    }
}
```

Update `save`:

```rust
fn save(&self) -> Value {
    saved_image_config(&self.current_topic, self.overlays.save())
}
```

- [ ] **Step 6: Remove legacy save helpers**

Delete `save_jpeg_image` and `save_ycbcr422_image`. Keep one save helper:

```rust
fn save_raw_image(buffer: &BufferHandle<Ros2Image>, path: PathBuf) -> Result<()> {
    let buffer = buffer
        .get_last_value()?
        .ok_or_else(|| eyre!("no image available"))?;
    buffer.save_to_file(&path)?;
    info!("image saved to '{}'", path.display());
    Ok(())
}
```

- [ ] **Step 7: Simplify UI controls and save behavior**

In `impl Widget for &mut ImagePanel`, keep only overlays, topic completion, timestamp, and save:

```rust
ui.horizontal(|ui| {
    self.overlays.combo_box(ui);
    ui.label("Topic");
    let response = ui.add(TopicCompletionEdit::new(
        ui.id().with("image-topic"),
        &topic_state,
        &mut self.current_topic,
    ));
    if response.changed() {
        self.resubscribe();
    }

    let maybe_timestamp = match &self.image_buffer {
        ImageBuffer::TopicRaw(buffer) => buffer.get_last_timestamp(),
    };
    if let Ok(Some(timestamp)) = maybe_timestamp {
        let date: DateTime<Utc> = timestamp.as_system_time().into();
        ui.label(date.format("%T%.3f").to_string());
    }
    if ui.button("Save").clicked() {
        let time_stamp = Utc::now().format("%H:%M:%S%.3f").to_string();
        let directory = temp_dir().join("twix");
        if let Err(error) = create_dir_all(&directory) {
            warn!("failed to create temporary folder /tmp/twix: {error}");
        } else {
            let path = directory.join(format!("image_vision_{time_stamp}.png"));
            let result = match &self.image_buffer {
                ImageBuffer::TopicRaw(buffer) => save_raw_image(buffer, path),
            };
            if let Err(error) = result {
                warn!("failed to save image: {error}");
            }
        }
    }
});
```

- [ ] **Step 8: Simplify resubscribe and texture loading**

Update `resubscribe`:

```rust
fn resubscribe(&mut self) {
    self.image_buffer = subscribe_image(&self.robot, &self.current_topic);
}
```

Update `load_latest_texture` to only match `TopicRaw`:

```rust
fn load_latest_texture(&self, context: &Context) -> Result<(TextureId, (u32, u32))> {
    let image_identifier = "bytes://image-vision".to_string();
    match &self.image_buffer {
        ImageBuffer::TopicRaw(buffer) => {
            let ros_image = buffer
                .get_last_value()?
                .ok_or_else(|| eyre!("no image available"))?;
            if ros_image.height == 0 || ros_image.width == 0 {
                bail!(
                    "Image has no pixels. Dimensions: {}x{}",
                    ros_image.width,
                    ros_image.height
                );
            }

            let rgb_image: RgbImage = ros_image
                .try_into()
                .map_err(|error: image::ImageError| eyre!(error))?;

            let image = ColorImage::from_rgb(
                [rgb_image.width() as usize, rgb_image.height() as usize],
                rgb_image.as_bytes(),
            );
            let id = context
                .load_texture(&image_identifier, image, TextureOptions::NEAREST)
                .id();

            Ok((id, (rgb_image.width(), rgb_image.height())))
        }
    }
}
```

- [ ] **Step 9: Run image panel tests to verify GREEN**

Run: `cargo test -p twix panels::image::tests`

Expected: PASS with the three topic-only image panel tests passing.

## Task 4: Add Image Publisher Smoke Example

**Files:**
- Create: `tools/twix/examples/ros_z_twix_demo_image_pub.rs`

- [ ] **Step 1: Add demo image publisher example**

Create `tools/twix/examples/ros_z_twix_demo_image_pub.rs`:

```rust
use std::time::Duration;

use color_eyre::Result;
use ros2::{
    builtin_interfaces::time::Time,
    sensor_msgs::image::Image,
    std_msgs::header::Header,
};
use ros_z::context::ContextBuilder;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let context = ContextBuilder::default().build().await?;
    let node = context
        .create_node("twix_demo_image_pub")
        .with_namespace("tools")
        .build()
        .await?;
    let publisher = node.publisher::<Image>("/twix_demo/image").build().await?;

    let mut tick = 0u8;
    loop {
        publisher.publish(&demo_image(tick)).await?;
        tick = tick.wrapping_add(1);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn demo_image(tick: u8) -> Image {
    let width = 8;
    let height = 8;
    let mut data = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            data.push((x as u8).wrapping_mul(32).wrapping_add(tick));
            data.push((y as u8).wrapping_mul(32));
            data.push(tick);
        }
    }

    Image {
        header: Header {
            stamp: Time { sec: 0, nanosec: 0 },
            frame_id: "twix_demo_camera".to_string(),
        },
        height: height as u32,
        width: width as u32,
        encoding: "rgb8".to_string(),
        is_bigendian: 0,
        step: (width * 3) as u32,
        data,
    }
}
```

- [ ] **Step 2: Run examples check**

Run: `cargo check -p twix --examples`

Expected: PASS and includes `ros_z_twix_demo_image_pub`.

## Task 5: Full Verification

**Files:**
- No code edits unless a verification command fails.

- [ ] **Step 1: Format**

Run: `cargo fmt`

Expected: command exits 0.

- [ ] **Step 2: Run full verification commands**

Run these from `/home/maximilian/hulk/.worktrees/twix-ros-z`:

```bash
cargo fmt --check
cargo test -p ros2 sensor_msgs::image::tests
cargo test -p twix panels::image::tests
cargo test -p twix robot::tests::typed_received_to_datum_preserves_message_and_metadata
cargo test -p twix
cargo check -p twix
cargo check -p twix --examples
cargo clippy -p twix --all-targets -- -D warnings
```

Expected: all commands exit 0.

- [ ] **Step 3: Run dynamic smoke test regression**

Run:

```bash
set -euo pipefail; timeout 60s cargo run -p ros-z --example zenoh_router > /tmp/opencode/twix-ros-z-router.log 2>&1 & router_pid=$!; timeout 60s cargo run -p twix --example ros_z_twix_demo_dynamic_pub > /tmp/opencode/twix-ros-z-publisher.log 2>&1 & publisher_pid=$!; cleanup() { kill "$router_pid" "$publisher_pid" 2>/dev/null || true; wait "$router_pid" "$publisher_pid" 2>/dev/null || true; }; trap cleanup EXIT; sleep 5; cargo run -p twix --example ros_z_dynamic_spike -- --topic /twix_demo/status --graph-wait-secs 20 --discovery-timeout-secs 20
```

Expected: prints `graph topic: /twix_demo/status` and a JSON payload.

- [ ] **Step 4: Run image publisher smoke check**

Run:

```bash
set -euo pipefail; timeout 60s cargo run -p ros-z --example zenoh_router > /tmp/opencode/twix-ros-z-router.log 2>&1 & router_pid=$!; timeout 60s cargo run -p twix --example ros_z_twix_demo_image_pub > /tmp/opencode/twix-ros-z-image-publisher.log 2>&1 & publisher_pid=$!; cleanup() { kill "$router_pid" "$publisher_pid" 2>/dev/null || true; wait "$router_pid" "$publisher_pid" 2>/dev/null || true; }; trap cleanup EXIT; sleep 5; cargo run -p twix --example ros_z_dynamic_spike -- --topic /twix_demo/image --graph-wait-secs 20 --discovery-timeout-secs 20
```

Expected: prints `graph topic: /twix_demo/image`, `graph type: sensor_msgs/msg/Image`, and JSON containing `encoding`, `height`, `width`, and `data`.
