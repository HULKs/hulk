# ros-z

Zenoh-native robotics middleware in Rust.

`ros-z` provides typed publish/subscribe, services, graph discovery, parameters, runtime clocks, shared-memory payload support, and CDR serialization without ROS 2 C/C++ runtime dependencies.

> [!NOTE]
> This crate is part of the HULKs workspace.
> APIs are still evolving while the native `ros-z` stack is integrated.

## What It Includes

- `ros-z`: the main runtime crate for contexts, nodes, pub/sub, services, parameters, graph discovery, QoS, time, and shared memory.
- `ros-z-cdr`: CDR serialization primitives and Serde integration.
- `ros-z-protocol`: Zenoh key-expression formats and protocol entities.
- `ros-z-schema`: schema and type-shape support for generated and dynamic data.
- `ros-z-derive`: `#[derive(Message)]` support for typed messages.
- `ros-z-streams`: future queues and maps for timestamped sensor-fusion streams.
- `ros-z-cli`: graph, schema, topic, and parameter inspection commands.
- `ros-z-debug`: read-only debug subscriptions with retained samples and JSON views.

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

Builders that create runtime resources are async.
Build contexts, nodes, publishers, subscribers, services, and caches inside a Tokio-compatible runtime.
Endpoint factories return builders directly and defer schema, type, and graph-name validation until `.build().await`.

Core endpoint builders use one `?` at build time.
Service examples assume a user-defined `AddTwoInts` type that implements `Service` and `ServiceTypeInfo`:

```rust,ignore
let publisher = node.publisher::<String>("/chatter").build().await?;
let subscriber = node.subscriber::<String>("/chatter").build().await?;
let cache = node.subscriber::<String>("/chatter").cache(200).build().await?;
let server = node.service_server::<AddTwoInts>("add_two_ints").build().await?;
let client = node.service_client::<AddTwoInts>("add_two_ints").build().await?;
```

## Native CDR byte fields

`Message` derives field-level CDR encoding and decoding, composing through nested
structs, enums and containers without changing their alignment origin or schema.
Use `ros_z::ZSlice` for immutable owned byte sequences:

```rust
use ros_z::{Message, ZSlice};
use serde::{Deserialize, Serialize};

#[derive(Message, Serialize, Deserialize)]
struct Frame {
    sequence: u64,
    #[serde(with = "ros_z::serde_zslice")]
    data: ZSlice,
}
```

Move a `Vec<u8>` into a `ZSlice` with `.into()`. Ordinary `publish(&frame)` and
`recv()` retain pixel owners when the byte field occupies one payload segment;
fragmented byte fields are copied individually. Decoding borrowed bytes must
copy `ZSlice` fields. Explicit SHM publishing and contiguous serialization copy
bytes into their destination; SHM is not enabled by default.

`CdrCodec<T>` is the single typed CDR codec (also exported by the prelude).
Field composition lives on the required `Message::encode_cdr` and `decode_cdr`
methods, not codec defaults. Foreign-type implementations (including nalgebra
and ndarray) use internal Serde field bridges without violating orphan rules.
Bridges preserve the enclosing alignment origin and consume exactly their fields.
Custom `WireEncoder`/`WireDecoder` implementations remain supported.

Wire-affecting Serde attributes generate a whole-value bridge automatically,
still using `CdrCodec<T>`. For hand-written Serde implementations, add
`#[message(serde)]` to retain their custom wire semantics. Serde-only fields do not retain
`ZSlice` owners. The field adapter above also supports JSON tooling.
For a field that implements `MessageSchema` and Serde but not `Message`, put
`#[message(serde)]` on that field to bridge only it; neighboring native byte
fields still retain their owners.
Generic bridged fields require `MessageSchema`, `Serialize`, and `DeserializeOwned`,
not `Message`; their generic type names use Rust's `type_name`. Native fields
require `Message`. Field-type bounds support const-generic matrices wherever the
concrete nalgebra dimensions implement `Message`; recursive fields do not add
self-referential bounds.

SHM publishing counts encoded size without copying payloads, then encodes directly
into an exact-sized allocation, reusing the size calculated for threshold selection.
Explicit calls with inaccurate size hints retry when necessary. Heap/ZSlice publishing
does not copy byte fields. Exhausted SHM pools attempt garbage collection without waiting,
then publishers fall back to heap encoding, so retained cache entries cannot stall
pool allocation indefinitely.

Dynamic subscriptions also retain contiguous byte fields as `DynamicValue::Bytes(ZSlice)`;
construct these with `DynamicValue::Bytes(vec.into())`. Fragmented fields require a copy.
Their JSON rendering is unchanged, including compact byte previews.

## Name Rules

`ros-z` uses Zenoh-native concrete graph names.
Namespace, node, topic, and service components may start with digits, so a namespace such as `/42` is valid.

Names must still qualify to concrete Zenoh keys.
Components cannot be empty and cannot contain `/`, `%`, `#`, `$`, `?`, or `*`.
Slash separates components, `%` is reserved by the current ros-z liveliness identity encoding, and `*` is a selector wildcard rather than a concrete endpoint character.

Applications should pass graph names through unchanged except for adding a leading slash where they accept a bare namespace.
For example, a bare robot number `42` becomes namespace `/42`; invalid names such as `robot%01` are rejected instead of rewritten.

## Examples

Run examples from the workspace root:

```bash
cargo run -p ros-z --example custom_message_status_publisher
cargo run -p ros-z --example custom_message_status_subscriber
cargo run -p ros-z --example service_server
cargo run -p ros-z --example service_client
cargo run -p ros-z --example zenoh_router
```

Dynamic message examples live under `examples/dynamic_message` and are exposed as `dynamic_message_basic`, `dynamic_message_serialization`, and `dynamic_message_interop`.

## Common Imports

Use the prelude for application code:

```rust
use ros_z::prelude::*;
```

Import lower-level types from their modules when you need narrower control, such as `ros_z::pubsub`, `ros_z::service`, `ros_z::parameter`, `ros_z::time`, or `ros_z::shm`.

## Testing

Useful focused checks while working on `ros-z`:

```bash
cargo test -p ros-z
cargo test -p ros-z-cdr
cargo test -p ros-z-streams
cargo fmt --check
```

Run `cargo test --workspace` before broad integration changes.
