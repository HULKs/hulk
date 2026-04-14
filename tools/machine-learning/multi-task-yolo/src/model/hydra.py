import logging
from collections.abc import Mapping, Sequence
from typing import Any, cast
from zipfile import Path

import torch
import torch.nn as nn
from ultralytics.models.yolo.model import YOLO
from ultralytics.nn.autobackend import check_class_names
from ultralytics.nn.tasks import DetectionModel

logger = logging.getLogger(__name__)

ClassNames = Mapping[int, str] | Sequence[str] | None


def get_backbone_length(yaml_config: dict) -> int:
    """Returns the index of the last layer of the backbone."""
    return len(yaml_config.get("backbone", []))


def get_backbone(model: DetectionModel) -> nn.ModuleList:
    """Extracts the backbone as an nn.ModuleList dynamically."""
    split_idx = get_backbone_length(cast(dict[str, Any], model.yaml))
    return nn.ModuleList(list(model.model.children())[:split_idx])


def get_head(model: DetectionModel) -> nn.ModuleList:
    """Extracts the neck + head head dynamically."""
    split_idx = get_backbone_length(cast(dict[str, Any], model.yaml))
    return nn.ModuleList(list(model.model.children())[split_idx:])


def set_backbone(model: DetectionModel, backbone: nn.ModuleList) -> None:
    """Replaces the backbone modules of a model."""
    split_idx = get_backbone_length(cast(dict[str, Any], model.yaml))
    head = list(model.model.children())

    nodes = list(backbone) + head[split_idx:]
    model.model = nn.Sequential(*nodes)


def _normalize_class_names(class_names: ClassNames) -> dict[int, str]:
    if isinstance(class_names, Mapping):
        return check_class_names(dict(class_names))
    if isinstance(class_names, Sequence) and not isinstance(class_names, str):
        return check_class_names(list(class_names))
    return {}


class MissingHydraHeadError(KeyError):
    def __init__(self, head_name: str) -> None:
        super().__init__(f"Hydra head '{head_name}' was not found")


class UnsupportedHydraHeadError(ValueError):
    def __init__(self, head_name: str) -> None:
        super().__init__(f"Hydra head '{head_name}' is not mapped to a task")


class InvalidHydraOutputError(TypeError):
    def __init__(self, head_name: str) -> None:
        super().__init__(
            f"Hydra output did not contain expected head '{head_name}'"
        )


