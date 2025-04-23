use std::{
    ops::{Index, IndexMut},
    time::SystemTime,
};

use color_eyre::{eyre::Context, Result};
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput, PerceptionInput};
use itertools::Itertools;
use linear_algebra::{point, Isometry2, Point2, Vector2};
use nalgebra::clamp;
use ndarray::{array, Array2};
use ndarray_conv::{ConvExt, ConvMode, PaddingMode};
use serde::{Deserialize, Serialize};
use spl_network_messages::{HulkMessage, SubState, Team};
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::{FieldDimensions, Half, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    messages::{self, IncomingMessage},
    parameters::SearchSuggestorParameters,
    primary_state::PrimaryState,
};

use crate::team_ball_receiver::get_spl_messages;

#[derive(Deserialize, Serialize)]
pub struct SearchSuggestor {
    heatmap: Heatmap,
}

#[context]
pub struct CreationContext {
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    search_suggestor_configuration: Parameter<SearchSuggestorParameters, "search_suggestor">,
}

#[context]
pub struct CycleContext {
    search_suggestor_configuration: Parameter<SearchSuggestorParameters, "search_suggestor">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    ball_position: Input<Option<BallPosition<Ground>>, "ball_position?">,
    team_ball: Input<Option<BallPosition<Field>>, "team_ball?">,
    hypothetical_ball_positions:
        Input<Vec<HypotheticalBallPosition<Ground>>, "hypothetical_ball_positions">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    primary_state: Input<PrimaryState, "primary_state">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    network_message: PerceptionInput<Option<IncomingMessage>, "SplNetwork", "filtered_message?">,

    heatmap: AdditionalOutput<Array2<f32>, "ball_search_heatmap">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub suggested_search_position: MainOutput<Option<Point2<Field>>>,
}

impl SearchSuggestor {
    pub fn new(context: CreationContext) -> Result<Self> {
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
            field_dimensions: *context.field_dimensions,
            cells_per_meter: context.search_suggestor_configuration.cells_per_meter,
        };
        Ok(Self { heatmap })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        self.update_heatmap(&context)?;
        let suggested_search_position = self
            .heatmap
            .get_maximum_position(context.search_suggestor_configuration.minimum_validity);

        context
            .heatmap
            .fill_if_subscribed(|| self.heatmap.map.clone());

        Ok(MainOutputs {
            suggested_search_position: suggested_search_position.into(),
        })
    }

    fn update_heatmap(&mut self, context: &CycleContext) -> Result<()> {
        if let Some(ball_position) = context.ball_position {
            if let Some(ground_to_field) = context.ground_to_field {
                self.heatmap[ground_to_field * ball_position.position] = 1.0;
            }
        }
        for ball_hypothesis in context.hypothetical_ball_positions {
            if let Some(ground_to_field) = context.ground_to_field {
                let ball_hypothesis_position = ground_to_field * ball_hypothesis.position;
                self.heatmap[ball_hypothesis_position] =
                    (self.heatmap[ball_hypothesis_position] + ball_hypothesis.validity) / 2.0;
            }
        }
        if let Some(filtered_game_controller_state) = context.filtered_game_controller_state {
            for rule_ball_hypothesis in get_rule_hypotheses(
                *context.primary_state,
                filtered_game_controller_state,
                *context.field_dimensions,
            ) {
                self.heatmap[rule_ball_hypothesis] = 1.0;
            }
        }

        let messages = get_spl_messages(&context.network_message.persistent);
        for (time, message) in messages {
            self.heatmap.get_teamballs(time, message);
        }

        let kernel = create_kernel(
            context
                .search_suggestor_configuration
                .heatmap_convolution_kernel_weight,
        );
        self.heatmap.map = self
            .heatmap
            .map
            .conv(&kernel, ConvMode::Same, PaddingMode::Replicate)
            .wrap_err("heatmap convolution failed")?;
        self.heatmap.map /= self.heatmap.map.sum();
        Ok(())
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
    field_dimensions: FieldDimensions,
    cells_per_meter: f32,
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

    fn get_maximum_position(&self, minimum_validity: f32) -> Option<Point2<Field>> {
        let linear_maximum_heat_heatmap_position =
            self.map.iter().position_max_by(|a, b| a.total_cmp(b))?;
        let maximum_heat_heatmap_position = (
            linear_maximum_heat_heatmap_position / self.map.dim().1,
            linear_maximum_heat_heatmap_position % self.map.dim().1,
        );
        if self.map[maximum_heat_heatmap_position] > minimum_validity {
            let search_suggestion = point![
                ((maximum_heat_heatmap_position.0 as f32 + 1.0 / 2.0) / self.cells_per_meter
                    - self.field_dimensions.length / 2.0),
                ((maximum_heat_heatmap_position.1 as f32 + 1.0 / 2.0) / self.cells_per_meter
                    - self.field_dimensions.width / 2.0)
            ];
            return Some(search_suggestion);
        }
        None
    }
    
    fn get_teamballs(&mut self, time: SystemTime, message: HulkMessage) {
        let (_, ball) = match message {
            HulkMessage::Striker(striker_message) => (
                striker_message.player_number,
                Some(BallPosition {
                    position: striker_message.ball_position.position,
                    velocity: Vector2::zeros(),
                    last_seen: time - striker_message.ball_position.age,
                }),
            ),
            HulkMessage::Loser(_) | HulkMessage::VisualReferee(_) => return,
        };
        if let Some(ball_position) = ball {
            self[ball_position.position] = 1.0;
        }
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
