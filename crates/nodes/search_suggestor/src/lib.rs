use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc};

use color_eyre::Result;

use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2};
use ros_z::parameter;
use ros_z::{prelude::*, qos::QosDurability};
use types::time_wrapper::TimeWrapper;
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    messages::IncomingMessage,
    parameters::SearchSuggestorParameters,
    primary_state::PrimaryState,
};
use types::{ball_position, field_dimensions, parameters};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("search_suggestor").build().await?;

    let parameters = node.bind_parameter_as::<SearchSuggestorParameters>("search_suggestor")?;
    let field_dimensions_cache = node
        .create_cache::<FieldDimensions>("field_dimensions", 1)?
        .with_qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let _ball_position_sub = node
        .subscriber::<Option<BallPosition<Ground>>>("ball_filter/ball_position")?
        .build()
        .await?;
    let hypothetical_ball_positions_cache = node
        .create_cache::<Vec<HypotheticalBallPosition<Ground>>>("hypothetical_ball_positions", 10)?
        .with_stamp(|wrapper| wrapper.time)
        .build()
        .await?;
    let ground_to_field_cache = node
        .create_cache::<TimeWrapper<Isometry2<Ground, Field>>>("ground_to_field", 10)?
        .with_stamp(|wrapper| wrapper.time)
        .build()
        .await?;
    let primary_state_cache = node
        .create_cache::<PrimaryState>("primary_state", 1)?
        .with_qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let filtered_game_controller_state_sub = node
        .subscriber::<TimeWrapper<Option<FilteredGameControllerState>>>(
            "filtered_game_controller_state",
        )?
        .build()
        .await?;
    let network_message_sub = node
        .subscriber::<TimeWrapper<IncomingMessage>>("filtered_message")?
        .build()
        .await?;
    // TODO: do we need to directly publish ndarray here? Choose another type or manually implement
    // support for `Array: Message` in ros-z
    // let _heatmap_pub = node
    //     .publisher::<Array2<f32>>("ball_search_heatmap")
    //     .build()
    //     .await?;
    let suggested_search_position_pub = node
        .publisher::<Point2<Field>>("suggested_search_position")?
        .build()
        .await?;

    let (heatmap_length, heatmap_width) = (
        (context.field_dimensions.length
            * context.search_suggestor_configuration.cells_per_meter)
            .round() as usize,
        (context.field_dimensions.width
            * context.search_suggestor_configuration.cells_per_meter)
            .round() as usize,
    );
    let heatmap = Heatmap {
        map: Array2::ones((heatmap_length, heatmap_width))
            / (heatmap_length * heatmap_width) as f32,
        cells_per_meter: context.search_suggestor_configuration.cells_per_meter,
        last_maximum_heatmap_position: None,
        has_decided_for_heatmap_tile: false,
    };

    loop {
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        let field_dimensions = field_dimensions_cache.get_latest();

         self.update_heatmap(&context)?;

        tokio::select! {
            timed_ball_position = ball_position_sub.recv().await? => {}
            network_mesages = network_message_sub.recv().await? => {}
            filtered_game_controller_state = filtered_game_controller_state_sub.recv().await? => {}
        }


//TODO update heatmap function into tokio select to hand network messages, gamecontrollermessages and overall ballpositions separately and put them into the heatmap
// TODO also maybe put heatmap into separate file

        self.update_heatmap(&context)?;

        if !heatmap.has_decided_for_heatmap_tile {
            let suggested_search_index = self
                .heatmap
                .get_maximum_position(context.search_suggestor_configuration.minimum_validity);
            if suggested_search_index.is_some() {
                self.heatmap.has_decided_for_heatmap_tile = true;
            }
            heatmap.last_maximum_heatmap_position = suggested_search_index;
        } else if let Some(last_maximum_heatmap_index) = self.heatmap.last_maximum_heatmap_position
        {
            let global_max_value = heatmap
                .get_maximum_position(0.0)
                .map_or(0.0, |idx| self.heatmap.map[idx]);
            let current_tile_value = self.heatmap.map[last_maximum_heatmap_index];

            if current_tile_value
                < global_max_value
                    * context
                        .search_suggestor_configuration
                        .tile_switch_hysteresis
            {
                self.heatmap.has_decided_for_heatmap_tile = false;
            }
    }
}

fn create_kernel(alpha: f32) -> Array2<f32> {
    array![
        [alpha, alpha, alpha],
        [alpha, 1.0 - alpha, alpha],
        [alpha, alpha, alpha]
    ] / (1.0 + 7.0 * alpha)
}

#[derive(Deserialize, Serialize)]
struct Heatmap {
    map: Array2<f32>,
    cells_per_meter: f32,
    last_maximum_heatmap_position: Option<(usize, usize)>,
    has_decided_for_heatmap_tile: bool,
}

impl Heatmap {
    fn field_to_heatmap(&self, field_point: Point2<Field>) -> (usize, usize) {
        let heatmap_point = (
            ((field_point.x() + self.field_dimensions.length / 2.0) * self.cells_per_meter)
                as usize,
            ((field_point.y() + self.field_dimensions.width / 2.0) * self.cells_per_meter) as usize,
        );
        (
            clamp(heatmap_point.0, 0, self.map.dim().0 - 1),
            clamp(heatmap_point.1, 0, self.map.dim().1 - 1),
        )
    }

