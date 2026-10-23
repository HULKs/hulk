use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Pose2, point};
use types::{behavior_tree::Status, obstacles::ObstacleKind};
use voronoi::VoronoiGrid;

use crate::node::Blackboard;

pub fn calculate_voronoi_grid(blackboard: &mut Blackboard) -> Status {
    let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field else {
        return Status::Failure;
    };

    let sites = collect_sites(blackboard, ground_to_field.as_pose());
    calculate_voronoi_grid_from_sites(blackboard, ground_to_field, &sites)
}

pub fn calculate_voronoi_grid_without_goalkeeper(blackboard: &mut Blackboard) -> Status {
    let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field else {
        return Status::Failure;
    };

    let mut sites = collect_sites(blackboard, ground_to_field.as_pose());
    let goalkeeper = blackboard.parameters.goalkeeper.player_number;
    sites.retain(|(_, player_number)| *player_number != goalkeeper);

    calculate_voronoi_grid_from_sites(blackboard, ground_to_field, &sites)
}

fn calculate_voronoi_grid_from_sites(
    blackboard: &mut Blackboard,
    ground_to_field: Isometry2<Ground, Field>,
    sites: &[(Pose2<Field>, PlayerNumber)],
) -> Status {
    let field_dimensions = &blackboard.field_dimensions;
    let voronoi_parameters = &blackboard.parameters.voronoi;
    // Teammate bodies must not penalize their own seeds in each other's local map.
    let obstacles: Vec<_> = blackboard
        .world_state
        .obstacles
        .iter()
        .filter(|obstacle| {
            !matches!(
                obstacle.kind,
                ObstacleKind::Robot | ObstacleKind::Person | ObstacleKind::Unknown
            ) || !sites.iter().any(|(pose, _)| {
                (ground_to_field * obstacle.position - pose.position()).norm()
                    <= obstacle
                        .radius_at_hip_height
                        .max(obstacle.radius_at_foot_height)
            })
        })
        .copied()
        .collect();
    let rule_obstacles = &blackboard.world_state.rule_obstacles;

    for (pose, _) in sites {
        blackboard.voronoi_inputs.push(*pose);
    }

    let length_half = field_dimensions.length / 2.0;
    let width_half = field_dimensions.width / 2.0;
    let padding = voronoi_parameters.padding;

    let grid_min = point!(-length_half - padding, -width_half - padding);
    let grid_max = point!(length_half + padding, width_half + padding);

    let mut map = VoronoiGrid::new(grid_min, grid_max, voronoi_parameters.grid_resolution);
    map.initialize_obstacles(&obstacles, rule_obstacles, ground_to_field);
    map.multi_source_dijkstra(sites);
    blackboard.voronoi_map = Some(map);
    Status::Success
}

