use ndarray::{aview1, Array2, ArrayView3};

use crate::object_detection::{LabelIndex, NUMBER_OF_VALUES_PER_OBJECT, Object};

pub const NUMBER_OF_MASK_COEFFICIENTS: usize = 32;
pub const NUMBER_OF_VALUES_PER_SEGMENTED_OBJECT: usize =
    NUMBER_OF_VALUES_PER_OBJECT + NUMBER_OF_MASK_COEFFICIENTS;
pub const PROTOTYPE_MASK_CHANNELS: usize = 32;
pub const PROTOTYPE_MASK_HEIGHT: usize = 160;
pub const PROTOTYPE_MASK_WIDTH: usize = 160;

#[derive(Clone, Debug)]
pub struct SegmentedObject<T> {
    pub object: Object<T>,
    pub mask: Array2<f32>,
}

impl<T> From<(&[f32; NUMBER_OF_VALUES_PER_SEGMENTED_OBJECT], ArrayView3<'_, f32>)>
    for SegmentedObject<T>
where
    T: LabelIndex,
{
    fn from(
        (values, proto): (&[f32; NUMBER_OF_VALUES_PER_SEGMENTED_OBJECT], ArrayView3<'_, f32>),
    ) -> Self {
        let object_values: [f32; NUMBER_OF_VALUES_PER_OBJECT] =
            values[..NUMBER_OF_VALUES_PER_OBJECT]
                .try_into()
                .expect("slice length guaranteed by type");
        let object = Object::from(object_values);

        let mask_coefficients = aview1(&values[NUMBER_OF_VALUES_PER_OBJECT..]);

        let proto_2d = proto
            .to_owned()
            .into_shape((
                PROTOTYPE_MASK_CHANNELS,
                PROTOTYPE_MASK_HEIGHT * PROTOTYPE_MASK_WIDTH,
            ))
            .expect("proto shape is always [32, 160, 160]");

        let mask_flat = mask_coefficients.dot(&proto_2d);
        let mask_2d = mask_flat
            .into_shape((PROTOTYPE_MASK_HEIGHT, PROTOTYPE_MASK_WIDTH))
            .expect("product shape is always 25600");

        let mask = mask_2d.mapv(|x| 1.0_f32 / (1.0 + (-x).exp()));

        Self { object, mask }
    }
}

#[cfg(test)]
mod tests {
    use ndarray::Array3;

    use super::*;

    #[test]
    fn test_zero_coefficients_gives_half_mask() {
        let mut values = [0.0f32; NUMBER_OF_VALUES_PER_SEGMENTED_OBJECT];
        values[2] = 1.0;
        values[3] = 1.0;
        values[4] = 0.9;

        let proto = Array3::<f32>::ones((
            PROTOTYPE_MASK_CHANNELS,
            PROTOTYPE_MASK_HEIGHT,
            PROTOTYPE_MASK_WIDTH,
        ));

        let segmented: SegmentedObject<YOLOObjectLabel> =
            SegmentedObject::from((&values, proto.view()));

        assert_eq!(
            segmented.mask.shape(),
            &[PROTOTYPE_MASK_HEIGHT, PROTOTYPE_MASK_WIDTH]
        );
        for &v in segmented.mask.iter() {
            assert!((v - 0.5).abs() < 1e-5, "expected 0.5, got {v}");
        }
    }

    #[test]
    fn test_positive_coefficients_give_high_mask() {
        let mut values = [0.0f32; NUMBER_OF_VALUES_PER_SEGMENTED_OBJECT];
        values[2] = 1.0;
        values[3] = 1.0;
        values[4] = 0.9;
        values[6] = 1.0;

        let mut proto = Array3::<f32>::zeros((
            PROTOTYPE_MASK_CHANNELS,
            PROTOTYPE_MASK_HEIGHT,
            PROTOTYPE_MASK_WIDTH,
        ));
        proto.slice_mut(ndarray::s![0, .., ..]).fill(10.0);

        let segmented: SegmentedObject<YOLOObjectLabel> =
            SegmentedObject::from((&values, proto.view()));

        for &v in segmented.mask.iter() {
            assert!(v > 0.999, "expected ~1.0, got {v}");
        }
    }
}
