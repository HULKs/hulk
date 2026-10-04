use std::sync::Arc;

use behavior_node::node::Blackboard;
use color_eyre::Result;
use coordinate_systems::Field;
use ros_z_debug::{SampleRecord, TopicObservation};

use crate::{backend::RobotBackend, panels::map::layer::Layer};
use twix_visualization::twix_painter::TwixPainter;

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

        painter.voronoi_grid(grid, &blackboard.voronoi_inputs);

        Ok(())
    }
}
