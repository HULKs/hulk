from __future__ import annotations

import os
from pathlib import Path

import click
import numpy as np
import onnx
import onnxruntime as ort
import torch
from torch import ByteTensor, Tensor, nn

from xfeat_export.export_lighterglue_onnx import (
    LighterGlueFixedWrapper,
    default_weights_path as default_lighterglue_weights_path,
)
from xfeat_export.export_xfeat_onnx import (
    XFeatNv12TopKWrapper,
    default_weights_path as default_xfeat_weights_path,
    validate_image_size,
)


class XFeatLighterGlueWrapper(nn.Module):
    def __init__(
        self,
        xfeat_weights_path: Path,
        lighterglue_weights_path: Path,
        *,
        keypoint_count: int,
        detection_threshold: float,
        min_confidence: float,
    ) -> None:
        super().__init__()
        self.extractor = XFeatNv12TopKWrapper(
            xfeat_weights_path,
            keypoint_count=keypoint_count,
            detection_threshold=detection_threshold,
        )
        self.matcher = LighterGlueFixedWrapper(
            lighterglue_weights_path,
            min_confidence=min_confidence,
        )

    def forward(
        self,
        current_left: ByteTensor,
        current_right: ByteTensor,
        previous_left_keypoints: Tensor,
        previous_left_descriptors: Tensor,
        previous_left_valid: Tensor,
    ) -> tuple[
        Tensor,
        Tensor,
        Tensor,
        Tensor,
        Tensor,
        Tensor,
        Tensor,
        Tensor,
        Tensor,
        Tensor,
        Tensor,
        Tensor,
        Tensor,
        Tensor,
    ]:
        current_left_rgb = self.extractor._preprocess(current_left)
        current_right_rgb = self.extractor._preprocess(current_right)
        rgb_images = torch.cat([current_left_rgb, current_right_rgb], dim=0)
        keypoints, descriptors, _, valid = self.extractor.forward_rgb(rgb_images)

        stereo_matches, stereo_reverse_matches, stereo_scores, stereo_reverse_scores = self.matcher(
            keypoints[0:1],
            keypoints[1:2],
            descriptors[0:1],
            descriptors[1:2],
            valid[0:1],
            valid[1:2],
        )
        temporal_matches, temporal_reverse_matches, temporal_scores, temporal_reverse_scores = self.matcher(
            previous_left_keypoints.unsqueeze(0),
            keypoints[0:1],
            previous_left_descriptors.unsqueeze(0),
            descriptors[0:1],
            previous_left_valid.unsqueeze(0),
            valid[0:1],
        )

        return (
            keypoints[0],
            descriptors[0],
            valid[0],
            keypoints[1],
            descriptors[1],
            valid[1],
            stereo_matches.squeeze(0),
            stereo_scores.squeeze(0),
            stereo_reverse_matches.squeeze(0),
            stereo_reverse_scores.squeeze(0),
            temporal_matches.squeeze(0),
            temporal_scores.squeeze(0),
            temporal_reverse_matches.squeeze(0),
            temporal_reverse_scores.squeeze(0),
        )


