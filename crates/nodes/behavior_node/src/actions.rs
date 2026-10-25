use coordinate_systems::Field;
use geometry::{circle::Circle, line_segment::LineSegment};
use linear_algebra::{Orientation2, Point2, point, vector};
use types::{
    behavior_tree::Status,
    controller_input::{Axis, Button, ControllerInput},
    field_dimensions::FieldDimensions,
    motion_command::{BodyMotion, HeadMotion, OrientationMode},
    path::direct_path,
    rule_obstacles::RuleObstacle,
    walking_velocity_limits::WalkingVelocityLimits,
};

use crate::node::Blackboard;

const ESCAPE_PATH_LENGTH: f32 = 1.0;

pub fn damping(blackboard: &mut Blackboard) -> Status {
    blackboard.body_motion = Some(BodyMotion::Damping);
    Status::Success
}

pub fn injected_motion_command(blackboard: &mut Blackboard) -> Status {
    if blackboard
        .parameters
        .control
        .injected_motion_command
        .is_some()
    {
        blackboard.is_injected_motion_command = true;
        Status::Success
    } else {
        Status::Failure
    }
}

pub fn prepare(blackboard: &mut Blackboard) -> Status {
    blackboard.body_motion = Some(BodyMotion::Prepare);
    Status::Success
}

pub fn remote_control(blackboard: &mut Blackboard) -> Status {
    let Some(input) = &blackboard.controller_input else {
        return Status::Failure;
    };

    blackboard.body_motion = Some(gamepad_walking_command(
        input,
        blackboard.walking_velocity_limits,
    ));
    blackboard.head_motion = Some(HeadMotion::MoveWithVelocity {
        yaw: input.button_value(Button::DPadLeft) - input.button_value(Button::DPadRight),
        pitch: input.button_value(Button::DPadDown) - input.button_value(Button::DPadUp),
    });
    Status::Success
}

fn gamepad_walking_command(input: &ControllerInput, limits: WalkingVelocityLimits) -> BodyMotion {
    BodyMotion::WalkWithVelocity {
        velocity: vector![
            limits.scale_forward_axis(input.axis_value(Axis::LeftStickY)),
            limits.scale_lateral_axis(-input.axis_value(Axis::LeftStickX)),
        ],
        angular_velocity: limits.scale_angular_axis(-input.axis_value(Axis::RightStickX)),
    }
}

pub fn stand(blackboard: &mut Blackboard) -> Status {
    blackboard.body_motion = Some(BodyMotion::Stand);
    Status::Success
}

pub fn stand_up(blackboard: &mut Blackboard) -> Status {
    blackboard.body_motion = Some(BodyMotion::StandUp);
    Status::Success
}

pub fn walk_out_off_rule_obstacles(blackboard: &mut Blackboard) -> Status {
    let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field else {
        return Status::Failure;
    };

    let robot_position = ground_to_field * point!(0.0, 0.0);
    let Some(containing_rule_obstacle) = blackboard
        .world_state
        .rule_obstacles
        .iter()
        .find(|obstacle| obstacle.contains(robot_position))
    else {
        return Status::Failure;
    };

    let parameters = &blackboard.parameters.walking.path_planning;
    let clearance = parameters.robot_radius + 0.05;
    let ball_obstacle = blackboard.world_state.ball.map(|ball| {
        Circle::new(
            ground_to_field * ball.ball_in_ground,
            parameters.ball_obstacle_radius + parameters.robot_radius,
        )
    });
    let Some(escape_point) = rule_obstacle_escape_point(
        containing_rule_obstacle,
        robot_position,
        &blackboard.field_dimensions,
        clearance,
        ball_obstacle,
    ) else {
        return stand(blackboard);
    };

    let escape_direction = (escape_point - robot_position).normalize();
    let target_position = escape_point + escape_direction * ESCAPE_PATH_LENGTH;
    let field_to_ground = ground_to_field.inverse();
    let target_direction = field_to_ground * escape_direction;

    blackboard.body_motion = Some(BodyMotion::Walk {
        // Use a virtual start outside the obstacle so the segment points outward.
        path: direct_path(
            field_to_ground * escape_point,
            field_to_ground * target_position,
        ),
        orientation_mode: OrientationMode::AlignWithPath,
        target_orientation: Orientation2::from_vector(target_direction),
        distance_to_be_aligned: blackboard
            .parameters
            .walking
            .walk_and_stand
            .normal_distance_to_be_aligned,
        speed: blackboard.parameters.walking.speed.search,
    });

    Status::Success
}

fn rule_obstacle_escape_point(
    obstacle: &RuleObstacle,
    robot_position: Point2<Field>,
    field_dimensions: &FieldDimensions,
    clearance: f32,
    ball_obstacle: Option<Circle<Field>>,
) -> Option<Point2<Field>> {
    let candidates = match obstacle {
        RuleObstacle::Rectangle(rectangle) => vec![
            point![rectangle.min.x() - clearance, robot_position.y()],
            point![rectangle.max.x() + clearance, robot_position.y()],
            point![robot_position.x(), rectangle.min.y() - clearance],
            point![robot_position.x(), rectangle.max.y() + clearance],
        ],
        RuleObstacle::Circle(circle) => {
            let direction = (robot_position - circle.center)
                .try_normalize(f32::EPSILON)
                .unwrap_or_else(|| vector![1.0, 0.0]);
            let radius = circle.radius + clearance;
            vec![
                circle.center + direction * radius,
                circle.center + vector![radius, 0.0],
                circle.center + vector![-radius, 0.0],
                circle.center + vector![0.0, radius],
                circle.center + vector![0.0, -radius],
            ]
        }
    };

    candidates
        .into_iter()
        .filter(|point| {
            point.x().abs() < field_dimensions.length / 2.0
                && point.y().abs() < field_dimensions.width / 2.0
        })
        .filter(|escape_point| {
            ball_obstacle.is_none_or(|ball| {
                let direction = (*escape_point - robot_position).normalize();
                let target = *escape_point + direction * ESCAPE_PATH_LENGTH;
                // Include the approach to the virtual start, not just the virtual segment.
                !ball.intersects_line_segment(&LineSegment(robot_position, target))
            })
        })
        .min_by(|left, right| {
            (left - robot_position)
                .norm_squared()
                .total_cmp(&(right - robot_position).norm_squared())
        })
}