    fn get_maximum_position(&self, minimum_validity: f32) -> Option<(usize, usize)> {
        let linear_maximum_heat_heatmap_position =
            self.map.iter().position_max_by(|a, b| a.total_cmp(b))?;
        let maximum_heat_heatmap_position = (
            linear_maximum_heat_heatmap_position / self.map.dim().1,
            linear_maximum_heat_heatmap_position % self.map.dim().1,
        );
        if self.map[maximum_heat_heatmap_position] > minimum_validity {
            return Some(maximum_heat_heatmap_position);
        }
        None
    }

    fn add_teamballs(&mut self, time: SystemTime, message: HulkMessage, team_ball_weight: f32) {
        let ball = match message {
            HulkMessage::State(StateMessage { ball_position, .. }) => {
                ball_position.map(|ball| BallPosition {
                    position: ball.position,
                    velocity: Vector2::zeros(),
                    last_seen: time - ball.age,
                })
            }
        };
        if let Some(ball_position) = ball {
            self[ball_position.position] = team_ball_weight;
        }
    }

    fn decay_tiles_in_fov(
        &mut self,
        robot_position: Vector2<Field>,
        left_edge: Vector2<Field>,
        right_edge: Vector2<Field>,
        decay_distance_factor: f32,
        heatmap_decay_range: Range<f32>,
    ) {
        self.map.indexed_iter_mut().for_each(|((x, y), value)| {
            let tile_center_in_field: Vector2<Field> = vector![
                ((x as f32 + 1.0 / 2.0) / self.cells_per_meter
                    - self.field_dimensions.length / 2.0),
                ((y as f32 + 1.0 / 2.0) / self.cells_per_meter - self.field_dimensions.width / 2.0)
            ];
            let robot_to_tile = tile_center_in_field - robot_position;
            let is_inside_sight = get_direction(left_edge, robot_to_tile)
                == Direction::Counterclockwise
                && get_direction(right_edge, robot_to_tile) == Direction::Clockwise;
            let distance_to_tile = robot_to_tile.norm();
            let relative_distance_to_tile =
                clamp(distance_to_tile / heatmap_decay_range.end, 0.0, 1.0);
            if is_inside_sight && heatmap_decay_range.contains(&distance_to_tile) {
                *value *= 1.0 - decay_distance_factor * (1.0 - relative_distance_to_tile);
            }
        });
    }
}

impl Index<Point2<Field>> for Heatmap {
    type Output = f32;
    fn index(&self, field_point: Point2<Field>) -> &Self::Output {
        let heatmap_point = self.field_to_heatmap(field_point);
        &self.map[heatmap_point]
    }
}

impl IndexMut<Point2<Field>> for Heatmap {
    fn index_mut(&mut self, field_point: Point2<Field>) -> &mut Self::Output {
        let heatmap_point = self.field_to_heatmap(field_point);
        &mut self.map[heatmap_point]
    }
}

fn get_rule_hypotheses(
    primary_state: PrimaryState,
    filtered_game_controller_state: &FilteredGameControllerState,
    field_dimensions: FieldDimensions,
) -> Vec<Point2<Field>> {
    let kicking_team_half = kicking_team_half(filtered_game_controller_state.kicking_team);

    match (primary_state, filtered_game_controller_state.sub_state) {
        (PrimaryState::Ready, Some(SubState::PenaltyKick)) => {
            let kicking_team_half = kicking_team_half.unwrap_or(Half::Own).mirror();
            vec![field_dimensions.penalty_spot(kicking_team_half)]
        }
        // Kick-off
        (PrimaryState::Ready, None) => vec![field_dimensions.center()],
        (PrimaryState::Playing, Some(SubState::CornerKick)) => {
            if let Some(kicking_team_half) = kicking_team_half {
                let kicking_team_half = kicking_team_half.mirror();
                vec![
                    field_dimensions.corner(kicking_team_half, Side::Left),
                    field_dimensions.corner(kicking_team_half, Side::Right),
                ]
            } else {
                vec![
                    field_dimensions.corner(Half::Own, Side::Left),
                    field_dimensions.corner(Half::Opponent, Side::Left),
                    field_dimensions.corner(Half::Own, Side::Right),
                    field_dimensions.corner(Half::Opponent, Side::Right),
                ]
            }
        }
        (PrimaryState::Playing, Some(SubState::GoalKick)) => {
            if let Some(kicking_team_half) = kicking_team_half {
                vec![
                    field_dimensions.goal_box_corner(kicking_team_half, Side::Left),
                    field_dimensions.goal_box_corner(kicking_team_half, Side::Right),
                ]
            } else {
                vec![
                    field_dimensions.goal_box_corner(Half::Own, Side::Left),
                    field_dimensions.goal_box_corner(Half::Opponent, Side::Left),
                    field_dimensions.goal_box_corner(Half::Own, Side::Right),
                    field_dimensions.goal_box_corner(Half::Opponent, Side::Right),
                ]
            }
        }
        (_, _) => Vec::new(),
    }
}

fn kicking_team_half(kicking_team: Option<Team>) -> Option<Half> {
    match kicking_team {
        Some(Team::Opponent) => Some(Half::Opponent),
        Some(Team::Hulks) => Some(Half::Own),
        None => None,
    }
}

fn get_direction(base_vector: Vector2<Field>, vector_to_test: Vector2<Field>) -> Direction {
    let clockwise_normal_vector = base_vector.rotate_90_degrees(Direction::Clockwise);
    let directed_cathetus = clockwise_normal_vector.dot(&vector_to_test);

    match directed_cathetus {
        0.0 => Direction::Collinear,
        f if f > 0.0 => Direction::Clockwise,
        f if f < 0.0 => Direction::Counterclockwise,
        f => panic!("directed cathetus was not a real number: {f}"),
    }
}
