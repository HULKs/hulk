import argparse
import logging
import os
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any, cast

import torch
from torch import nn
from ultralytics.models.yolo.detect.val import DetectionValidator
from ultralytics.models.yolo.model import YOLO
from ultralytics.models.yolo.pose.val import PoseValidator
from ultralytics.nn.autobackend import check_class_names

from model.multi_task_yolo import Hydra

logger = logging.getLogger(__name__)

ClassNames = Mapping[int, str] | Sequence[str] | None


@dataclass(frozen=True)
class ValidationTaskConfig:
    data: str | Path
    split: str = "val"
    imgsz: int = 640
    batch: int = 16
    workers: int = 8
    conf: float = 0.001
    iou: float = 0.6
    max_det: int = 300
    half: bool = False
    plots: bool = True
    save_json: bool = False
    save_txt: bool = True
    project: str | Path = "runs"
    exist_ok: bool = True
    device: str | None = None


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


def _normalize_class_names(class_names: ClassNames) -> dict[int, str]:
    if isinstance(class_names, Mapping):
        return check_class_names(dict(class_names))
    if isinstance(class_names, Sequence) and not isinstance(class_names, str):
        return check_class_names(list(class_names))
    return {}


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


class MultiTaskHydraValidator:
    def __init__(
        self,
        hydra_model: Hydra,
        device: str | None = None,
    ) -> None:
        self.device = torch.device(
            device or ("cuda" if torch.cuda.is_available() else "cpu")
        )
        self.model = hydra_model.to(self.device)
        self.model.eval()

        self.task_adapters = self._build_task_adapters()

    @staticmethod
    def _head_to_task(head_name: str) -> str:
        if head_name == "detection":
            return "detect"
        if head_name == "pose":
            return "pose"
        raise UnsupportedHydraHeadError(head_name)

    def _build_task_adapters(self) -> dict[str, HydraTaskModelAdapter]:
        adapters: dict[str, HydraTaskModelAdapter] = {}

        head_class_names = cast(
            dict[str, ClassNames],
            getattr(self.model, "head_class_names", {}),
        )
        head_strides = cast(
            dict[str, torch.Tensor],
            getattr(self.model, "head_strides", {}),
        )
        head_end2end = cast(
            dict[str, bool],
            getattr(self.model, "head_end2end", {}),
        )
        head_kpt_shapes = cast(
            dict[str, tuple[int, int] | None],
            getattr(self.model, "head_kpt_shapes", {}),
        )

        for head_name in self.model.heads:
            task = self._head_to_task(head_name)
            head_model_name = getattr(self.model, "head_model_names", {}).get(
                head_name, "unknown"
            )

            adapters[head_name] = HydraTaskModelAdapter(
                hydra_model=self.model,
                head_name=head_name,
                head_model_name=head_model_name,
                task=task,
                names=head_class_names.get(head_name),
                stride=head_strides.get(head_name, torch.tensor([8, 16, 32])),
                end2end=head_end2end.get(head_name, True),
                kpt_shape=head_kpt_shapes.get(head_name),
            )

        return adapters

    def _build_validator_args(
        self,
        task: str,
        head_model_name: str,
        config: ValidationTaskConfig,
    ) -> dict[str, Any]:
        return {
            "task": task,
            "data": str(config.data),
            "split": config.split,
            "imgsz": config.imgsz,
            "batch": config.batch,
            "workers": config.workers,
            "conf": config.conf,
            "iou": config.iou,
            "max_det": config.max_det,
            "half": config.half,
            "plots": config.plots,
            "save_json": config.save_json,
            "save_txt": config.save_txt,
            "project": config.project,
            "name": Path("val")
            / (self.model.foundation_name + "_" + head_model_name),
            "exist_ok": config.exist_ok,
            "device": config.device or str(self.device),
        }

    def validate_task(
        self,
        head_name: str,
        config: ValidationTaskConfig,
    ) -> dict[str, float]:
        if head_name not in self.task_adapters:
            raise MissingHydraHeadError(head_name)

        adapter = self.task_adapters[head_name]
        validator_args = self._build_validator_args(
            adapter.task, adapter.head_model_name, config
        )

        validator: DetectionValidator | PoseValidator
        if adapter.task == "detect":
            validator = DetectionValidator(args=validator_args)
        elif adapter.task == "pose":
            validator = PoseValidator(args=validator_args)
        else:
            raise UnsupportedHydraHeadError(head_name)

        stats = validator(model=adapter)
        return cast(dict[str, float], stats)

    def validate(
        self,
        task_configs: Mapping[str, ValidationTaskConfig],
    ) -> dict[str, dict[str, float]]:
        results: dict[str, dict[str, float]] = {}

        for head_name in ("detection", "pose"):
            if head_name not in task_configs:
                continue
            logger.info("Validating task head: %s", head_name)
            results[head_name] = self.validate_task(
                head_name,
                task_configs[head_name],
            )

        for head_name, config in task_configs.items():
            if head_name in results:
                continue
            logger.info("Validating task head: %s", head_name)
            results[head_name] = self.validate_task(head_name, config)

        return results


