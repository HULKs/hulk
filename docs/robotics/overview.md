# Robotics overview

The HULKs software on the Booster K1 runs as a set of Rust nodes launched by `hulk_ros_z`.
The nodes publish and subscribe to typed ROS-Z topics. Operations such as motion inference use ROS-Z services.
Each robot runs in its own namespace and loads base, location-specific, and robot-specific parameter layers.
Topic names in this section are relative to that namespace unless indicated otherwise.  
[K1 launcher](https://github.com/HULKs/hulk/blob/main/tools/k1-setup/launch-hulk) · [Runtime and node registration](https://github.com/HULKs/hulk/blob/main/crates/hulk_ros_z/src/main.rs)

| Area | Data flow | Documentation |
| --- | --- | --- |
| Perception and state estimation | Camera, IMU, kinematics, and odometry feed object detection, localization, and tracking. Whistle detections feed the game-state filter. | [Vision](perception/vision.md), [Localization](perception/localization.md), [Ball and obstacle tracking](perception/filters.md), [Audio](perception/audio.md) |
| Team and game state | The HSL UDP endpoint handles GameController packets and team broadcasts. Nodes expose the resulting match and teammate state on ROS-Z topics. | [Team communication](perception/hsl_network.md) |
| Behavior | The behavior tree reads local state, chooses an action, and publishes `behavior/motion_command`. | [Behavior](behavior/overview.md) |
| Motion and hardware | Motion services turn the behavior request into `commands/robot_command`. The hardware interface applies the requested mode and publishes low-level joint commands in custom mode. | [Motion](motion/overview.md), [Fall detection](fall-detection.md) |

## Inspecting the data flow

| Topic | What it shows |
| --- | --- |
| `ground_to_field` | Transform from the robot-relative plane to the team's field frame. |
| `ball_state` | Selected ball estimate, including its source and observation time. |
| `player_states` | Recent reports from teammates. |
| `behavior/trace` and `behavior/blackboard` | Behavior-tree decisions and the state used to make them. |
| `behavior/motion_command` | The action requested by behavior. |
| `commands/robot_command` | Control mode and joint commands produced by motion. |

These topics can be inspected live in Twix. MCAP replay exposes the topics included in a recording.  
[Twix](../tooling/twix.md) · [Behavior simulator](../tooling/behavior_simulator.md)
