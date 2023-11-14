use std::{path::PathBuf, sync::Arc};

use eframe::epaint::ColorImage;
use egui_plot::PlotPoint;
use ndarray::{s, Array2, Array4, ArrayD, ArrayView1};
use ort::{Environment, Session, SessionBuilder, Value};

use crate::{boundingbox::BoundingBox, classes::Classes};

pub struct Yolo {
    model: Session,
}

impl Yolo {
    pub fn try_from_onnx(path: PathBuf) -> Yolo {
        let environment = Arc::new(Environment::builder().with_name("Yolov8").build().unwrap());
        let model = SessionBuilder::new(&environment)
            .unwrap()
            .with_model_from_file(path)
            .unwrap();

        Self { model }
    }

    pub fn infer(&self, color_image: &ColorImage) -> Vec<BoundingBox> {
        let [width, height] = color_image.size;
        assert_eq!(width, 640);
        assert_eq!(height, 480);

        let image: ArrayD<f32> = Array4::from_shape_fn((1, 3, width, width), |(_, c, y, x)| {
            // let mean = [0.485, 0.456, 0.406][c];
            // let std = [0.229, 0.224, 0.225][c];
            if y >= height {
                // letterbox resize
                return 0.0;
            }

            let pixel = color_image.pixels[width * y + x];
            let color = match c {
                0 => pixel.r() as f32 / 255.0,
                1 => pixel.g() as f32 / 255.0,
                2 => pixel.b() as f32 / 255.0,
                _ => panic!("channel {c} not in image"),
            };

            color
            // (resized[(x as _, y as _)][c] as f32 / 255.0 - mean) / std
        })
        .into_dyn();

        let data = &image.as_standard_layout();
        let input = Value::from_array(self.model.allocator(), data).unwrap();
        let result = self.model.run(vec![input]).unwrap();

        let result = result[0].try_extract::<f32>().unwrap();
        let output: Array2<f32> = result.view().to_shape((8, 8400)).unwrap().to_owned();

        let mut boxes = vec![];

        let x_gain = width as f32   / 640.;
        let y_gain = height as f32 / 640.;

        for detection in output.columns() {
            let labels = detection.slice(s![4..]);
            let (label, score) = softmax_idx(labels);
            if score > 0.3 {
                let min_x = (detection[0] - detection[2] / 2.) * x_gain;
                let min_y = (detection[1] - detection[3] / 2.) * y_gain;
                let max_x = (detection[0] + detection[2] / 2.) * x_gain;
                let max_y = (detection[1] + detection[3] / 2.) * y_gain;

                // let cx = detection[0] as f64;
                // let cy = ((1.0 - detection[1] / 640.0 )* height as f32) as f64;
                // let w = detection[2] as f64;
                // let h = (detection[3] / 640.0 * height as f32) as f64;

                let corner = PlotPoint::new(min_x as f64, 480. - max_y as f64);
                let opposing_corner = PlotPoint::new(max_x as f64, 480. - min_y as f64);

                boxes.push((
                    score,
                    BoundingBox::new(corner, opposing_corner, Classes::from(label)),
                ));
            }
        }

        non_maximum_suppression(boxes, 0.45)
    }
}

pub fn softmax_idx<'a>(array: ArrayView1<'a, f32>) -> (usize, f32) {
    let total: f32 = array.iter().map(|value| value.exp()).sum();
    let argmax = array
        .iter()
        .enumerate()
        .max_by(|(_, value0), (_, value1)| value0.total_cmp(value1))
        .map(|(idx, _)| idx)
        .unwrap();

    return (argmax, array[argmax].exp() / total);
}

pub fn non_maximum_suppression(
    mut detections: Vec<(f32, BoundingBox)>,
    iou_threshold: f32,
) -> Vec<BoundingBox> {
    detections.sort_unstable_by(|(score1, _), (score2, _)| score1.total_cmp(&score2));
    let mut nms_detections = Vec::new();

    while let Some((_, bbox)) = detections.pop() {
        detections = detections
            .into_iter()
            .filter(|(_, cnd_bbox)| bbox.iou(cnd_bbox) < iou_threshold)
            .collect();
        nms_detections.push(bbox);
    }

    nms_detections
}
