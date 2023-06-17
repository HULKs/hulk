use nalgebra::Point2;
use projection::Projection;
use types::{CameraMatrices, CameraMatrix, Line, Line2};

#[derive(Clone)]
pub struct Lines {
    pub top: LinesPerCamera,
    pub bottom: LinesPerCamera,
}

impl Lines {
    pub fn to_projected(&self, matrices: &CameraMatrices) -> Result<Self, LinesError> {
        Ok(Self {
            top: project_lines_and_map_error(&matrices.top, &self.top, "top")?,
            bottom: project_lines_and_map_error(&matrices.bottom, &self.bottom, "bottom")?,
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

fn project_lines_and_map_error(
    matrix: &CameraMatrix,
    lines: &LinesPerCamera,
    which: &str,
) -> Result<LinesPerCamera, LinesError> {
    Ok(LinesPerCamera {
        border_line: project_line_and_map_error(
            matrix,
            lines.border_line,
            &format!("{which} border line"),
        )?,
        goal_box_line: project_line_and_map_error(
            matrix,
            lines.goal_box_line,
            &format!("{which} goal box line"),
        )?,
        connecting_line: project_line_and_map_error(
            matrix,
            lines.connecting_line,
            &format!("{which} connecting line"),
        )?,
    })
}

fn project_line_and_map_error(
    matrix: &CameraMatrix,
    line: Line2,
    which: &str,
) -> Result<Line2, LinesError> {
    Ok(Line(
        project_point_and_map_error(matrix, line.0, &format!("{which} point 0"))?,
        project_point_and_map_error(matrix, line.1, &format!("{which} point 1"))?,
    ))
}

fn project_point_and_map_error(
    matrix: &CameraMatrix,
    point: Point2<f32>,
    which: &str,
) -> Result<Point2<f32>, LinesError> {
    matrix
        .pixel_to_ground(point)
        .map_err(|source| LinesError::NotProjected {
            source,
            which: which.to_string(),
        })
}

#[derive(Clone)]
pub struct LinesPerCamera {
    pub border_line: Line2,
    pub goal_box_line: Line2,
    pub connecting_line: Line2,
}
