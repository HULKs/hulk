# Hulkz CLI

Command-line tool for introspection and debugging of the hulkz middleware.

## Installation

```bash
cargo install --path tools/hulkz-cli
```

Or build from the workspace:

```bash
cargo build -p hulkz-cli --release
```

## Usage

```bash
hulkz [OPTIONS] <COMMAND>
```

### Global Options

| Option | Description |
|--------|-------------|
| `-n, --namespace <NAME>` | Namespace to operate in (default: `default`, env: `HULKZ_NAMESPACE`) |
| `--json` | Output in JSON format for machine parsing |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

## Commands

### List Resources

List nodes, publishers, or sessions in the network.

```bash
# List all nodes in the namespace
hulkz list nodes

# List all publishers (optionally filter by node)
hulkz list publishers
hulkz list publishers --node navigation

# List all sessions
hulkz list sessions
```

### Watch Events

Watch for resource changes in real-time.

```bash
# Watch for node join/leave events
hulkz watch nodes

# Watch for publisher advertise/unadvertise events
hulkz watch publishers

# Watch for session join/leave events
hulkz watch sessions
```

### View (Subscribe)

Subscribe to a topic and print messages as they arrive. Subscribes to the **view plane** (JSON) which is ideal for debugging and introspection.

```bash
# Subscribe to a local topic
hulkz view camera/image

# Subscribe to a global topic
hulkz view /fleet_status

# Subscribe to a private topic
hulkz view ~/debug/state

# Exit after receiving 10 messages
hulkz view sensor/data --count 10
```

### Parameter Operations

Get and set parameters on nodes.

```bash
# List all parameters in the namespace
hulkz param list

# List parameters for a specific node (private parameters only)
hulkz param list --node navigation

# Get a parameter value (searches all nodes)
hulkz param get max_speed

# Get a parameter from a specific node
hulkz param get max_speed --node navigation

# Set a parameter value
hulkz param set max_speed 2.5

# Set a parameter on a specific node
hulkz param set max_speed 2.5 --node navigation
```

### Topic Info

Show information about a topic including publishers and message schema.

```bash
hulkz info camera/image
```

### Graph

Show a network topology overview of all sessions, nodes, and their publishers.

```bash
hulkz graph
```

Example output:

```
Network Topology
================

Session: abc123@robot1
  Node: navigation
    Publishers:
      - odometry (local)
      - ~/debug/path (private)
  Node: perception
    Publishers:
      - detections (local)

Session: def456@robot2
  Node: coordinator
    Publishers:
      - /fleet_status (global)
```

## JSON Output

All commands support `--json` for machine-readable output:

```bash
hulkz --json list nodes
```

```json
[
  {"name": "navigation", "session_id": "abc123@robot1"},
  {"name": "perception", "session_id": "abc123@robot1"}
]
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `HULKZ_NAMESPACE` | Default namespace (overridden by `-n/--namespace`) |

## Examples

### Monitor a Robot

```bash
# In terminal 1: Watch for node changes
hulkz -n chappie watch nodes

# In terminal 2: Watch for publishers
hulkz -n chappie watch publishers

# In terminal 3: View a topic
hulkz -n chappie view odometry
```

### Debug Parameters

```bash
# Get a parameter value
hulkz -n chappie param get max_speed --node navigation

# Tune in real-time
hulkz -n chappie param set max_speed 1.5 --node navigation
```

## See Also

- [`hulkz`](../../crates/hulkz/) - The core library crate
- [Architecture Guide](../../crates/hulkz/ARCHITECTURE.md) - Key concepts and design
