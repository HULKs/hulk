import os
import re
from collections import Counter
from collections.abc import Mapping
from pathlib import Path
from typing import Any, cast

import click
import torch
from torch import ByteTensor, Tensor, nn

from model.hydra import Hydra, HydraHeadSpec, OutputSpec
from utils.model_naming import (
    HYDRA_MODEL_NAME_TYPE,
    HydraModelName,
    TaskType,
    resolve_model_path,
)
from utils.nv12_to_rgb import NV12ToRgb

OUTPUT_NAME_PATTERN = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")


class InvalidHydraOutputError(TypeError):
    def __init__(self, output_name: str, actual_type: type) -> None:
        super().__init__(
            f"Hydra output '{output_name}' must be a tensor, got {actual_type}"
        )


class HydraWrapper(nn.Module):
    def __init__(
        self, hydra_model: Hydra, head_specs: list[HydraHeadSpec]
    ) -> None:
        super().__init__()
        self.hydra = hydra_model
        self.head_specs = head_specs

    def forward(self, x: Tensor) -> Tensor | tuple[Tensor, ...]:
        outputs = self.hydra(x)
        if not isinstance(outputs, Mapping):
            raise TypeError("Hydra model output must be a mapping")  # noqa: TRY003

        selected_outputs: list[Tensor] = []
        for head_spec in self.head_specs:
            for output_name in head_spec.output_names():
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


def parse_head_output_names(head_outputs: tuple[str, ...]) -> dict[str, str]:
    output_names: dict[str, str] = {}
    for head_output in head_outputs:
        head_name, separator, output_name = head_output.partition("=")
        if not separator or not head_name or not output_name:
            raise click.BadParameter(  # noqa: TRY003
                "--head-output must use HEAD=OUTPUT_NAME"
            )
        if not OUTPUT_NAME_PATTERN.fullmatch(output_name):
            raise click.BadParameter(  # noqa: TRY003
                "output names must match [A-Za-z_][A-Za-z0-9_]*"
            )
        if head_name in output_names:
            raise click.BadParameter(  # noqa: TRY003
                f"output name for head '{head_name}' was provided twice"
            )
        output_names[head_name] = output_name
    return output_names


def output_specs_for_head(
    task_type: TaskType, output_name: str | None
) -> tuple[OutputSpec, ...]:
    default_specs = task_type.output_specs()
    if output_name is None:
        return tuple(default_specs)

    if task_type != TaskType.SEGMENTATION:
        return ((output_name, default_specs[0][1]),)

    base_name = output_name.removesuffix("_output")
    return (
        (output_name, default_specs[0][1]),
        (f"{base_name}_proto", default_specs[1][1]),
    )


def check_output_names(head_specs: list[HydraHeadSpec]) -> None:
    output_names = [
        output_name
        for head_spec in head_specs
        for output_name in head_spec.output_names()
    ]
    duplicate_output_names = [
        output_name
        for output_name, count in Counter(output_names).items()
        if count > 1
    ]
    if duplicate_output_names:
        raise click.BadParameter(
            "duplicate output name(s): " + ", ".join(duplicate_output_names)
        )

    reserved_names = {"images", "raw_bytes_input"}
    reserved_output_names = sorted(reserved_names.intersection(output_names))
    if reserved_output_names:
        raise click.BadParameter(
            "output name(s) conflict with input name(s): "
            + ", ".join(reserved_output_names)
        )


def build_head_specs(
    hydra_model_name: HydraModelName,
    train_folder_path: Path,
    val_folder_path: Path,
    output_name_by_head: dict[str, str],
) -> list[HydraHeadSpec]:
    task_counts = Counter(head.task_type() for head in hydra_model_name.heads)
    missing_output_names = [
        head.name
        for head in hydra_model_name.heads
        if task_counts[head.task_type()] > 1
        and head.name not in output_name_by_head
    ]
    if missing_output_names:
        raise click.BadParameter(
            "duplicate task heads require --head-output for: "
            + ", ".join(missing_output_names)
        )

    head_specs: list[HydraHeadSpec] = []
    for index, head in enumerate(hydra_model_name.heads):
        integrated_model_name = hydra_model_name.integrated_model_name(head)
        task_type = head.task_type()
        path = (
            train_folder_path / integrated_model_name / "weights/best.pt"
            if head.is_finetuned_model()
            else val_folder_path
            / integrated_model_name
            / f"{integrated_model_name}.pt"
        )
        head_specs.append(
            HydraHeadSpec(
                name=f"head_{index}",
                task_type=task_type,
                path=path,
                output_specs=output_specs_for_head(
                    task_type,
                    output_name_by_head.get(head.name),
                ),
            )
        )

    check_output_names(head_specs)
    return head_specs


def export_onnx(
    wrapper: nn.Module,
    dummy_input: Tensor,
    export_path: Path,
    head_specs: list[HydraHeadSpec],
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
    for head_spec in head_specs:
        for name, axes in head_spec.output_specs:
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
    "--assets_dir",
    type=Path,
    default=Path("assets"),
    help="Directory containing base YOLO checkpoints.",
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
@click.option(
    "--head-output",
    multiple=True,
    metavar="HEAD=OUTPUT_NAME",
    help=(
        "Override a head output name. Required for every head when multiple "
        "heads have the same task type."
    ),
)
def main(
    hydra_model_names: list[HydraModelName],
    export_folder: Path,
    *,
    assets_dir: Path,
    runs_dir: Path,
    val_dir: Path,
    train_dir: Path,
    imgsz: int,
    opset: int,
    export_format: str,
    device: str,
    with_nv12_layer: bool,
    head_output: tuple[str, ...],
) -> None:
    if imgsz <= 0:
        raise click.BadParameter("--imgsz must be > 0")  # noqa: TRY003

    train_folder_path = runs_dir / train_dir
    val_folder_path = runs_dir / val_dir
    output_name_by_head = parse_head_output_names(head_output)

    known_head_names = {
        head.name
        for hydra_model_name in hydra_model_names
        for head in hydra_model_name.heads
    }
    unknown_head_names = sorted(
        set(output_name_by_head).difference(known_head_names)
    )
    if unknown_head_names:
        raise click.BadParameter(
            "--head-output references unknown head(s): "
            + ", ".join(unknown_head_names)
        )

    for hydra_model_name in hydra_model_names:
        backbone = hydra_model_name.backbone

        head_specs = build_head_specs(
            hydra_model_name=hydra_model_name,
            train_folder_path=train_folder_path,
            val_folder_path=val_folder_path,
            output_name_by_head=output_name_by_head,
        )

        hydra_model = Hydra(
            backbone_path=str(resolve_model_path(backbone.name, assets_dir)),
            heads=head_specs,
            number_of_frozen_modules=(
                hydra_model_name.number_of_frozen_modules
            ),
        ).to(device)
        hydra_model.eval()
        set_export_mode(hydra_model)

        base_wrapper = HydraWrapper(hydra_model, head_specs=head_specs).to(
            device
        )
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
                head_specs=head_specs,
                opset=opset,
                with_nv12=with_nv12_layer,
            )
            click.echo(
                "Exported Hydra ONNX model to: "
                f"{os.path.abspath(export_folder)}"
            )
            continue

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
