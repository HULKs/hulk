use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::egui::{Align2, Color32, ColorImage, FontId, Stroke, TextureOptions};
use geometry::rectangle::Rectangle;
use linear_algebra::point;
use types::{
    object_detection::YOLOObjectLabel,
    segmentation_detection::{PROTOTYPE_MASK_HEIGHT, PROTOTYPE_MASK_WIDTH, SegmentedObject},
};

use crate::{panels::image::overlay::Overlay, robot::Robot, value_buffer::BufferHandle};

const IMAGE_WIDTH: f32 = 544.0;
const IMAGE_HEIGHT: f32 = 448.0;

pub struct Segmentation {
    segmentations: BufferHandle<Vec<SegmentedObject<YOLOObjectLabel>>>,
}

impl Overlay for Segmentation {
    const NAME: &'static str = "Segmentation";

    fn new(robot: Arc<Robot>) -> Self {
        let segmentations = robot.subscribe_value("Hydra.main_outputs.detected_segments");
        Self { segmentations }
    }

    fn paint(&self, painter: &crate::twix_painter::TwixPainter<Pixel>) -> Result<()> {
        let Some(segmentations) = self.segmentations.get_last_value()? else {
            return Ok(());
        };

        for (index, segmented_object) in segmentations.into_iter().enumerate() {
            paint_segmented_object(painter, segmented_object, index);
        }

        Ok(())
    }

    fn config_ui(&mut self, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(10.0);
        });
    }
}

fn label_color(label: &YOLOObjectLabel) -> Color32 {
    let index = *label as u8;
    let hue = (index as f32 * 137.508) % 360.0;
    hsv_to_color32(hue, 0.8, 0.9)
}

fn hsv_to_color32(h: f32, s: f32, v: f32) -> Color32 {
    let sector = h / 60.0;
    let i = sector.floor() as u32;
    let f = sector - sector.floor();
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

fn paint_segmented_object(
    painter: &crate::twix_painter::TwixPainter<Pixel>,
    segmented_object: SegmentedObject<YOLOObjectLabel>,
    index: usize,
) {
    let color = label_color(&segmented_object.object.label);
    let bbox = segmented_object.object.bounding_box;

    painter.rect_stroke(bbox.area.min, bbox.area.max, Stroke::new(1.0, color));
    painter.floating_text(
        bbox.area.min,
        Align2::RIGHT_BOTTOM,
        format!("{:.2}", bbox.confidence),
        FontId::default(),
        Color32::WHITE,
    );
    painter.floating_text(
        bbox.area.max,
        Align2::RIGHT_TOP,
        segmented_object.object.label.into(),
        FontId::default(),
        Color32::WHITE,
    );

    let mask_color =
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 100);
    let rgba_bytes: Vec<u8> = segmented_object
        .mask
        .iter()
        .flat_map(|&v| {
            if v > 0.5 {
                [mask_color.r(), mask_color.g(), mask_color.b(), mask_color.a()]
            } else {
                [0, 0, 0, 0]
            }
        })
        .collect();

    let mask_image = ColorImage::from_rgba_unmultiplied(
        [PROTOTYPE_MASK_WIDTH, PROTOTYPE_MASK_HEIGHT],
        &rgba_bytes,
    );

    let texture_id = painter
        .ctx()
        .load_texture(
            format!("seg-mask-{index}"),
            mask_image,
            TextureOptions::NEAREST,
        )
        .id();

    painter.image(
        texture_id,
        Rectangle {
            min: point![0.0_f32, 0.0_f32],
            max: point![IMAGE_WIDTH, IMAGE_HEIGHT],
        },
    );
}
