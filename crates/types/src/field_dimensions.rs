use serde::{Deserialize, Serialize};

use linear_algebra::{point, Point2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use coordinate_systems::Field;

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct FieldDimensions {
    pub ball_radius: f32,
    pub length: f32,
    pub width: f32,
    pub line_width: f32,
    pub penalty_marker_size: f32,
    pub goal_box_area_length: f32,
    pub goal_box_area_width: f32,
    pub penalty_area_length: f32,
    pub penalty_area_width: f32,
    pub penalty_marker_distance: f32,
    pub center_circle_diameter: f32,
    pub border_strip_width: f32,
    pub goal_inner_height: f32,
    pub goal_inner_width: f32,
    pub goal_post_diameter: f32,
    pub goal_depth: f32,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    PartialEq,
    Eq,
)]
pub enum GlobalFieldSide {
    Home,
    Away,
}

impl GlobalFieldSide {
    pub fn sign(self) -> f32 {
        match self {
            GlobalFieldSide::Home => -1.0,
            GlobalFieldSide::Away => 1.0,
        }
    }

    pub fn mirror(self) -> Self {
        match self {
            GlobalFieldSide::Home => GlobalFieldSide::Away,
            GlobalFieldSide::Away => GlobalFieldSide::Home,
        }
    }
}

impl FieldDimensions {
    pub const SPL_2025: Self = Self {
        ball_radius: 0.05,
        length: 9.0,
        width: 6.0,
        line_width: 0.05,
        penalty_marker_size: 0.1,
        goal_box_area_length: 0.6,
        goal_box_area_width: 2.2,
        penalty_area_length: 1.65,
        penalty_area_width: 4.0,
        penalty_marker_distance: 1.3,
        center_circle_diameter: 1.5,
        border_strip_width: 2.0,
        goal_inner_height: 0.8,
        goal_inner_width: 1.5,
        goal_post_diameter: 0.1,
        goal_depth: 0.5,
    };
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Half {
    Own,
    Opponent,
}

impl Half {
    fn sign(self) -> f32 {
        match self {
            Half::Own => -1.0,
            Half::Opponent => 1.0,
        }
    }

    pub fn mirror(self) -> Self {
        match self {
            Half::Own => Half::Opponent,
            Half::Opponent => Half::Own,
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    PartialEq,
    Eq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    fn sign(self) -> f32 {
        match self {
            Side::Left => 1.0,
            Side::Right => -1.0,
        }
    }

    pub fn opposite(&self) -> Side {
        match self {
            Side::Left => Self::Right,
            Side::Right => Self::Left,
        }
    }
}

impl FieldDimensions {
    pub fn is_inside_field(&self, position: Point2<Field>) -> bool {
        position.x().abs() < self.length / 2.0 && position.y().abs() < self.width / 2.0
    }

    pub fn is_inside_any_goal(&self, position: Point2<Field>) -> bool {
        position.x().abs() > self.length / 2.0 && position.y().abs() < self.goal_inner_width / 2.0
    }

    pub fn penalty_spot(&self, half: Half) -> Point2<Field> {
        let unsigned_x = self.length / 2.0 - self.penalty_marker_distance;
        point![unsigned_x * half.sign(), 0.0]
    }

    pub fn corner(&self, half: Half, side: Side) -> Point2<Field> {
        let unsigned_x = self.length / 2.0;
        let unsigned_y = self.width / 2.0;
        point![unsigned_x * half.sign(), unsigned_y * side.sign()]
    }

    pub fn goal_box_corner(&self, half: Half, side: Side) -> Point2<Field> {
        let unsigned_x = self.length / 2.0 - self.goal_box_area_length;
        let unsigned_y = self.goal_box_area_width / 2.0;
        point![unsigned_x * half.sign(), unsigned_y * side.sign()]
    }

    pub fn t_crossing(&self, side: Side) -> Point2<Field> {
        let unsigned_y = self.width / 2.0;
        point![0.0, unsigned_y * side.sign()]
    }

    pub fn penalty_box_corner(&self, half: Half, side: Side) -> Point2<Field> {
        let unsigned_x = self.length / 2.0 - self.penalty_area_length;
        let unsigned_y = self.penalty_area_width / 2.0;
        point![unsigned_x * half.sign(), unsigned_y * side.sign()]
    }

    pub fn goal_box_goal_line_intersection(&self, half: Half, side: Side) -> Point2<Field> {
        let unsigned_x = self.length / 2.0;
        let unsigned_y = self.goal_box_area_width / 2.0;
        point![unsigned_x * half.sign(), unsigned_y * side.sign()]
    }

    pub fn penalty_box_goal_line_intersection(&self, half: Half, side: Side) -> Point2<Field> {
        let unsigned_x = self.length / 2.0;
        let unsigned_y = self.penalty_area_width / 2.0;
        point![unsigned_x * half.sign(), unsigned_y * side.sign()]
    }

    pub fn center(&self) -> Point2<Field> {
        Point2::origin()
    }
}
