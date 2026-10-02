# HSL Network and GameController

The current stack bridges UDP game/team traffic into ROS-Z topics.
`message_handler` owns the network endpoint; other nodes consume parsed messages
and publish outgoing requests rather than opening their own UDP sockets.
See the [robotics overview](../overview.md) for topic namespaces and subscription mechanics.

## UDP Bridge

`crates/nodes/message_handler/src/lib.rs` binds the parameter group
`message_receiver` and creates an `hsl_network::endpoint::Endpoint` at startup.
The base configuration in `etc/parameters/base/message_receiver.json5` specifies:

| Setting | Base value | Purpose |
| --- | --- | --- |
| `ports.game_controller_state` | `3838` | Receive GameController state datagrams. |
| `ports.game_controller_return` | `3939` | Send return messages to the GameController's IP. |
| `ports.hsl` | `10024` | Receive and broadcast HULK team messages. |
| `ports.hsl_broadcast_address.octets` | `[10, 0, 255, 255]` | Destination address for team broadcasts. |

Location/robot parameter layers can override these defaults. Socket configuration
is read once when the endpoint is created; changes require restarting the node.
The endpoint parses the GameController wire format and uses bincode for HULK
messages. Invalid datagrams are logged and discarded.

The ROS-Z side has two topics:

- `inputs/message`: `TimeWrapper<IncomingMessage>`, stamped with the local clock
  after the UDP message is read. This is a local receipt time, not a sender sensor timestamp.
- `outputs/message`: `OutgoingMessage` requests, serialized and sent by the endpoint.
  GameController requests carry a destination address; HSL requests use the configured broadcast address.

## Incoming State

`message_filter` waits for `player_number`, drops HSL State messages from that
same player number, and publishes the remaining messages on `filtered_message`,
preserving the wrapper's receipt time.

- `game_controller_filter` extracts GameController state and sender contacts,
  publishing `game_controller_state` and `game_controller_address`.
  `game_controller_state_filter` produces `filtered_game_controller_state` for
  downstream game-state and behavior processing.
- `player_states_receiver` extracts teammate State messages into `player_states`.
  It removes penalized teammates and expires entries according to
  `player_states_receiver.maximum_age`, including on a 100 ms expiry timer.
  Behavior uses these poses for Voronoi task assignment and support positioning.
- `search_suggestor` consumes team messages directly to update its ball-search heatmap.

At main revision `0711900e0`, dedicated team-ball reception is implemented as
`team_ball_filter`, replacing the earlier `team_ball_receiver` placeholder.
It consumes typed `player_states` and `filtered_game_controller_state`, not
bare network messages. On input changes and a 100 ms timer it publishes:

- `team_ball`: the received Field-frame ball with the newest `last_seen`, if
  its age is valid and below `team_ball_filter.maximum_age`; otherwise `None`.
- `team_balls`: age-filtered per-player balls, published only when subscribed.

This selects a recent report rather than statistically fusing several reports.
Penalty shootout and penalty-kick substates suppress `team_ball` by publishing
`None`; that branch skips publication of `team_balls`, so an older debug value
can remain visible. The search heatmap also uses teammate reports directly.

## Outgoing State

[Behavior](../behavior/overview.md#team-communication) creates outgoing requests:

- GameController return messages are interval-limited and require a known
  GameController address. They report player number, pose, fall status, and ball
  position/age when available. Missing pose information uses the default transform;
  `fallen` is true only when an SDK fall state is present and is not `IsReady`.
  Missing fall state therefore reports `fallen: false` in this revision.
- HSL State messages are sent while Playing, with a known field pose, after the
  send cooldown, and only with a known remaining game message budget at or above
  the configured stop-sending threshold. They contain player number, pose, and
  optional Field-frame ball position/age.

Timing and budget settings are under `behavior_node.network` in
`etc/parameters/base/behavior_node.json5`.
UDP receipt and ROS-Z publication do not guarantee teammate delivery.

## Implementation and Inspection

- `crates/hsl_network/src/endpoint.rs`: sockets, parsing, and sending.
- `crates/nodes/message_handler` and `message_filter`: ROS-Z bridge and self-message filtering.
- `crates/nodes/player_states_receiver`: teammate state and expiry.
- `crates/nodes/team_ball_filter`: team-ball selection, expiry, and penalty suppression.
- `crates/nodes/behavior_node/src/send_message.rs`: outgoing State and return messages.

Inspect `inputs/message`, `filtered_message`, `outputs/message`, `player_states`,
`team_ball`, `team_balls`, `game_controller_address`, and
`filtered_game_controller_state` with Twix.
