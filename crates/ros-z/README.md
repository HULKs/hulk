# ros-z

Zenoh-native robotics middleware in Rust.

`ros-z` provides typed publish/subscribe, services, graph discovery, parameters,
runtime clocks, shared-memory payload support, and CDR serialization without ROS 2
C/C++ runtime dependencies.

> [!NOTE]
> This crate is part of the HULKs workspace. APIs are still evolving while the
> native `ros-z` stack is integrated.

## What It Includes

- `ros-z`: the main runtime crate for contexts, nodes, pub/sub, services,
  parameters, graph discovery, QoS, time, and shared memory.
- `ros-z-cdr`: CDR serialization primitives and Serde integration.
- `ros-z-protocol`: Zenoh key-expression formats and protocol entities.
- `ros-z-schema`: schema and type-shape support for generated and dynamic data.
- `ros-z-derive`: `#[derive(Message)]` support for typed messages.
- `ros-z-streams`: future queues and maps for timestamped sensor-fusion streams.
- `ros-z-cli`: graph, schema, topic, parameter inspection, and MCAP recording commands.
- `ros-z-debug`: read-only debug subscriptions with retained samples and JSON views.
- `ros-z-recording`: MCAP exact-topic recording backend used by `rosz record`.

## Quick Start

```rust
use ros_z::prelude::*;

async fn demo() -> ros_z::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("talker").build().await?;
    let publisher = node.publisher::<String>("/chatter").build().await?;

    publisher.publish(&"hello".to_owned()).await?;
    Ok(())
}
```

Builders that create runtime resources are async. Build contexts, nodes,
publishers, subscribers, services, and caches inside a Tokio-compatible runtime.
Endpoint factories return builders directly and defer schema, type, and graph-name
validation until `.build().await`.

Core endpoint builders use one `?` at build time. Service examples assume a
user-defined `AddTwoInts` type that implements `Service` and `ServiceTypeInfo`:

```rust,ignore
let publisher = node.publisher::<String>("/chatter").build().await?;
let subscriber = node.subscriber::<String>("/chatter").build().await?;
let cache = node.subscriber::<String>("/chatter").cache(200).build().await?;
let server = node.service_server::<AddTwoInts>("add_two_ints").build().await?;
let client = node.service_client::<AddTwoInts>("add_two_ints").build().await?;
```

## Name Rules

`ros-z` uses Zenoh-native concrete graph names. Namespace, node, topic, and
service components may start with digits, so a namespace such as `/42` is valid.

Names must still qualify to concrete Zenoh keys. Components cannot be empty and
cannot contain `/`, `%`, `#`, `$`, `?`, or `*`. Slash separates components, `%`
is reserved by the current ros-z liveliness identity encoding, and `*` is a
selector wildcard rather than a concrete endpoint character.

Applications should pass graph names through unchanged except for adding a
leading slash where they accept a bare namespace. For example,
`hulk_ros_z --robot 42` uses namespace `/42`; invalid names such as `robot%01`
are rejected instead of rewritten.

## Examples

Run examples from the workspace root:

```bash
cargo run -p ros-z --example custom_message_status_publisher
cargo run -p ros-z --example custom_message_status_subscriber
cargo run -p ros-z --example service_server
cargo run -p ros-z --example service_client
cargo run -p ros-z --example zenoh_router
```

Dynamic message examples live under `examples/dynamic_message` and are exposed as
`dynamic_message_basic`, `dynamic_message_serialization`, and
`dynamic_message_interop`.

## Common Imports

Use the prelude for application code:

```rust
use ros_z::prelude::*;
```

Import lower-level types from their modules when you need narrower control, such
as `ros_z::pubsub`, `ros_z::service`, `ros_z::parameter`, `ros_z::time`, or
`ros_z::shm`.

## Testing

Useful focused checks while working on `ros-z`:

```bash
cargo test -p ros-z
cargo test -p ros-z-cdr
cargo test -p ros-z-streams
cargo fmt --check
```

Run `cargo test --workspace` before broad integration changes.
