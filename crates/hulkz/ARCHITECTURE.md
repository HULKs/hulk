# Hulkz Architecture

A concise overview of hulkz concepts and design decisions.

## Key Space

Hulkz partitions the Zenoh key space into **5 functional planes**:

```
hulkz/
├── data/      CDR-encoded production data
├── view/      JSON mirror of data (lazy serialization)
├── param/     Configuration (read/write branches)
├── graph/     Liveliness-based discovery
└── cmd/       RPC services (planned)
```

This separation ensures wildcard subscriptions are safe (e.g., `hulkz/data/**` won't trigger services).

## Scopes

Within each plane, paths are organized by visibility scope:

| Scope | Prefix | Key Pattern | Use Case |
|-------|--------|-------------|----------|
| Global | `/` | `{plane}/global/{path}` | Fleet-wide coordination |
| Local | (none) | `{plane}/local/{namespace}/{path}` | Robot-internal (default) |
| Private | `~/` | `{plane}/private/{namespace}/{node}/{path}` | Node-internal debug data |

## Core Types

```
Session ──creates──> Node ──creates──> Publisher
                          ──creates──> Subscriber
                          ──creates──> Buffer
                          ──creates──> Parameter
```

- **Session**: Zenoh connection with namespace context. Entry point for discovery.
- **Node**: Unit of computation. Registers in graph plane for discovery.
- **Publisher**: Dual-plane publishing (CDR + JSON). View plane is lazy.
- **Subscriber**: Receives from data or view plane.
- **Buffer**: Temporal message storage for timestamp-based lookups.
- **Parameter**: Runtime-configurable value with remote read/write.

## Timestamps

All published messages carry explicit timestamps. The `put()` API requires a timestamp parameter to enforce correct temporal semantics:

- **Sensor data**: `session.now()` — capture time
- **Derived data**: Source message's timestamp — maintains temporal coherence

This distinction matters for sensor fusion, replay, and debugging.

## Temporal Alignment

`Buffer` stores recent messages indexed by timestamp for lookup:

```rust
let (imu, driver) = node.buffer::<Imu>("imu", 200).await?;
tokio::spawn(driver);

// On camera frame arrival, look up temporally-aligned IMU
let imu_msg = imu.lookup_nearest(&camera_msg.timestamp).await;
```

Lookup methods: `lookup_nearest`, `lookup_before`, `lookup_after`, `lookup_interval`.

## Discovery

Sessions, nodes, and publishers register via Zenoh liveliness tokens:

```
hulkz/graph/sessions/{namespace}/{session_id}
hulkz/graph/nodes/{namespace}/{node}
hulkz/graph/publishers/{namespace}/{node}/{scope}/{path}
```

List current state or watch for changes:

```rust
let nodes = session.list_nodes().await?;
let (watcher, driver) = session.watch_nodes().await?;
```

## Parameters

Parameters expose values for remote query/update via the param plane:

```
hulkz/param/read/{scope}/{namespace}/[{node}/]{path}   ← Query current value
hulkz/param/write/{scope}/{namespace}/[{node}/]{path}  ← Update value
```

Updates are validated locally and broadcast to subscribers.

## Dual-Plane Publishing

Publishers send to both planes:

1. **Data plane** (`hulkz/data/...`): CDR encoding, always sent
2. **View plane** (`hulkz/view/...`): JSON encoding, lazy (only when subscribed)

This enables high-performance production data while allowing CLI/debug tools to inspect messages as JSON without modifying the publisher.

## Runtime Agnostic

Hulkz does not spawn tasks internally. APIs that require background processing return `(Handle, Driver)` tuples. The caller spawns the driver on their runtime:

```rust
let (buffer, driver) = node.buffer::<T>("topic", 100).await?;
tokio::spawn(driver);  // Caller controls spawning
```

This gives full control over task management and error handling.

## CLI

The `hulkz-cli` tool provides introspection:

```bash
hulkz list nodes          # List active nodes
hulkz view camera/image   # Subscribe to view plane (JSON)
hulkz param get max_speed # Query parameter
hulkz graph               # Show topology
```