def validate_exported_model(
    export_path: Path,
    height: int,
    width: int,
    keypoint_count: int,
) -> None:
    onnx.checker.check_model(onnx.load(export_path))

    session = ort.InferenceSession(str(export_path), providers=["CPUExecutionProvider"])
    image_shape = [height // 2, width // 2, 6]
    expected_inputs = [
        ("current_left", "tensor(uint8)", image_shape),
        ("current_right", "tensor(uint8)", image_shape),
        ("previous_left_keypoints", "tensor(float)", [keypoint_count, 2]),
        ("previous_left_descriptors", "tensor(float)", [keypoint_count, 64]),
        ("previous_left_valid", "tensor(bool)", [keypoint_count]),
    ]
    actual_inputs = [(input.name, input.type, input.shape) for input in session.get_inputs()]
    if actual_inputs != expected_inputs:
        raise RuntimeError(f"unexpected ONNX inputs: {actual_inputs}")

    image_shape = (height // 2, width // 2, 6)
    current_left = np.zeros(image_shape, dtype=np.uint8)
    current_right = np.zeros(image_shape, dtype=np.uint8)
    previous_keypoints = np.zeros((keypoint_count, 2), dtype=np.float32)
    previous_descriptors = np.zeros((keypoint_count, 64), dtype=np.float32)
    previous_valid = np.zeros((keypoint_count,), dtype=np.bool_)

    outputs = run_validation_inference(
        session,
        current_left,
        current_right,
        previous_keypoints,
        previous_descriptors,
        previous_valid,
        keypoint_count,
    )
    run_validation_inference(
        session,
        current_left,
        current_right,
        outputs["current_left_keypoints"],
        outputs["current_left_descriptors"],
        outputs["current_left_valid"],
        keypoint_count,
    )


def run_validation_inference(
    session: ort.InferenceSession,
    current_left: np.ndarray,
    current_right: np.ndarray,
    previous_left_keypoints: np.ndarray,
    previous_left_descriptors: np.ndarray,
    previous_left_valid: np.ndarray,
    keypoint_count: int,
) -> dict[str, np.ndarray]:
    outputs = dict(
        zip(
            [output.name for output in session.get_outputs()],
            session.run(
                None,
                {
                    "current_left": current_left,
                    "current_right": current_right,
                    "previous_left_keypoints": previous_left_keypoints,
                    "previous_left_descriptors": previous_left_descriptors,
                    "previous_left_valid": previous_left_valid,
                },
            ),
            strict=True,
        )
    )
    expected_outputs = {
        "current_left_keypoints": (np.float32, (keypoint_count, 2)),
        "current_left_descriptors": (np.float32, (keypoint_count, 64)),
        "current_left_valid": (np.bool_, (keypoint_count,)),
        "current_right_keypoints": (np.float32, (keypoint_count, 2)),
        "current_right_descriptors": (np.float32, (keypoint_count, 64)),
        "current_right_valid": (np.bool_, (keypoint_count,)),
        "stereo_matches": (np.int32, (keypoint_count,)),
        "stereo_scores": (np.float32, (keypoint_count,)),
        "stereo_reverse_matches": (np.int32, (keypoint_count,)),
        "stereo_reverse_scores": (np.float32, (keypoint_count,)),
        "temporal_matches": (np.int32, (keypoint_count,)),
        "temporal_scores": (np.float32, (keypoint_count,)),
        "temporal_reverse_matches": (np.int32, (keypoint_count,)),
        "temporal_reverse_scores": (np.float32, (keypoint_count,)),
    }
    if set(outputs) != set(expected_outputs):
        raise RuntimeError(f"unexpected ONNX outputs: {sorted(outputs)}")

    for name, (dtype, shape) in expected_outputs.items():
        output = outputs[name]
        if output.dtype != dtype or output.shape != shape:
            raise RuntimeError(f"unexpected {name}: dtype={output.dtype}, shape={output.shape}")
        if np.issubdtype(output.dtype, np.floating) and not np.isfinite(output).all():
            raise RuntimeError(f"{name} contains non-finite values")
        if "matches" in name and not (((output == -1) | ((0 <= output) & (output < keypoint_count))).all()):
            raise RuntimeError(f"{name} contains out-of-range match indices")

    return outputs


@click.command(context_settings={"help_option_names": ["-h", "--help"]})
@click.argument("export-path", type=click.Path(path_type=Path))
@click.option(
    "--xfeat-weights",
    type=click.Path(exists=True, path_type=Path),
    default=None,
    help="Path to the XFeat .pt weights. Defaults to the accelerated-features package weights.",
)
@click.option(
    "--lighterglue-weights",
    type=click.Path(exists=True, path_type=Path),
    default=None,
    help="Path to the XFeat LighterGlue .pt weights. Defaults to the accelerated-features package weights.",
)
@click.option("--height", default=488, show_default=True, help="Full-resolution dummy image height.")
@click.option("--width", default=544, show_default=True, help="Full-resolution dummy image width.")
@click.option("--keypoints", "keypoint_count", default=512, show_default=True, help="Fixed keypoint count.")
@click.option("--threshold", "detection_threshold", default=0.05, show_default=True, help="NMS detection threshold.")
@click.option("--min-confidence", default=0.1, show_default=True, help="Minimum match confidence.")
@click.option("--opset", default=20, show_default=True, help="ONNX opset version.")
@click.option("--device", default="cpu", show_default=True, help="Torch export device, e.g. cpu or cuda:0.")
def main(
    export_path: Path,
    *,
    xfeat_weights: Path | None,
    lighterglue_weights: Path | None,
    height: int,
    width: int,
    keypoint_count: int,
    detection_threshold: float,
    min_confidence: float,
    opset: int,
    device: str,
) -> None:
    validate_image_size(height, width)
    if keypoint_count <= 0:
        raise click.BadParameter("--keypoints must be > 0")

    wrapper = XFeatLighterGlueWrapper(
        xfeat_weights or default_xfeat_weights_path(),
        lighterglue_weights or default_lighterglue_weights_path(),
        keypoint_count=keypoint_count,
        detection_threshold=detection_threshold,
        min_confidence=min_confidence,
    ).to(device)
    wrapper.eval()

    image_shape = (height // 2, width // 2, 6)
    dummy_image = torch.zeros(image_shape, dtype=torch.uint8, device=device)
    dummy_keypoints = torch.zeros((keypoint_count, 2), dtype=torch.float32, device=device)
    dummy_descriptors = torch.zeros((keypoint_count, 64), dtype=torch.float32, device=device)
    dummy_valid = torch.zeros((keypoint_count,), dtype=torch.bool, device=device)
    input_names = [
        "current_left",
        "current_right",
        "previous_left_keypoints",
        "previous_left_descriptors",
        "previous_left_valid",
    ]
    output_names = [
        "current_left_keypoints",
        "current_left_descriptors",
        "current_left_valid",
        "current_right_keypoints",
        "current_right_descriptors",
        "current_right_valid",
        "stereo_matches",
        "stereo_scores",
        "stereo_reverse_matches",
        "stereo_reverse_scores",
        "temporal_matches",
        "temporal_scores",
        "temporal_reverse_matches",
        "temporal_reverse_scores",
    ]

    export_path.parent.mkdir(parents=True, exist_ok=True)
    torch.onnx.export(
        wrapper,
        (dummy_image, dummy_image, dummy_keypoints, dummy_descriptors, dummy_valid),
        export_path,
        input_names=input_names,
        output_names=output_names,
        dynamic_axes=None,
        opset_version=opset,
        external_data=False,
        dynamo=False,
    )
    validate_exported_model(export_path, height, width, keypoint_count)

    click.echo(f"Exported fused XFeat/LighterGlue ONNX model to: {os.path.abspath(export_path)}")


if __name__ == "__main__":
    main()
