//! Typed defaults for variant selectors. The form edits these messages without changing their schema.
use std::time::Duration;

use coordinate_systems::Ground;
use geometry::{arc::Arc, circle::Circle, direction::Direction, line_segment::LineSegment};
use hsl_network_messages::{GamePhase, Penalty, SubState, Team};
use linear_algebra::{Orientation2, point, vector};
use serde::Serialize;
use serde_json::Value;
use types::{
    field_dimensions::GlobalFieldSide,
    filtered_game_state::FilteredGameState,
    motion_command::{HeadMotion, ImageRegion, MotionCommand, OrientationMode},
    path::{PathSegment, direct_path},
};

pub fn value(value: impl Serialize) -> Value {
    serde_json::to_value(value).expect("finite editor defaults serialize")
}

pub fn motion_choices() -> Vec<Value> {
    let head = HeadMotion::ZeroAngles;
    [
        MotionCommand::Damping,
        MotionCommand::Prepare,
        MotionCommand::Stand { head },
        MotionCommand::StandUp { fast: false },
        MotionCommand::WalkWithVelocity {
            head,
            velocity: vector![0.0, 0.0],
            angular_velocity: 0.0,
        },
        MotionCommand::Walk {
            head,
            path: direct_path(point![0.0, 0.0], point![1.0, 0.0]),
            orientation_mode: OrientationMode::AlignWithPath,
            target_orientation: Orientation2::identity(),
            distance_to_be_aligned: 0.5,
            speed: 0.3,
        },
        MotionCommand::Kick {
            head,
            ball_position: point![0.2, 0.0],
            kick_direction: Orientation2::identity(),
            target_speed: 3.4,
            ball_velocity: vector![0.0, 0.0],
            soft: false,
            quick: false,
            strong: false,
        },
    ]
    .into_iter()
    .map(value)
    .collect()
}

pub fn segment_choices() -> Vec<Value> {
    [
        PathSegment::LineSegment(LineSegment(point![0.0, 0.0], point![1.0, 0.0])),
        PathSegment::Arc(Arc {
            circle: Circle {
                center: point![0.0, 0.0],
                radius: 1.0,
            },
            start: Orientation2::new(0.0),
            end: Orientation2::new(std::f32::consts::FRAC_PI_2),
            direction: Direction::Counterclockwise,
        }),
    ]
    .into_iter()
    .map(value)
    .collect()
}

pub fn choices(path: &str) -> Option<Vec<Value>> {
    let name = path.rsplit('/').next().unwrap_or(path);
    Some(match name {
        "motion" => motion_choices(),
        "head" => [
            HeadMotion::ZeroAngles,
            HeadMotion::Center {
                image_region_target: ImageRegion::Center,
            },
            HeadMotion::LookAround,
            HeadMotion::SearchForLostBall,
            HeadMotion::LookAt {
                target: point![1.0, 0.0],
                height_above_ground: 0.0,
                image_region_target: ImageRegion::Center,
            },
            HeadMotion::LookLeftAndRightOf {
                target: point![1.0, 0.0],
                height_above_ground: 0.0,
            },
            HeadMotion::Damping,
        ]
        .into_iter()
        .map(value)
        .collect(),
        "image_region_target" => [ImageRegion::Bottom, ImageRegion::Center, ImageRegion::Top]
            .into_iter()
            .map(value)
            .collect(),
        "orientation_mode" => [
            OrientationMode::Unspecified,
            OrientationMode::AlignWithPath,
            OrientationMode::LookTowards {
                direction: Orientation2::identity(),
                tolerance: 0.1,
            },
            OrientationMode::LookAt {
                target: point![1.0, 0.0],
                tolerance: 0.1,
            },
        ]
        .into_iter()
        .map(value)
        .collect(),
        "direction" if path.contains("/Arc/") => {
            [Direction::Clockwise, Direction::Counterclockwise]
                .into_iter()
                .map(value)
                .collect()
        }
        "game_state" | "opponent_game_state" => [
            FilteredGameState::Initial,
            FilteredGameState::Ready,
            FilteredGameState::Set,
            FilteredGameState::Playing {
                ball_is_free: true,
                kick_off: false,
            },
            FilteredGameState::Finished,
            FilteredGameState::Stop,
        ]
        .into_iter()
        .map(value)
        .collect(),
        "game_phase" => [
            GamePhase::Normal,
            GamePhase::PenaltyShootout {
                kicking_team: Team::Hulks,
            },
            GamePhase::Extratime,
            GamePhase::Timeout,
        ]
        .into_iter()
        .map(value)
        .collect(),
        "kicking_team" if path.contains("PenaltyShootout") => [Team::Hulks, Team::Opponent]
            .into_iter()
            .map(value)
            .collect(),
        "kicking_team" => [None, Some(Team::Hulks), Some(Team::Opponent)]
            .into_iter()
            .map(value)
            .collect(),
        "sub_state" => [
            None,
            Some(SubState::DirectFreeKick),
            Some(SubState::IndirectFreeKick),
            Some(SubState::PenaltyKick),
            Some(SubState::ThrowIn),
            Some(SubState::GoalKick),
            Some(SubState::CornerKick),
        ]
        .into_iter()
        .map(value)
        .collect(),
        "global_field_side" => [GlobalFieldSide::Home, GlobalFieldSide::Away]
            .into_iter()
            .map(value)
            .collect(),
        _ if path
            .rsplit_once('/')
            .is_some_and(|(parent, _)| parent.ends_with("/segments")) =>
        {
            segment_choices()
        }
        _ if path.rsplit_once('/').is_some_and(|(parent, _)| {
            parent.ends_with("/penalties") || parent.ends_with("_penalties_last_cycle")
        }) =>
        {
            penalty_choices()
        }
        _ => return None,
    })
}

