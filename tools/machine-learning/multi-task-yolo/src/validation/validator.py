import argparse
import json
import logging
import os
from collections.abc import Mapping
from dataclasses import asdict, dataclass, replace
from datetime import datetime
from pathlib import Path
from typing import Any, Literal, cast

import torch
from ultralytics.models.yolo.detect.val import DetectionValidator
from ultralytics.models.yolo.model import YOLO
from ultralytics.models.yolo.pose.val import PoseValidator

from model.hydra import (
    ClassNames,
    Hydra,
    HydraTaskModelAdapter,
    MissingHydraHeadError,
    UnsupportedHydraHeadError,
)

logger = logging.getLogger(__name__)

ValidationType = Literal["original", "multi_task"]


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
    save_txt: bool = False
    project: str | Path = "runs"
    exist_ok: bool = True
    device: str | None = None

    def to_dict(self, **overrides: Any) -> dict[str, Any]:
        """
        Convert config to dictionary for serialization.

        Args:
            **overrides: Optional field overrides to apply to the result

        Returns:
            Dictionary with config values and any specified overrides
        """
        result = asdict(self)
        # Convert Path objects to strings
        result["data"] = str(result["data"])
        result["project"] = str(result["project"])
        # Apply any overrides
        result.update(overrides)
        return result


@dataclass
class ValidationMetadata:
    """Metadata about a validation run."""

    timestamp: str
    validation_type: ValidationType
    task: str
    dataset: str
    split: str
    model_path: str | None = None
    model_name: str | None = None
    head_name: str | None = None
    foundation_name: str | None = None


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

    def validate_task(
        self,
        head_name: str,
        config: ValidationTaskConfig,
    ) -> dict[str, float]:
        if head_name not in self.task_adapters:
            raise MissingHydraHeadError(head_name)

        adapter = self.task_adapters[head_name]
        validator_args = config.to_dict(
            task=adapter.task,
            name=Path("val")
            / (self.model.foundation_name + "_" + adapter.head_model_name),
            device=config.device or str(self.device),
        )

        validator: DetectionValidator | PoseValidator
        if adapter.task == "detect":
            validator = DetectionValidator(args=validator_args)
        elif adapter.task == "pose":
            validator = PoseValidator(args=validator_args)
        else:
            raise UnsupportedHydraHeadError(head_name)

        stats = validator(model=adapter)

        save_dir = validator.save_dir
        metadata = ValidationMetadata(
            timestamp=datetime.now().isoformat(),
            validation_type="multi_task",
            task=adapter.task,
            dataset=str(config.data),
            split=config.split,
            model_name=adapter.head_model_name,
            head_name=head_name,
            foundation_name=self.model.foundation_name,
        )
        effective_config = replace(
            config, device=config.device or str(self.device)
        )
        save_validation_results(save_dir, stats, metadata, effective_config)

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


def save_validation_results(
    save_dir: Path,
    metrics: dict[str, float],
    metadata: ValidationMetadata,
    config: ValidationTaskConfig,
) -> None:
    """
    Save validation results to JSON files in the specified directory.

    Args:
        save_dir: Directory to save the results
        metrics: Validation metrics dictionary
        metadata: Metadata about the validation run
        config: Validation configuration
    """
    save_dir = Path(save_dir)
    save_dir.mkdir(parents=True, exist_ok=True)

    metrics_path = save_dir / "metrics.json"
    with open(metrics_path, "w") as f:
        json.dump(metrics, f, indent=2)
    logger.info("Saved metrics to %s", metrics_path)

    metadata_path = save_dir / "metadata.json"
    with open(metadata_path, "w") as f:
        json.dump(asdict(metadata), f, indent=2, default=str)
    logger.info("Saved metadata to %s", metadata_path)

    config_path = save_dir / "config.json"
    with open(config_path, "w") as f:
        json.dump(config.to_dict(), f, indent=2, default=str)
    logger.info("Saved config to %s", config_path)


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
        val_args = config.to_dict(
            project=str(project_dir),
            name=str(Path("val") / model_name),
        )

        # Run validation (will raise exception on failure)
        metrics = model.val(**val_args)

        # Extract metrics dictionary using .results_dict attribute
        results[task_name] = dict(metrics.results_dict)
        logger.info(
            "%s original model metrics: %s", task_name, results[task_name]
        )

        # Save results to JSON files
        save_dir = Path(project_dir) / "val" / model_name
        metadata = ValidationMetadata(
            timestamp=datetime.now().isoformat(),
            validation_type="original",
            task=task_name,
            dataset=str(config.data),
            split=config.split,
            model_path=str(model_path),
            model_name=model_name,
        )
        save_validation_results(save_dir, results[task_name], metadata, config)

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
