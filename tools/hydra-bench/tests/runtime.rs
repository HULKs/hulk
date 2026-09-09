use color_eyre::Result;
use ndarray::Array4;
use ort::session::Session;

#[test]
#[ignore = "requires ONNX Runtime; see tools/k1-setup/inference-runtime/README.md"]
fn inference_preserves_values_and_layout() -> Result<()> {
    // Float32 Identity graph with input `images` and output `out`, shape [1, 3, 2, 2].
    let mut session = Session::builder()?.commit_from_memory(IDENTITY_MODEL)?;
    assert_eq!(session.inputs()[0].name(), "images");
    let image = Array4::from_shape_fn((1, 3, 2, 2), |(_, channel, y, x)| {
        (channel * 4 + y * 2 + x) as f32
    });

    for image in [image.clone(), image.permuted_axes([0, 1, 3, 2])] {
        let outputs = hydra_bench::run_inference(&mut session, &image)?;
        let (shape, data) = outputs["out"].try_extract_tensor::<f32>()?;
        assert_eq!(&**shape, &[1, 3, 2, 2]);
        assert_eq!(data, image.iter().copied().collect::<Vec<_>>());
    }
    Ok(())
}

// ONNX IR 8, opset 13. Kept inline so the tiny fixture does not require Git LFS.
const IDENTITY_MODEL: &[u8] = &[
    8, 8, 58, 100, 10, 23, 10, 6, 105, 109, 97, 103, 101, 115, 18, 3, 111, 117, 116, 34, 8, 73,
    100, 101, 110, 116, 105, 116, 121, 18, 8, 105, 100, 101, 110, 116, 105, 116, 121, 90, 32, 10,
    6, 105, 109, 97, 103, 101, 115, 18, 22, 10, 20, 8, 1, 18, 16, 10, 2, 8, 1, 10, 2, 8, 3, 10, 2,
    8, 2, 10, 2, 8, 2, 98, 29, 10, 3, 111, 117, 116, 18, 22, 10, 20, 8, 1, 18, 16, 10, 2, 8, 1, 10,
    2, 8, 3, 10, 2, 8, 2, 10, 2, 8, 2, 66, 2, 16, 13,
];