def validate_original_models(
    task_dict: dict[str, Path],
    task_configs: dict[str, ValidationTaskConfig],
    project_dir: str | Path,
) -> dict[str, dict[str, float]]:
    """
    Validate original task models using standard YOLO().val() pipeline.

    Args:
        task_dict: Mapping of task names to model paths
            (e.g., {"detection": Path("yolo26m.pt")})
        task_configs: Validation configurations per task
        project_dir: Base project directory for results

    Returns:
        Dictionary mapping task names to their validation metrics

    Raises:
        Exception: If validation fails for any model (propagates from YOLO)
    """
    results: dict[str, dict[str, float]] = {}

    for task_name, model_path in task_dict.items():
        if task_name not in task_configs:
            logger.warning(
                "No validation config for task '%s', skipping", task_name
            )
            continue

        logger.info(
            "Validating original model for task '%s': %s", task_name, model_path
        )

        model = YOLO(model_path)
        model_name = model_path.stem
        config = task_configs[task_name]

        val_args = {
            "data": str(config.data),
            "split": config.split,
            "imgsz": config.imgsz,
            "batch": config.batch,
            "workers": config.workers,
            "conf": config.conf,
            "iou": config.iou,
            "max_det": config.max_det,
            "half": config.half,
            "plots": config.plots,
            "save_json": config.save_json,
            "save_txt": config.save_txt,
            "project": str(project_dir),
            "name": str(Path("val") / model_name),
            "exist_ok": config.exist_ok,
            "device": config.device,
        }

        metrics = model.val(**val_args)

        results[task_name] = dict(metrics.results_dict)
        logger.info(
            "%s original model metrics: %s", task_name, results[task_name]
        )

    return results


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run Ultralytics validation for Hydra heads"
    )
    parser.add_argument(
        "--foundation",
        type=Path,
        default="assets/yolo26m.pt",
        help="Path to the foundation checkpoint",
    )
    parser.add_argument(
        "--detection-model",
        type=Path,
        default="assets/yolo26m.pt",
        help="Path to the detection checkpoint",
    )
    parser.add_argument(
        "--pose-model",
        type=Path,
        default="assets/yolo26m-pose.pt",
        help="Path to the pose checkpoint",
    )
    parser.add_argument(
        "--detection-data",
        default="coco.yaml",
        help="Detection dataset YAML path",
    )
    parser.add_argument(
        "--pose-data",
        default="coco-pose.yaml",
        help="Pose dataset YAML path",
    )
    parser.add_argument(
        "--imgsz",
        type=int,
        default=640,
        help="Validation image size",
    )
    parser.add_argument(
        "--batch",
        type=int,
        default=16,
        help="Validation batch size",
    )
    parser.add_argument(
        "--device",
        default=None,
        help="Device to use, e.g. cpu, cuda, cuda:0",
    )
    parser.add_argument(
        "--validate-original",
        action="store_true",
        help="Validate original task models before multi-task validation",
    )
    args = parser.parse_args()

    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s | %(levelname)s | %(message)s",
    )

    tasks = {
        "detection": args.detection_model,
        "pose": args.pose_model,
    }

    hydra_model = Hydra(
        foundation_path=args.foundation,
        task_dict=tasks,
    )

    repo_root = os.path.abspath(".")
    project_dir = os.path.join(repo_root, "runs")

    task_configs: dict[str, ValidationTaskConfig] = {
        "detection": ValidationTaskConfig(
            data=args.detection_data,
            imgsz=args.imgsz,
            batch=args.batch,
            device=args.device,
            project=project_dir,
        ),
        "pose": ValidationTaskConfig(
            data=args.pose_data,
            imgsz=args.imgsz,
            batch=args.batch,
            device=args.device,
            project=project_dir,
        ),
    }

    # Validate original models if requested
    if args.validate_original:
        logger.info("Running validation on original task models")
        _original_metrics = validate_original_models(
            tasks, task_configs, project_dir
        )
        logger.info("Original model validation complete")

    validator = MultiTaskHydraValidator(hydra_model, device=args.device)
    metrics = validator.validate(task_configs)
    for head_name, stats in metrics.items():
        logger.info("%s metrics: %s", head_name, stats)


if __name__ == "__main__":
    main()
