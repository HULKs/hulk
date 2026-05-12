use coordinate_systems::Field;
use hsl_network_messages::PlayerNumber;
use linear_algebra::Pose2;
use types::behavior_tree::Status;
use voronoi::VoronoiGrid;

use crate::behavior::node::Blackboard;

pub fn calculate_voronoi_grid(blackboard: &mut Blackboard) -> Status {
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field {
        let field_dimensions = &blackboard.field_dimensions;
        let voronoi_parameters = &blackboard.parameters.voronoi;
        let obstacles = &blackboard.world_state.obstacles;
        let rule_obstacles = &blackboard.world_state.rule_obstacles;

        let sites = collect_sites(blackboard, ground_to_field.as_pose());
        for (pose, _) in &sites {
            blackboard.voronoi_inputs.push(*pose);
        }

        let mut map = VoronoiGrid::new(
            field_dimensions.length,
            field_dimensions.width,
            voronoi_parameters.padding,
            voronoi_parameters.grid_resolution,
        );
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
