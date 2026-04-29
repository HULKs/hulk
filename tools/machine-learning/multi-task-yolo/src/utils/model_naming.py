from enum import Enum
from typing import Self

import click


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

    def output_name(self) -> str:
        return f"{self.value}_output"


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
        match self.name:
            case str if str.startswith("yolo26m-pose"):
                return TaskType.POSE
            case str if str.startswith("yolo26m-seg"):
                return TaskType.SEGMENTATION
            case str if str.startswith("yolo26m"):
                return TaskType.OBJECT
            case _:
                raise ModelNameError(self.name)

    def is_finetuned_model(self) -> bool:
        return len(self.name.split("~")) == 2


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
