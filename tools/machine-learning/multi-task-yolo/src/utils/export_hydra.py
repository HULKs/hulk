import os
from collections.abc import Mapping
from pathlib import Path
from typing import Any, cast

import click
import torch
from torch import ByteTensor, Tensor, nn

from model.hydra import Hydra
from utils.model_naming import HYDRA_MODEL_NAME_TYPE, HydraModelName, TaskType
from utils.nv12_to_rgb import NV12ToRgb


class InvalidHydraOutputError(TypeError):
    def __init__(self, output_name: str, actual_type: type) -> None:
        super().__init__(
            f"Hydra output '{output_name}' must be a tensor, got {actual_type}"
        )


class HydraWrapper(nn.Module):
    def __init__(
        self, hydra_model: Hydra, task_dict: dict[TaskType, Path]
    ) -> None:
        super().__init__()
        self.hydra = hydra_model
        self.task_dict = task_dict

    def forward(self, x: Tensor) -> Tensor | tuple[Tensor, ...]:
        outputs = self.hydra(x)
        if not isinstance(outputs, Mapping):
            raise TypeError("Hydra model output must be a mapping")  # noqa: TRY003

        selected_outputs: list[Tensor] = []
        for task_type in self.task_dict:
            for output_name in task_type.output_names():
                head_output = outputs.get(output_name)
                if not isinstance(head_output, torch.Tensor):
                    raise InvalidHydraOutputError(
                        output_name, type(head_output)
                    )
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


def set_export_mode(module: nn.Module) -> None:
    for child in module.modules():
        if hasattr(child, "export"):
            cast(Any, child).export = True


def build_task_dict(
    hydra_model_name: HydraModelName,
    train_folder_path: Path,
    val_folder_path: Path,
) -> dict[TaskType, Path]:
    return {
        head.task_type(): (
            train_folder_path
            / hydra_model_name.integrated_model_name(head)
            / "weights/best.pt"
            if head.is_finetuned_model()
            else val_folder_path
            / hydra_model_name.integrated_model_name(head)
            / (hydra_model_name.integrated_model_name(head) + ".pt")
        )
        for head in hydra_model_name.heads
    }


def export_onnx(
    wrapper: nn.Module,
    dummy_input: Tensor,
    export_path: Path,
    task_dict: dict[TaskType, Path],
    opset: int,
    *,
    with_nv12: bool,
) -> None:
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

    output_names: list[str] = []
    for task_type in task_dict:
        for name, axes in task_type.output_specs():
            output_names.append(name)
            dynamic_axes[name] = axes

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


def export_torchscript(
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


@click.command(
    context_settings={"help_option_names": ["-h", "--help"]},
    help=(
        "Export one or more Hydra models to ONNX or TorchScript format.\n\n"
        "Arguments:\n\n"
        "  HYDRA_MODEL_NAME  One or more Hydra model names to export\n\n"
        "  EXPORT_FOLDER     Destination folder for the exported model(s)"
    ),
)
@click.argument(
    "hydra-model-names",
    nargs=-1,
    type=HYDRA_MODEL_NAME_TYPE,
)
@click.argument(
    "export-folder",
    nargs=1,
    type=click.Path(path_type=Path),
)
@click.option(
    "--runs_dir",
    type=Path,
    default=Path("runs"),
    help="Directory to save training runs.",
)
@click.option(
    "--val_dir",
    type=Path,
    default=Path("val"),
    help="Directory to save validation runs. Relative to `--runs_dir`.",
)
@click.option(
    "--train_dir",
    type=Path,
    default=Path("train"),
    help="Directory to save validation runs. Relative to `--runs_dir`.",
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
    hydra_model_names: list[HydraModelName],
    export_folder: Path,
    *,
    runs_dir: Path,
    val_dir: Path,
    train_dir: Path,
    imgsz: int,
    opset: int,
    export_format: str,
    device: str,
    with_nv12_layer: bool,
) -> None:
    if imgsz <= 0:
        raise click.BadParameter("--imgsz must be > 0")  # noqa: TRY003

    train_folder_path = runs_dir / train_dir
    val_folder_path = runs_dir / val_dir

    for hydra_model_name in hydra_model_names:
        backbone = hydra_model_name.backbone

        task_dict = build_task_dict(
            hydra_model_name=hydra_model_name,
            train_folder_path=train_folder_path,
            val_folder_path=val_folder_path,
        )

        hydra_model = Hydra(
            backbone_path=str(backbone),
            task_dict=task_dict,
        ).to(device)
        hydra_model.eval()
        set_export_mode(hydra_model)

        base_wrapper = HydraWrapper(hydra_model, task_dict=task_dict).to(device)
        wrapper: nn.Module = base_wrapper
        if with_nv12_layer:
            wrapper = HydraNv12Wrapper(base_wrapper).to(device)
        wrapper.eval()

        export_folder.mkdir(parents=True, exist_ok=True)

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
            export_onnx(
                wrapper=wrapper,
                dummy_input=dummy_input,
                export_path=export_folder / (str(hydra_model_name) + ".onnx"),
                task_dict=task_dict.keys(),
                opset=opset,
                with_nv12=with_nv12_layer,
            )
            click.echo(
                "Exported Hydra ONNX model to: "
                f"{os.path.abspath(export_folder)}"
            )
            return

        export_torchscript(
            wrapper=wrapper,
            dummy_input=dummy_input,
            export_path=export_folder / (str(hydra_model_name) + ".onnx"),
        )
        click.echo(
            "Exported Hydra TorchScript model to: "
            f"{os.path.abspath(export_folder)}"
        )


if __name__ == "__main__":
    main()
