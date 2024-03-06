use nalgebra::Point2;
use projection::Projection;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{
    camera_matrix::CameraMatrix,
    line::{Line, Line2},
};

#[derive(Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct GoalBoxCalibrationLines {
    #[serialize_hierarchy(leaf)]
    pub border_line: Line2,
    #[serialize_hierarchy(leaf)]
    pub goal_box_line: Line2,
    #[serialize_hierarchy(leaf)]
    pub connecting_line: Line2,
}

impl GoalBoxCalibrationLines {
    pub fn to_projected(&self, matrix: &CameraMatrix) -> Result<Self, LinesError> {
        Ok(GoalBoxCalibrationLines {
            border_line: project_line_and_map_error(matrix, self.border_line, "border line")?,
            goal_box_line: project_line_and_map_error(matrix, self.goal_box_line, "goal box line")?,
            connecting_line: project_line_and_map_error(
                matrix,
                self.connecting_line,
                "connecting line",
            )?,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LinesError {
    #[error("failed to project {which} to ground")]
    NotProjected {
        source: projection::Error,
        which: String,
    },
}

fn project_line_and_map_error(
    matrix: &CameraMatrix,
    line: Line2,
    which: &str,
) -> Result<Line2, LinesError> {
    Ok(Line(
        project_point_and_map_error(matrix, line.0, format!("{which} point 0"))?,
        project_point_and_map_error(matrix, line.1, format!("{which} point 1"))?,
    ))
}

fn project_point_and_map_error(
    matrix: &CameraMatrix,
    point: Point2<f32>,
    which: String,
) -> Result<Point2<f32>, LinesError> {
    matrix
        .pixel_to_ground(point)
        .map_err(|source| LinesError::NotProjected { source, which })
}
