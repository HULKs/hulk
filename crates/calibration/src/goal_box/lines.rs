use serde::{Deserialize, Serialize};

use coordinate_systems::{Ground, Pixel};
use geometry::line_segment::LineSegment;
use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use projection::{camera_matrix::CameraMatrix, Projection};

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathIntrospect, PathDeserialize,
)]
pub struct Lines<Frame> {
    pub border_line: LineSegment<Frame>,
    pub goal_box_line: LineSegment<Frame>,
    pub connecting_line: LineSegment<Frame>,
}

impl Lines<Pixel> {
    pub fn project_to_ground(&self, matrix: &CameraMatrix) -> Result<Lines<Ground>, LinesError> {
        Ok(Lines {
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
    line: LineSegment<Pixel>,
    which: &str,
) -> Result<LineSegment<Ground>, LinesError> {
    Ok(LineSegment(
        project_point_and_map_error(matrix, line.0, format!("{which} point 0"))?,
        project_point_and_map_error(matrix, line.1, format!("{which} point 1"))?,
    ))
}

fn project_point_and_map_error(
    matrix: &CameraMatrix,
    point: Point2<Pixel>,
    which: String,
) -> Result<Point2<Ground>, LinesError> {
    matrix
        .pixel_to_ground(point)
        .map_err(|source| LinesError::NotProjected { source, which })
}
