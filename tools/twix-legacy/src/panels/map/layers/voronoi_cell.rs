use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Field;
use eframe::egui::{Color32, Stroke};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Point2, Pose2};
use serde_json::{Value, from_value};
use voronoi::{Ownership, VoronoiGrid};

use crate::{
    panels::map::layer::Layer, robot::Robot, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct VoronoiCell {
    voronoi_grid: BufferHandle<Value>,
    voronoi_inputs: BufferHandle<Value>,
    target_voronoi_position: BufferHandle<Value>,
}

impl Layer<Field> for VoronoiCell {
    const NAME: &'static str = "Voronoi Cells";

    fn new(robot: Arc<Robot>) -> Self {
        let voronoi_grid =
            robot.subscribe_json("WorldState.additional_outputs.behavior.voronoi_map");
        let voronoi_inputs =
            robot.subscribe_json("WorldState.additional_outputs.behavior.voronoi_inputs");
        let target_voronoi_position =
            robot.subscribe_json("WorldState.additional_outputs.behavior.target_voronoi_position");
        Self {
            voronoi_grid,
            voronoi_inputs,
            target_voronoi_position,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &types::field_dimensions::FieldDimensions,
    ) -> Result<()> {
        let Some(grid_value) = self.voronoi_grid.get_last_value()? else {
            return Ok(());
        };
        let grid: VoronoiGrid = match from_value(grid_value) {
            Ok(grid) => grid,
            Err(_) => return Ok(()),
        };

        let colors = [
            Color32::from_rgb(0, 114, 178),   // Dark Blue
            Color32::from_rgb(230, 159, 0),   // Orange
            Color32::from_rgb(204, 121, 167), // Reddish Purple
            Color32::from_rgb(86, 180, 233),  // Sky Blue
            Color32::from_rgb(213, 94, 0),    // Vermillion
            Color32::from_rgb(240, 228, 66),  // Yellow
            Color32::from_rgb(0, 0, 0),       // Black
            Color32::from_rgb(255, 255, 255), // White
            Color32::from_rgb(148, 103, 189), // Lavender
            Color32::from_rgb(227, 119, 194), // Pink
            Color32::from_rgb(127, 127, 127), // Grey
            Color32::from_rgb(188, 189, 34),  // Olive
        ];

        for (index, ownership) in grid.tiles.iter().copied().enumerate() {
            let point = grid.index_to_point(index);
            let color = match ownership {
                Ownership::Blocked => Color32::from_gray(40),
                Ownership::Robot(player_number) => {
                    let color_index = match player_number {
                        PlayerNumber::One => 0,
                        PlayerNumber::Two => 1,
                        PlayerNumber::Three => 2,
                        PlayerNumber::Four => 3,
                        PlayerNumber::Five => 4,
                    };
                    colors[color_index % colors.len()]
                }
                Ownership::Free => Color32::from_gray(120),
            };

            painter.circle_filled(point, 0.035, color);
            painter.circle_stroke(point, 0.035, Stroke::new(0.01, Color32::BLACK));
        }

        for player_number in [
            PlayerNumber::One,
            PlayerNumber::Two,
            PlayerNumber::Three,
            PlayerNumber::Four,
            PlayerNumber::Five,
        ] {
            if let Some(centroid) = grid.centroid_for_player(player_number) {
                painter.target(
                    centroid,
                    0.06,
                    Stroke::new(0.01, Color32::GREEN),
                    Color32::RED,
                );
            }
        }

        if let Some(voronoi_inputs) = self.voronoi_inputs.get_last_value()? {
            let voronoi_inputs: Vec<Pose2<Field>> = match from_value(voronoi_inputs) {
                Ok(inputs) => inputs,
                Err(_) => return Ok(()),
            };
            for voronoi_input in &voronoi_inputs {
                painter.pose(
                    *voronoi_input,
                    0.08,
                    0.12,
                    Color32::from_rgba_premultiplied(255, 0, 0, 128),
                    Stroke::new(0.01, Color32::BLACK),
                );
            }
        }

        if let Some(target_voronoi_position) = self.target_voronoi_position.get_last_value()? {
            let target_voronoi_position: Option<Point2<Field>> =
                match from_value(target_voronoi_position) {
                    Ok(position) => position,
                    Err(_) => return Ok(()),
                };
            if let Some(position) = target_voronoi_position {
                painter.target(
                    position,
                    0.1,
                    Stroke::new(0.02, Color32::YELLOW),
                    Color32::TRANSPARENT,
                );
            }
        }

        Ok(())
    }
}