pub fn penalty_choices() -> Vec<Value> {
    let remaining = Duration::from_secs(30);
    std::iter::once(Value::Null)
        .chain(
            [
                Penalty::IllegalPosition { remaining },
                Penalty::MotionInSet { remaining },
                Penalty::MotionInStop { remaining },
                Penalty::LocalGameStuck { remaining },
                Penalty::IncapableRobot { remaining },
                Penalty::PickUp { remaining },
                Penalty::BallHolding { remaining },
                Penalty::LeavingTheField { remaining },
                Penalty::PlayingWithArmsHands { remaining },
                Penalty::Pushing { remaining },
                Penalty::Cautioned { remaining },
                Penalty::SentOff { remaining },
                Penalty::Substitute { remaining },
            ]
            .into_iter()
            .map(value),
        )
        .collect()
}

pub fn is_angle(path: &str, current: &Value) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    matches!(
        name,
        "kick_direction" | "target_orientation" | "start" | "end" | "direction"
    ) && serde_json::from_value::<Orientation2<Ground>>(current.clone()).is_ok()
}

pub fn variant(value: &Value) -> &str {
    match value {
        Value::String(name) => name,
        Value::Object(fields) if fields.len() == 1 => fields.keys().next().unwrap(),
        Value::Null => "None",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_motion_and_nested_head_variant_round_trips() {
        for mut command in motion_choices() {
            let motion: MotionCommand = serde_json::from_value(command.clone()).unwrap();
            assert_eq!(value(&motion), command);
            if let Some(fields) = command
                .as_object_mut()
                .and_then(|map| map.values_mut().next())
                .and_then(Value::as_object_mut)
                && fields.contains_key("head")
            {
                for head in choices("/motion/Stand/head").unwrap() {
                    fields.insert("head".into(), head.clone());
                    serde_json::from_value::<HeadMotion>(head).unwrap();
                }
            }
            serde_json::from_value::<MotionCommand>(command).unwrap();
        }
    }

    #[test]
    fn all_orientation_modes_and_path_segments_are_editable() {
        for mode in choices("/motion/Walk/orientation_mode").unwrap() {
            serde_json::from_value::<OrientationMode>(mode).unwrap();
        }
        for segment in segment_choices() {
            serde_json::from_value::<PathSegment>(segment).unwrap();
        }
        let angle = value(Orientation2::<Ground>::new(0.75));
        assert!(is_angle("/motion/Kick/kick_direction", &angle));
        assert!(!is_angle(
            "/motion/Kick/ball_position",
            &value(point![<Ground>, 0.2, 0.1])
        ));
    }

    #[test]
    fn every_game_controller_choice_matches_its_message_field() {
        use types::filtered_game_controller_state::FilteredGameControllerState;
        let mut state = value(FilteredGameControllerState::default());
        for name in [
            "game_state",
            "opponent_game_state",
            "game_phase",
            "kicking_team",
            "sub_state",
            "global_field_side",
        ] {
            for option in choices(&format!("/game/{name}")).unwrap() {
                state[name] = option;
                serde_json::from_value::<FilteredGameControllerState>(state.clone()).unwrap();
            }
        }
        for penalty in penalty_choices() {
            state["penalties"]["one"] = penalty;
            serde_json::from_value::<FilteredGameControllerState>(state.clone()).unwrap();
        }
    }
}
