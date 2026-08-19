from enum import Enum
from pathlib import Path, PurePath
from typing import Self

import click

YOLO26_SIZES = ("n", "s", "m", "l", "x")


def _yolo26_variant_suffix(stem: str) -> str | None:
    for size in YOLO26_SIZES:
        prefix = f"yolo26{size}"
        if stem == prefix:
            return ""
        if stem.startswith(f"{prefix}-"):
            return stem.removeprefix(prefix)
    return None


def _model_path_candidates(model_path: Path) -> tuple[Path, ...]:
    if model_path.suffix:
        return (model_path,)
    return (
        model_path,
        model_path.with_suffix(".pt"),
        model_path.with_suffix(".yaml"),
    )


def resolve_model_path(model_name: str, assets_dir: Path) -> str | Path:
    model_path = Path(model_name)
    if model_path.is_absolute() or model_path.parent != Path("."):
        candidates = _model_path_candidates(model_path)
    else:
        candidates = _model_path_candidates(assets_dir / model_path)

    for candidate in candidates:
        if candidate.exists():
            return candidate

    if model_path.suffix:
        return model_name
    return str(model_path.with_suffix(".pt"))


class ModelNameError(Exception):
    def __init__(self, name: str) -> None:
        self.name = name
        super().__init__(f"Unknown model name: {name}")


class TaskType(Enum):
    OBJECT = "object"
    POSE = "pose"
    SEGMENTATION = "segmentation"

    def __str__(self) -> str:
        return self.value

    def output_specs(self) -> list[tuple[str, dict[int, str]]]:
        base = (f"{self.value}_output", {0: "batch_size", 1: "num_predictions"})
        if self == TaskType.SEGMENTATION:
            return [base, (f"{self.value}_proto", {0: "batch_size"})]
        return [base]

    def output_names(self) -> list[str]:
        return [name for name, _ in self.output_specs()]


class ModelName:
    name: str

    def __init__(self, name: str | Self) -> None:
        if isinstance(name, ModelName):
            self.name = name.name
        else:
            self.name = name

    def __str__(self) -> str:
        return f"{self.name}"

    def task_type(self) -> TaskType:
        stem = PurePath(self.name).stem.split("~", maxsplit=1)[0]
        variant_suffix = _yolo26_variant_suffix(stem)
        if variant_suffix is None:
            raise ModelNameError(self.name)

        if variant_suffix.startswith("-pose"):
            return TaskType.POSE
        if variant_suffix.startswith("-seg"):
            return TaskType.SEGMENTATION
        return TaskType.OBJECT

    def is_finetuned_model(self) -> bool:
        return "~" in self.name


class HydraModelName:
    backbone: ModelName
    heads: list[ModelName]
    number_of_frozen_modules: int

    def __init__(
        self,
        backbone: ModelName | str,
        heads: list[ModelName] | list[str],
        number_of_frozen_modules: int,
    ) -> None:
        self.backbone = ModelName(backbone)
        self.heads = [ModelName(head) for head in heads]
        self.number_of_frozen_modules = number_of_frozen_modules

    @classmethod
    def parse(cls, model_string: str) -> "HydraModelName":
        backbone_and_frozen_layers, *heads = model_string.split("+")
        heads = [ModelName(head) for head in heads]
        backbone, number_of_frozen_modules = backbone_and_frozen_layers.split(
            "="
        )

        return cls(backbone, heads, int(number_of_frozen_modules.strip("f")))

    def __str__(self) -> str:
        heads = "+".join(str(head) for head in self.heads)
        return f"{self.backbone}=f{self.number_of_frozen_modules}+{heads}"

    def integrated_model_name(self, model_name: ModelName) -> str | None:
        return (
            f"{self.backbone}=f{self.number_of_frozen_modules}+{model_name!s}"
        )


class HydraModelNameParam(click.ParamType):
    name = "hydra-model-name"

    def convert(
        self,
        value: str,
        param: click.Parameter | None,
        ctx: click.Context | None,
    ) -> HydraModelName:
        try:
            return HydraModelName.parse(value)
        except ValueError as exc:
            self.fail(str(exc), param, ctx)


HYDRA_MODEL_NAME_TYPE = HydraModelNameParam()
