import json
import logging
import os
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, cast

import click
from ultralytics.models.yolo.model import YOLO
from ultralytics.nn.tasks import DetectionModel
from ultralytics.utils.metrics import DetMetrics

from model.hydra import (
    get_backbone,
    set_backbone,
)
from utils.model_naming import (
    HYDRA_MODEL_NAME_TYPE,
    HydraModelName,
    TaskType,
)

logger = logging.getLogger(__name__)


class DatasetNotFoundError(Exception):
    def __init__(self, detection_type: TaskType) -> None:
        self.detection_type = detection_type
        super().__init__(f"No dataset specified for {detection_type}")


@dataclass(frozen=True)
class ValidationConfig:
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
    device: int | str | list = -1

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


def save_validation_results(
    save_dir: Path,
    metrics: dict[str, float],
    config: ValidationConfig,
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

    config_path = save_dir / "config.json"
    with open(config_path, "w") as f:
        json.dump(config.to_dict(), f, indent=2, default=str)
    logger.info("Saved config to %s", config_path)


def validate_hydra_model(
    hydra_model: HydraModelName, config: ValidationConfig, assets_dir: Path
) -> None:
    model_val_folder = Path("val") / str(hydra_model)
    validation_run_folder = Path(config.project) / model_val_folder

    backbone_model = cast(
        DetectionModel, YOLO(assets_dir / hydra_model.backbone.name).model
    )
    head_model_yolo_wrapper = YOLO(assets_dir / hydra_model.heads[0].name)
    head_model = cast(DetectionModel, head_model_yolo_wrapper.model)
    backbone = get_backbone(
        backbone_model, hydra_model.number_of_frozen_modules
    )
    set_backbone(head_model, backbone, hydra_model.number_of_frozen_modules)

    head_model_yolo_wrapper.eval()

    metrics = head_model_yolo_wrapper.val(
        **config.to_dict(name=model_val_folder)
    )
    metrics = cast(DetMetrics, metrics)

    save_validation_results(validation_run_folder, metrics.results_dict, config)

    head_model_yolo_wrapper.save(
        validation_run_folder / (str(hydra_model) + ".pt")
    )


@click.command()
@click.option(
    "--hydra_model_name",
    multiple=True,
    required=True,
    type=HYDRA_MODEL_NAME_TYPE,
    help=(
        "Hydra model name using the given naming convention. "
        "Example: --model_name yolo26m=f11+yolo26m-pose"
    ),
)
@click.option(
    "--object_dataset_name",
    default="coco.yaml",
    type=Path,
    help="Name of the object detection dataset. "
    "Is assumed to be relative to `./assets/datasets/`.",
)
@click.option(
    "--pose_dataset_name",
    type=Path,
    default="coco-pose.yaml",
    help="Name of the pose detection dataset. "
    "Is assumed to be relative to `./assets/datasets/`.",
)
@click.option(
    "--segmentation_dataset_name",
    type=Path,
    default="coco.yaml",
    help="Name of the segmentation detection dataset. "
    "Is assumed to be relative to `./assets/datasets/`.",
)
@click.option(
    "--assets_dir",
    type=Path,
    default=Path("assets"),
    help="Directory containing model assets.",
)
@click.option(
    "--runs_dir",
    type=Path,
    default=Path("runs"),
    help="Directory to save validation runs.",
)
@click.option(
    "--imgsz",
    type=int,
    default=640,
    show_default=True,
    help="Validation image size.",
)
@click.option(
    "--device",
    default="-1",
    type=str,
    show_default=True,
    help="Device to be used by ultralytics. Example: -1, cuda, cpu, or [1,2].",
)
@click.option(
    "--batch",
    default=16,
    show_default=True,
    help="Validation batch size",
)
def main(
    *,
    hydra_model_name: list[HydraModelName],
    object_dataset_name: Path,
    pose_dataset_name: Path,
    segmentation_dataset_name: Path,
    assets_dir: Path,
    runs_dir: Path,
    imgsz: int,
    device: str,
    batch: int,
) -> None:
    flattened_hydra_model_names = [
        HydraModelName(
            backbone=hydra_model_name.backbone,
            heads=[head],
            number_of_frozen_modules=hydra_model_name.number_of_frozen_modules,
        )
        for hydra_model_name in hydra_model_name
        for head in hydra_model_name.heads
    ]

    repo_root = os.path.abspath(".")
    project_dir = os.path.join(repo_root, runs_dir)

    for hydra_model in flattened_hydra_model_names:
        dataset_name = None
        match hydra_model.heads[0].task_type():
            case TaskType.OBJECT:
                dataset_name = object_dataset_name
            case TaskType.POSE:
                dataset_name = pose_dataset_name
            case TaskType.SEGMENTATION:
                dataset_name = segmentation_dataset_name
        if dataset_name is None:
            raise DatasetNotFoundError(hydra_model.heads[0].task_type())
        data = assets_dir / "datasets" / dataset_name
        config = ValidationConfig(
            data=data,
            project=project_dir,
            imgsz=imgsz,
            batch=batch,
            device=device,
        )

        print(config)

        validate_hydra_model(hydra_model, config=config, assets_dir=assets_dir)


if __name__ == "__main__":
    main()
