//! Tests for the dynamic message module.

use crate::dynamic::{
    ByteRenderPolicy, DynamicJsonRenderPolicy, DynamicValue, NonFiniteFloatRenderPolicy,
    dynamic_value_to_json,
};

#[test]
fn dynamic_json_default_distinguishes_non_finite_floats_from_absent_optionals() {
    let value = dynamic_value_to_json(
        &DynamicValue::Sequence(vec![
            DynamicValue::Float32(f32::NAN),
            DynamicValue::Optional(None),
        ]),
        DynamicJsonRenderPolicy::default(),
    );

    assert_eq!(
        value,
        serde_json::json!([
            { "$type": "non_finite_float", "value": "NaN" },
            null
        ])
    );
}

#[test]
fn dynamic_json_policy_can_preserve_cli_byte_and_float_shape() {
    let value = dynamic_value_to_json(
        &DynamicValue::Sequence(vec![
            DynamicValue::Bytes(vec![1, 2, 3]),
            DynamicValue::Float64(f64::INFINITY),
        ]),
        DynamicJsonRenderPolicy {
            bytes: ByteRenderPolicy::FullArray,
            non_finite_float: NonFiniteFloatRenderPolicy::Null,
        },
    );

    assert_eq!(value, serde_json::json!([[1, 2, 3], null]));
}
