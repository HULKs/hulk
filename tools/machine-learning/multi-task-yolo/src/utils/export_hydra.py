from collections.abc import Mapping
from pathlib import Path
from typing import Any, cast

import click
import torch
from torch import ByteTensor, Tensor, nn

from model.hydra import Hydra
from utils.nv12_to_rgb import NV12ToRgb


def _set_export_mode(module: nn.Module) -> None:
    for child in module.modules():
        if hasattr(child, "export"):
            cast(Any, child).export = True


def _parse_head_pairs(head_pairs: tuple[str, ...]) -> dict[str, str]:
    head_paths: dict[str, str] = {}

    for pair in head_pairs:
        if "=" not in pair:
            raise click.BadParameter("Invalid head format")  # noqa: TRY003

        head_name, model_path = pair.split("=", maxsplit=1)
        head_name = head_name.strip()
        model_path = model_path.strip()

        if not head_name or not model_path:
            raise click.BadParameter("Invalid head format")  # noqa: TRY003

        if head_name in head_paths:
            raise click.BadParameter("Duplicate head name")  # noqa: TRY003

        if not Path(model_path).exists():
            raise click.BadParameter("Missing model path")  # noqa: TRY003

        head_paths[head_name] = model_path

    return head_paths


class HydraWrapper(nn.Module):
    def __init__(self, hydra_model: Hydra, head_names: list[str]) -> None:
        super().__init__()
        self.hydra = hydra_model
        self.head_names = head_names

    def forward(self, x: Tensor) -> Tensor | tuple[Tensor, ...]:
        outputs = self.hydra(x)
        if not isinstance(outputs, Mapping):
            raise TypeError("Hydra model output must be a mapping")  # noqa: TRY003

        selected_outputs: list[Tensor] = []
        for head_name in self.head_names:
            head_output = outputs.get(head_name)
            if not isinstance(head_output, torch.Tensor):
                raise TypeError("Hydra head output must be tensor")  # noqa: TRY003
            selected_outputs.append(head_output)

        if len(selected_outputs) == 1:
            return selected_outputs[0]
        return tuple(selected_outputs)


class HydraNv12Wrapper(nn.Module):
    def __init__(self, hydra_wrapper: HydraWrapper) -> None:
        super().__init__()
        self.hydra_wrapper = hydra_wrapper
        self.preprocessor = NV12ToRgb(subsample=False)

    def forward(self, x: ByteTensor) -> Tensor | tuple[Tensor, ...]:
        rgb = self.preprocessor(x).unsqueeze(0).permute(0, 3, 1, 2)
        return self.hydra_wrapper(rgb)


def _export_onnx(
    wrapper: nn.Module,
    dummy_input: Tensor,
    export_path: Path,
    head_names: list[str],
    opset: int,
    *,
    with_nv12: bool,
) -> None:
    output_names = [f"{head_name}_output" for head_name in head_names]

    input_name = "images"
    dynamic_axes: dict[str, dict[int, str]]
    if with_nv12:
        input_name = "raw_bytes_input"
        dynamic_axes = {
            input_name: {0: "half_height", 1: "half_width"},
        }
    else:
        dynamic_axes = {
            input_name: {0: "batch_size", 2: "height", 3: "width"},
        }

    for output_name in output_names:
        dynamic_axes[output_name] = {
            0: "batch_size",
            2: "num_predictions",
        }

    torch.onnx.export(
        wrapper,
        (dummy_input,),
        export_path,
        input_names=[input_name],
        output_names=output_names,
        dynamic_axes=dynamic_axes,
        opset_version=opset,
        external_data=False,
        dynamo=False,
    )


def _export_torchscript(
    wrapper: nn.Module,
    dummy_input: Tensor,
    export_path: Path,
) -> None:
    traced = torch.jit.trace(
        wrapper,
        (dummy_input,),
        strict=False,
        check_trace=False,
    )
    if isinstance(traced, tuple):
        raise TypeError("Unexpected trace return type")  # noqa: TRY003
    cast(torch.jit.ScriptModule, traced).save(str(export_path))


@click.command()
@click.argument(
    "backbone",
    type=click.Path(exists=True, path_type=Path),
)
@click.option(
    "--head",
    "head_pairs",
    multiple=True,
    required=True,
    help=(
        "Task head mapping in the form NAME=MODEL_PATH. "
        "Example: --head detection=assets/yolo26m.pt"
    ),
)
@click.argument(
    "export-path",
    type=click.Path(path_type=Path),
)
@click.option(
    "--imgsz",
    type=int,
    default=640,
    show_default=True,
    help="Square input image size used for ONNX tracing.",
)
@click.option(
    "--opset",
    type=int,
    default=20,
    show_default=True,
    help="ONNX opset version.",
)
@click.option(
    "--format",
    "export_format",
    type=click.Choice(["onnx", "pt"], case_sensitive=False),
    default="onnx",
    show_default=True,
    help="Export format: ONNX or TorchScript .pt.",
)
@click.option(
    "--device",
    default="cpu",
    show_default=True,
    help="Torch device for export, e.g. cpu or cuda:0.",
)
@click.option(
    "--with-nv12-layer",
    is_flag=True,
    default=False,
    help="Add NV12 preprocessing layer before Hydra model.",
)
def main(
    backbone: Path,
    head_pairs: tuple[str, ...],
    export_path: Path,
    *,
    imgsz: int,
    opset: int,
    export_format: str,
    device: str,
    with_nv12_layer: bool,
) -> None:
    if imgsz <= 0:
        raise click.BadParameter("--imgsz must be > 0")  # noqa: TRY003

    head_paths = _parse_head_pairs(head_pairs)
    head_names = list(head_paths.keys())

    hydra_model = Hydra(
        backbone_path=str(backbone),
        task_dict=head_paths,
    ).to(device)
    hydra_model.eval()
    _set_export_mode(hydra_model)

    base_wrapper = HydraWrapper(hydra_model, head_names).to(device)
    wrapper: nn.Module = base_wrapper
    if with_nv12_layer:
        wrapper = HydraNv12Wrapper(base_wrapper).to(device)
    wrapper.eval()

    export_path.parent.mkdir(parents=True, exist_ok=True)

    if with_nv12_layer:
        if imgsz % 2 != 0:
            raise click.BadParameter("--imgsz must be even for NV12")  # noqa: TRY003
        dummy_input = torch.zeros(
            (imgsz // 2, imgsz // 2, 6),
            dtype=torch.uint8,
            device=device,
        )
    else:
        dummy_input = torch.zeros(
            (1, 3, imgsz, imgsz),
            dtype=torch.float32,
            device=device,
        )

    if export_format == "onnx":
        _export_onnx(
            wrapper=wrapper,
            dummy_input=dummy_input,
            export_path=export_path,
            head_names=head_names,
            opset=opset,
            with_nv12=with_nv12_layer,
        )
        click.echo(f"Exported Hydra ONNX model to: {export_path}")
        return

    _export_torchscript(
        wrapper=wrapper,
        dummy_input=dummy_input,
        export_path=export_path,
    )
    click.echo(f"Exported Hydra TorchScript model to: {export_path}")


if __name__ == "__main__":
    main()
