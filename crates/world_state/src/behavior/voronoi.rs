use coordinate_systems::Field;
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Pose2, point};
use types::behavior_tree::Status;
use voronoi::{VoronoiBounds, VoronoiGrid};

use crate::behavior::node::Blackboard;

pub fn calculate_voronoi_grid(blackboard: &mut Blackboard) -> Status {
    if blackboard.voronoi_map.is_some() {
        return Status::Success;
    }

    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field {
        let field_dimensions = &blackboard.field_dimensions;
        let voronoi_parameters = &blackboard.parameters.voronoi;
        let obstacles = &blackboard.world_state.obstacles;
        let rule_obstacles = &blackboard.world_state.rule_obstacles;

        let sites = collect_sites(blackboard, ground_to_field.as_pose());
        for (pose, _) in &sites {
            blackboard.voronoi_inputs.push(*pose);
        }

        let length_half = field_dimensions.length / 2.0;
        let width_half = field_dimensions.width / 2.0;
        let border_strip_width = field_dimensions.border_strip_width;

        let bounds = VoronoiBounds {
            grid_min: point!(
                -length_half - border_strip_width,
                -width_half - border_strip_width
            ),
            grid_max: point!(
                length_half + border_strip_width,
                width_half + border_strip_width
            ),
            centroid_min: point!(-length_half, -width_half),
            centroid_max: point!(length_half, width_half),
        };

        let mut map = VoronoiGrid::new(bounds, voronoi_parameters.grid_resolution);
        map.initialize_obstacles(obstacles, rule_obstacles, ground_to_field);
        map.multi_source_dijkstra(&sites, voronoi_parameters.orientation_bias);
        blackboard.voronoi_map = Some(map);
        Status::Success
    } else {
        Status::Failure
    }
}

fn collect_sites(
    blackboard: &Blackboard,
    robot_pose: Pose2<Field>,
) -> Vec<(Pose2<Field>, PlayerNumber)> {
    let robot_player_number = blackboard.world_state.robot.player_number;
    let mut sites = vec![(robot_pose, robot_player_number)];

    for (player_number, player_state) in blackboard.world_state.player_states.iter() {
        if let Some(player_state) = player_state
            && player_number != robot_player_number
        {
            sites.push((player_state.pose, player_number));
        }
    }

    sites
}
