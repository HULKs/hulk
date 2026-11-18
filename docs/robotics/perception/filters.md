# Ball and obstacle tracking

## Local ball

`ball_filter` projects camera detections into `Ground` and maintains moving and resting ball hypotheses.
Odometry accounts for motion of the robot between updates.
`ball_filter/ball_percepts` contains individual projected observations, while `ball_filter/ball_position` contains the selected local estimate when available.  
For kicking, `visual_kick_ball_selector` selects a nearby percept and publishes it on `visual_kick/ball_position`.
[Ball filter](https://github.com/HULKs/hulk/blob/main/crates/nodes/ball_filter/src/lib.rs) · [Ball hypotheses](https://github.com/HULKs/hulk/blob/main/crates/nodes/ball_filter/src/hypothesis.rs) · [Visual kick ball selector](https://github.com/HULKs/hulk/blob/main/crates/nodes/visual_kick_ball_selector/src/lib.rs)

## Team ball and composed ball state

`team_ball_filter` reads ball reports from [teammate states](hsl_network.md#receiving-state-reports).
It selects the report with the most recent estimated observation time and checks its age before publishing `team_ball`.
Team ball data supplies a position in `Field` and an observation age. Its velocity is initialized to zero when the report is decoded.  
[Team ball filter](https://github.com/HULKs/hulk/blob/main/crates/nodes/team_ball_filter/src/lib.rs) · [Ball report conversion](https://github.com/HULKs/hulk/blob/main/crates/types/src/ball_position.rs)

`ball_state_composer` uses the local estimate when available and falls back to the team estimate.
With `ground_to_field` available, it publishes `ball_state` with positions in `Ground` and `Field`, velocity in `Ground`, observation time, and source.
Rule-generated ball positions are published separately on `rule_ball_state`.  
[Ball state composer](https://github.com/HULKs/hulk/blob/main/crates/nodes/ball_state_composer/src/lib.rs) · [Ball state type](https://github.com/HULKs/hulk/blob/main/crates/types/src/world_state.rs) · [Coordinate frames](localization.md#coordinate-frames)

## Obstacles

`obstacle_filter` combines projected detections, teammate pose reports, and field geometry.
It publishes `obstacles`, with a position in `Ground`, a kind, and radii at foot and hip height.
Visual robot detections share the `Robot` class. Identified teammate poses come from `player_states`.
Rule-based restrictions are available separately on `rule_obstacles`.  
[Obstacle filter](https://github.com/HULKs/hulk/blob/main/crates/nodes/obstacle_filter/src/lib.rs) · [Obstacle type](https://github.com/HULKs/hulk/blob/main/crates/types/src/obstacles.rs) · [Rule obstacles](https://github.com/HULKs/hulk/blob/main/crates/nodes/rule_obstacle_composer/src/lib.rs) · [Path planning](../motion/path_planning.md)
