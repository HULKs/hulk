# Overview

The current robot stack uses **ROS-Z nodes connected by typed, named topics**.
Each node receives messages, performs its part of the processing, and publishes results for other nodes to consume.
ROS-Z is our Zenoh-native Rust middleware; it provides publish/subscribe, services, discovery, parameters, and clocks without depending on the ROS 2 C/C++ runtime.

### Middleware Origin

Our `ros-z` stack is derived from [ZettaScaleLabs' ros-z project, now named Hiroz](https://github.com/ZettaScaleLabs/hiroz), developed by the Zenoh team at ZettaScale.
The upstream project implements a Rust robotics middleware stack directly on Zenoh, including interoperability with Zenoh-based ROS 2.
HULKs maintains its adapted version inside this repository in `crates/ros-z` and the supporting `ros-z-*` crates.
For the APIs used by our nodes, refer to the local code and `crates/ros-z/README.md`; upstream documentation describes the upstream implementation and can differ from ours.

## Runtime and Nodes

The executable in `crates/hulk_ros_z` creates a shared ROS-Z `Context` and starts the node entry points as Tokio tasks.
The `spawn_all` function in `crates/hulk_ros_z/src/main.rs` is the list of nodes started by that executable.
Node implementations live in `crates/nodes/`.

A typical node:

1. Creates a named node with `ctx.create_node("ball_filter").build().await?`.
2. Binds parameters and creates its subscribers, caches, and publishers.
3. Keeps algorithm state across loop iterations.
4. Waits for input messages or a timer, computes a result, and publishes it.

Each node decides what triggers its processing. There is no single cycle frequency shared by all nodes.
For example, detection waits for images, while the ball filter waits for batches of time-ordered odometry and detection events.
An async wait yields to Tokio so other tasks can run. A node task is not a dedicated operating-system thread.

## Inputs and Topic Names

Inputs are subscribed to explicitly in node code:

```rust
let subscriber = node.subscriber::<String>("example/input").build().await?;
let message = subscriber.recv().await?;
```

The producer publishes on the same topic with a compatible message type and QoS (delivery and history settings).
The middleware handles transport and endpoint discovery; the consumer does not directly call the producer.
Topic strings provide the connection, not Rust variable names or node startup order.

Topics are qualified relative to the node's namespace:

- `inputs/odometry` in namespace `/42` becomes `/42/inputs/odometry`.
- `/inputs/odometry` is absolute and remains unchanged.
- `~/status` is private to the node and becomes `/42/<node_name>/status`.

Ordinary relative topics do not automatically include the node name. The ball filter explicitly uses names such as `ball_filter/ball_position` for its outputs.

### Subscribers, Caches, and Fusion Streams

Nodes choose how to consume each input:

| Input style | Usage | Example |
| --- | --- | --- |
| Subscriber | Await the next queued message with `recv().await`. | Detection receives `inputs/left_image`. |
| Cache | Retain a bounded history and query it with `get_latest()` or `get_nearest(time)`. | The ball filter reads field dimensions and camera matrices. |
| Future map | Buffer several announced streams and release time-ordered batches. | The ball filter combines odometry and detected objects. |

A cache can use `.with_stamp(...)` to index messages by their sensor timestamp rather than their transport timestamp.
The ball filter does this for camera matrices, so a detection can use camera geometry from near the image capture time.

`ros-z-streams` adds an announcement protocol for delayed computation. A producer first announces that it will publish a result for a particular timestamp, then publishes the result after computing it.
The announcement is sent on `<topic>/announce` and is matched to the data using the publisher identity and sequence number.

A `FutureMap` combines these streams into a timestamp-keyed map. Its safe-time boundary accounts for outstanding announcements and each stream's configured transit safety lag.
`recv().await` returns two parts:

- **Persistent:** finalized events to apply to long-lived algorithm state in timestamp order.
- **Temporary:** buffered events that have not crossed the safe-time boundary yet.

An entry contains an `Option` for each input stream. Odometry and detections need not arrive at identical timestamps, and a batch can have no persistent events yet.
This ordering depends on producers using the announcement protocol and on appropriate transit-lag settings.

## Outputs and Timestamps

Outputs are also explicit. A node creates a publisher for each topic and calls `publisher.publish(&value).await?` when a result is ready.
ROS-Z serializes the message and distributes it to subscribers; downstream nodes choose when to process it.
An internal struct such as `BallFilterOutput` just groups local results. Its fields are not published automatically or as one atomic transaction.

Distinguish **the time a result describes** from **the time it is delivered**.
Ordinary `publish` uses the current node clock for the source timestamp. When a result describes an earlier sensor or filter time, use `publish_with_source_time(&value, time)` to preserve that time in message metadata.
Payloads can additionally carry their own timestamps, for example in a `TimeWrapper` or image header.

These timestamp channels have different uses:

| Timestamp | Where it is used |
| --- | --- |
| ROS-Z source time | Publication attachment metadata, available through `recv_with_metadata()`. |
| Zenoh transport time | The default index for a cache; it does not automatically use ROS-Z attachment source time. |
| Payload sensor time | An image header or `TimeWrapper.time`; use `.with_stamp(...)` to index a cache by it. |
| Announcement time | The timestamp supplied to `announce(time)` and used to order a future-map stream. |

Choose the appropriate timestamp explicitly when aligning sensors, estimates, and recordings.

## Parameters and Synchronous Work

Nodes bind typed parameters with `bind_parameter_as::<Parameters>(...)` and read snapshots for processing.
The runtime supports layered configuration and remote parameter access; [Twix](../tooling/twix.md) provides a parameter editor.
Whether an updated parameter takes effect immediately or requires rebuilding a resource depends on the node implementation.

CPU-intensive or blocking synchronous work is often wrapped in `tokio::task::block_in_place`.
This tells Tokio's multi-threaded runtime that the current worker will be occupied, allowing other work to be handed off.
The closure still runs synchronously: it does not become an async operation or a separately spawned task.

## Code Generation

The current node graph is wired explicitly in Rust through topic subscriptions, publications, and the startup list.
Compile-time macros still supply reusable infrastructure: `#[derive(ros_z::Message)]` generates message metadata/schema support, Serde derives provide serialization support, and `ros-z-streams` macros implement combinations of stream types.

## Where to Continue

- [Vision](perception/vision.md): the image-to-detection path.
- [Localization](perception/localization.md): visual/inertial estimates and coordinate-frame transformations.
- [Filters](perception/filters.md): a conceptual introduction to the ball filter and its loop.
- [Behavior](behavior/overview.md) and [motion](motion/overview.md): consuming state and issuing robot commands.
- `crates/ros-z/README.md`: middleware quick start and endpoint API examples.
- `crates/ros-z-streams/src/future_map.rs` and `future_queue.rs`: timestamp ordering and announcements.
- `crates/nodes/ball_filter/src/lib.rs`: a concrete example of input setup, processing, and output publication.
- [Debugging](../tooling/debugging.md): inspecting nodes, topics, and parameters with `rosz`, and attaching GDB/LLDB.