pub(crate) fn collect_sites(
    blackboard: &Blackboard,
    robot_pose: Pose2<Field>,
) -> Vec<(Pose2<Field>, PlayerNumber)> {
    let robot_player_number = blackboard.player_number;
    let mut sites = vec![(robot_pose, robot_player_number)];

    for (player_number, player_state) in blackboard.world_state.player_states.iter() {
        if let Some(player_state) = player_state
            && player_number != robot_player_number
            && player_state.time <= blackboard.world_state.now
            && blackboard.world_state.now.duration_since(player_state.time)
                <= blackboard
                    .parameters
                    .kicking
                    .strategy
                    .passing
                    .maximum_receiver_age
            && player_state
                .inner
                .pose
                .position()
                .inner
                .iter()
                .all(|value| value.is_finite())
            && player_state.inner.pose.angle().is_finite()
            && blackboard
                .world_state
                .filtered_game_controller_state
                .as_ref()
                .is_none_or(|game| game.penalties[player_number].is_none())
        {
            sites.push((player_state.inner.pose, player_number));
        }
    }

    if let Some(replacement) = blackboard.replacement_goalkeeper_coordinator.keeper() {
        sites.retain(|(_, number)| *number != replacement);
    }
    sites
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use hsl_network_messages::Penalty;
    use types::{
        field_dimensions::FieldDimensions, obstacles::Obstacle, time_wrapper::TimeWrapper,
        world_state::PlayerState,
    };
    use voronoi::Ownership;

    use super::*;

    #[test]
    fn teammate_bodies_do_not_give_both_players_ball_ownership() {
        let poses = [
            (PlayerNumber::Two, Pose2::new(point![-1.0, -0.4], 0.0)),
            (PlayerNumber::Three, Pose2::new(point![-1.0, 0.4], 0.0)),
        ];
        let mut owners = Vec::new();
        for (number, pose) in poses {
            let mut board = Blackboard {
                field_dimensions: FieldDimensions::SPL_2025,
                ..Default::default()
            };
            board.world_state.now = board.world_state.now + Duration::from_secs(1);
            board.player_number = number;
            board.world_state.robot.ground_to_field = Some(pose.as_transform());
            board.parameters.voronoi.grid_resolution = 0.2;
            board.parameters.voronoi.padding = 0.5;
            board
                .parameters
                .kicking
                .strategy
                .passing
                .maximum_receiver_age = Duration::from_secs(3);
            for (peer, peer_pose) in poses {
                if peer == number {
                    continue;
                }
                board.world_state.player_states[peer] = Some(TimeWrapper {
                    time: board.world_state.now,
                    inner: PlayerState {
                        pose: peer_pose,
                        ..crate::test_support::player_state()
                    },
                });
                board.world_state.obstacles.push(Obstacle::robot(
                    pose.as_transform::<Ground>().inverse() * peer_pose.position(),
                    0.4,
                    0.4,
                ));
            }
            board.world_state.obstacles.extend([
                Obstacle::robot(
                    pose.as_transform::<Ground>().inverse() * point![2.0, 1.0],
                    0.3,
                    0.3,
                ),
                Obstacle::goal_post(
                    pose.as_transform::<Ground>().inverse() * point![-4.4, 1.4],
                    0.1,
                ),
            ]);
            assert_eq!(calculate_voronoi_grid(&mut board), Status::Success);
            let map = board.voronoi_map.unwrap();
            for blocked in [point![2.0, 1.0], point![-4.4, 1.4]] {
                assert_eq!(map.ownership_at(blocked), Some(Ownership::Blocked));
            }
            owners.push(map.ownership_at(point![1.0, 0.0]));
        }
        assert_eq!(owners, vec![Some(Ownership::Robot(PlayerNumber::Two)); 2]);
    }

    #[test]
    fn ownership_sites_exclude_stale_future_nonfinite_and_penalized_players() {
        let mut board = Blackboard {
            player_number: PlayerNumber::Two,
            ..Default::default()
        };
        board.world_state.now = board.world_state.now + Duration::from_secs(10);
        board
            .parameters
            .kicking
            .strategy
            .passing
            .maximum_receiver_age = Duration::from_secs(3);
        board.world_state.player_states.three = Some(TimeWrapper {
            time: board.world_state.now,
            inner: crate::test_support::player_state(),
        });
        assert_eq!(collect_sites(&board, Pose2::default()).len(), 2);
        for case in 0..4 {
            let mut unavailable = board.clone();
            let peer = unavailable
                .world_state
                .player_states
                .three
                .as_mut()
                .unwrap();
            match case {
                0 => peer.time = Default::default(),
                1 => peer.time = board.world_state.now + Duration::from_secs(1),
                2 => peer.inner.pose = point![f32::NAN, 0.0].into(),
                _ => {
                    let mut game = crate::test_support::filtered_game_controller_state();
                    game.penalties.three = Some(Penalty::PickUp {
                        remaining: Duration::from_secs(1),
                    });
                    unavailable.world_state.filtered_game_controller_state = Some(game);
                }
            }
            assert_eq!(
                collect_sites(&unavailable, Pose2::default()).len(),
                1,
                "case {case}"
            );
        }
    }
}
