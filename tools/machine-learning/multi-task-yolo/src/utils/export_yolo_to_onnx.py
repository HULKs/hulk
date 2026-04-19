from pathlib import Path

import click
import torch
from torch import ByteTensor, Tensor, nn

from utils.nv12_to_rgb import NV12ToRgb


def load_raw_yolo_network(model_path: Path) -> nn.Module:
    checkpoint = torch.load(model_path, map_location="cpu", weights_only=False)
    yolo_network: nn.Module = checkpoint["model"].float().eval()

    for module in yolo_network.modules():
        if hasattr(module, "export"):
            module.export = True

    return yolo_network


class YoloNv12Wrapper(nn.Module):
    def __init__(self, yolo_model_path: Path, *, subsample: bool) -> None:
        super().__init__()
        self.subsample = subsample
        self.yolo = load_raw_yolo_network(yolo_model_path)
        self.preprocessor = NV12ToRgb(subsample=subsample)

    def forward(self, x: ByteTensor) -> Tensor:
        rgb = self.preprocessor(x).unsqueeze(0).permute(0, 3, 1, 2)
        detections = self.yolo(rgb)
        if self.subsample:
            detections = detections[..., :4] * 2
        return detections


@click.command()
@click.argument(
    "model-path",
    type=click.Path(exists=True, path_type=Path),
)
@click.argument(
    "export-path",
    type=click.Path(path_type=Path),
)
@click.option(
    "--subsample",
    is_flag=True,
    help="Whether to subsample the chroma channels.",
)
def main(
    model_path: Path,
    export_path: Path,
    *,
    subsample: bool,
) -> None:
    wrapper = YoloNv12Wrapper(model_path, subsample=subsample)
    wrapper.eval()
    dummy_height = 320
    dummy_width = 640
    dummy_input = torch.zeros((dummy_height, dummy_width, 6), dtype=torch.uint8)

    torch.onnx.export(
        wrapper,
        (dummy_input,),
        export_path,
        input_names=["raw_bytes_input"],
        output_names=["network_detections"],
        dynamic_axes={
            "raw_bytes_input": {0: "half_height", 1: "half_width"},
            "network_detections": {0: "batch_size", 1: "detection_count"},
        },
        opset_version=20,
        external_data=False,
    )


if __name__ == "__main__":
    main()
