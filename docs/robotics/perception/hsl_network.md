# Team communication

## Transport

The HSL UDP endpoint handles GameController packets and team broadcasts.
`message_handler` connects this endpoint to the ROS-Z topics `inputs/message` and `outputs/message`.
`message_filter` removes the robot's own team-state reports. GameController packets and teammate reports then feed their respective state filters.  
[UDP endpoint](https://github.com/HULKs/hulk/blob/main/crates/hsl_network/src/endpoint.rs) · [Message handler](https://github.com/HULKs/hulk/blob/main/crates/nodes/message_handler/src/lib.rs) · [Message filter](https://github.com/HULKs/hulk/blob/main/crates/nodes/message_filter/src/lib.rs) · [GameController filter](https://github.com/HULKs/hulk/blob/main/crates/nodes/game_controller_filter/src/lib.rs)

## Sending state reports

| `StateMessage` field | Contents |
| --- | --- |
| `player_number` | Sender's player number. |
| `pose` | Sender's pose in `Field`. |
| `head_yaw` | Commanded head yaw. |
| `ball_position` | Optional ball position in `Field` and observation age. |

[Wire schema](https://github.com/HULKs/hulk/blob/main/crates/hsl_network_messages/src/lib.rs) · [State message construction](https://github.com/HULKs/hulk/blob/main/crates/nodes/behavior_node/src/send_message.rs)

Behavior sends state reports in `Playing` when it has a field pose, GameController state, and an elapsed send interval.
It computes the interval from the remaining match time, the number of unpenalized players, and the GameController's remaining message count minus a configured reserve.
The remaining match time includes the configured second-half duration during the first half of normal play or extra time.
A zero usable message count, player count, or remaining time suppresses sending.

```text
usable_messages = max(remaining_number_of_messages - reserve, 0)
send_interval = remaining_match_time * unpenalized_players / usable_messages
```

Behavior exposes the computed interval on `hsl_state_message_send_interval`.
The search suggestor uses it when assessing teammate freshness.
[Send conditions and interval](https://github.com/HULKs/hulk/blob/main/crates/nodes/behavior_node/src/send_message.rs) · [Behavior](../behavior/overview.md#team-communication)

## Receiving state reports

The receiver stamps reports with its local reception time.
It estimates the ball observation time by subtracting the transmitted age from that timestamp. Packet delay makes this estimate later than the original observation time.
The receiver rejects non-finite reports and removes states for penalized players.

| Topic | Contents |
| --- | --- |
| `player_states` | Latest teammate reports, subject to a configurable expiry. |
| `last_received_player_states` | Latest accepted report for each teammate, retained until replaced or removed after a penalty. Consumers apply their own freshness checks. |

UDP datagrams can be delayed, reordered, or lost.  
[Reception timestamps](https://github.com/HULKs/hulk/blob/main/crates/nodes/message_handler/src/lib.rs) · [Player states](https://github.com/HULKs/hulk/blob/main/crates/nodes/player_states_receiver/src/lib.rs) · [Ball age conversion](https://github.com/HULKs/hulk/blob/main/crates/types/src/ball_position.rs) · [Team ball](filters.md#team-ball-and-composed-ball-state)