class Hydra(nn.Module):
    def __init__(
        self,
        backbone_path: str,
        task_dict: dict[str, str],
    ) -> None:
        super().__init__()

        logger.info("Loading backbone from: %s", backbone_path)
        backbone_yolo = YOLO(backbone_path)
        backbone_root = cast(DetectionModel, backbone_yolo.model)

        self.backbone_length = get_backbone_length(
            cast(dict, backbone_root.yaml)
        )

        backbone_model_name = backbone_yolo.model_name or "unknown"
        self.backbone_name = Path(backbone_model_name).stem
        self.shared_backbone = get_backbone(backbone_root)
        self.save_backbone = cast(list[int], backbone_root.save)

        self.heads = nn.ModuleDict()
        self.branch_saves: dict[str, list[int]] = {}
        self.head_class_names: dict[str, Any] = {}
        self.head_model_names: dict[str, Any] = {}
        self.head_strides: dict[str, torch.Tensor] = {}
        self.head_end2end: dict[str, bool] = {}
        self.head_kpt_shapes: dict[str, tuple[int, int] | None] = {}

        for task_name, model_path in task_dict.items():
            logger.info("Extracting %s head from: %s", task_name, model_path)
            task_yolo = YOLO(model_path)
            task_root = cast(DetectionModel, task_yolo.model)
            task_head = task_root.model[-1]

            self.heads[task_name] = get_head(task_root)
            self.branch_saves[task_name] = cast(list[int], task_root.save)
            self.head_class_names[task_name] = getattr(task_root, "names", {})
            task_model_name = task_yolo.model_name or "unknown"
            self.head_model_names[task_name] = Path(task_model_name).stem
            stride = getattr(task_head, "stride", torch.tensor([8, 16, 32]))
            self.head_strides[task_name] = torch.as_tensor(stride)
            self.head_end2end[task_name] = bool(
                getattr(
                    task_head,
                    "end2end",
                    getattr(task_root, "end2end", False),
                )
            )

            raw_kpt_shape = getattr(task_head, "kpt_shape", None)
            if (
                isinstance(raw_kpt_shape, (list, tuple))
                and len(raw_kpt_shape) >= 2
            ):
                self.head_kpt_shapes[task_name] = (
                    int(raw_kpt_shape[0]),
                    int(raw_kpt_shape[1]),
                )
            else:
                self.head_kpt_shapes[task_name] = None

    def forward(self, x: torch.Tensor) -> dict[str, Any]:
        y_backbone: list[torch.Tensor | None] = []
        backbone_activations: Any = x

        for i, m in enumerate(self.shared_backbone):
            from_index = cast(Any, m.f)
            if from_index != -1:
                backbone_activations = (
                    y_backbone[from_index]
                    if isinstance(from_index, int)
                    else [
                        backbone_activations if j == -1 else y_backbone[j]
                        for j in cast(list[int], from_index)
                    ]
                )
            backbone_activations = m(backbone_activations)
            y_backbone.append(
                backbone_activations if i in self.save_backbone else None
            )

        outputs: dict[str, Any] = {}

        for head_name, head_module in self.heads.items():
            head = cast(nn.ModuleList, head_module)
            y_head = list(y_backbone)
            head_activations: list[torch.Tensor] | torch.Tensor = (
                backbone_activations
            )

            for i, m in enumerate(head):
                module_index = i + self.backbone_length

                from_index = cast(Any, m.f)
                if from_index != -1:
                    head_activations = (
                        cast(torch.Tensor, y_head[from_index])
                        if isinstance(from_index, int)
                        else [
                            cast(torch.Tensor, head_activations)
                            if j == -1
                            else cast(torch.Tensor, y_head[j])
                            for j in cast(list[int], from_index)
                        ]
                    )

                head_activations = m(head_activations)

                y_head.append(
                    cast(torch.Tensor, head_activations)
                    if module_index in self.branch_saves[head_name]
                    else None
                )

            outputs[head_name] = head_activations

        return outputs


class HydraTaskModelAdapter(nn.Module):
    def __init__(
        self,
        hydra_model: Hydra,
        head_name: str,
        head_model_name: str,
        task: str,
        names: ClassNames,
        stride: torch.Tensor | Sequence[int] | int,
        *,
        end2end: bool,
        kpt_shape: tuple[int, int] | None = None,
    ) -> None:
        super().__init__()
        self.hydra = hydra_model
        self.head_name = head_name
        self.head_model_name = head_model_name
        self.task = task

        self.names = _normalize_class_names(names)
        self.nc = len(self.names)

        self.stride = torch.as_tensor(stride)
        self.end2end = end2end
        self.format = "pt"
        self.fp16 = False
        self.dynamic = False
        self.yaml = {"channels": 3}

        if task == "pose" and kpt_shape is not None:
            self.kpt_shape = kpt_shape

    def forward(
        self,
        x: torch.Tensor,
        *,
        augment: bool = False,
        visualize: bool = False,
        embed: list[int] | None = None,
        **kwargs: Any,
    ) -> Any:
        del augment, visualize, embed, kwargs

        raw_outputs = self.hydra(x)
        if (
            not isinstance(raw_outputs, Mapping)
            or self.head_name not in raw_outputs
        ):
            raise InvalidHydraOutputError(self.head_name)
        return raw_outputs[self.head_name]

    def set_head_attr(self, **kwargs: Any) -> None:
        if self.head_name not in self.hydra.heads:
            return
        head = cast(nn.ModuleList, self.hydra.heads[self.head_name])
        head_module = head[-1]
        set_head_attr_fn = getattr(head_module, "set_head_attr", None)
        if callable(set_head_attr_fn):
            set_head_attr_fn(**kwargs)
