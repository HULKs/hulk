use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Field;
use eframe::egui::{Color32, Stroke};
use hsl_network_messages::PlayerNumber;
use ros_z_debug::{SampleRecord, TopicObservation};
use voronoi::Ownership;
use world_state::behavior::node::Blackboard;

use crate::{backend::RobotBackend, panels::map::layer::Layer, twix_painter::TwixPainter};

pub struct VoronoiCell {
    blackboard: TopicObservation<Blackboard>,
}

impl Layer<Field> for VoronoiCell {
    const NAME: &'static str = "Voronoi Cells";

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();

        let blackboard = backend
            .observer()
            .observe_typed("behavior/blackboard")
            .expect("failed to construct blackboard observer")
            .spawn();

        Self { blackboard }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &types::field_dimensions::FieldDimensions,
    ) -> Result<()> {
        let latest_blackboard_sample = self.blackboard.latest();

        let Some(SampleRecord {
            value: blackboard, ..
        }) = latest_blackboard_sample.as_deref()
        else {
            return Ok(());
        };

        let Some(grid) = blackboard.voronoi_map.as_ref() else {
            return Ok(());
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

        for voronoi_input in &blackboard.voronoi_inputs {
            painter.pose(
                *voronoi_input,
                0.08,
                0.12,
                Color32::from_rgba_premultiplied(255, 0, 0, 128),
                Stroke::new(0.01, Color32::BLACK),
            );
        }

        Ok(())
    }
}
