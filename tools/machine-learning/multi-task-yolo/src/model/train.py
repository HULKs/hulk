import os
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import click
import wandb
import yaml
from ultralytics.models.yolo.model import YOLO
from wonderwords import RandomWord

from utils.model_naming import (
    HYDRA_MODEL_NAME_TYPE,
    HydraModelName,
    TaskType,
)
from validation.validator import DatasetNotFoundError

DEVICE_FORMAT_ERROR = "must be a comma-separated list of integers, e.g. 0,1"
DEVICE_EMPTY_ERROR = "must contain at least one device index, e.g. 0"


@dataclass(frozen=True)
class TrainingConfig:
    name: str | Path
    data: str | Path
    project: str | Path = "runs"
    epochs: int = 100
    imgsz: int = 640
    batch: int = 32
    optimizer: str = "auto"
    freeze: int = 11
    plots: bool = True
    exist_ok: bool = True
    val: bool = True
    save: bool = True
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


def do_hyperparameter_tuning(config: TrainingConfig, model_path: Path) -> Path:
    # Define search space
    search_space = {
        "lr0": (1e-5, 1e-1),
        "lrf": (0.01, 1.0),
        "momentum": (0.6, 0.98),
        "weight_decay": (0.0, 0.001),
        "warmup_epochs": (0.0, 5.0),
        "warmup_momentum": (0.0, 0.95),
        "box": (0.02, 0.2),
        "cls": (0.2, 4.0),
        "dfl": (0.4, 6.0),
        "hsv_h": (0.0, 0.1),
        "hsv_s": (0.0, 0.9),
        "hsv_v": (0.0, 0.9),
        "degrees": (0.0, 30.0),
        "translate": (0.0, 0.9),
        "scale": (0.0, 0.9),
        "shear": (0.0, 10.0),
        "perspective": (0.0, 0.001),
        # "flipup": (0.0, 1.0),
        "fliplr": (0.0, 1.0),
        # "bgr": (0.0, 1.0),
        "mosaic": (0.0, 1.0),
        "mixup": (0.0, 1.0),
        "copy_paste": (0.0, 1.0),
        "close_mosaic": (0, 10),
    }

    yolo_model_wrapper = YOLO(model_path)

    yolo_model_wrapper.tune(
        optimizer="AdamW",
        space=search_space,
        **config.to_dict(),
    )

    return model_path.parent / "best_hyperparameters.yaml"


@click.command(
    context_settings={"help_option_names": ["-h", "--help"]},
    help="Finetune models based on a hydra model name.",
)
@click.option(
    "--hydra_model_name",
    multiple=True,
    required=True,
    type=HYDRA_MODEL_NAME_TYPE,
    help=(
        "Hydra model name using the given naming convention. "
        "Example: yolo26m=f11+yolo26m-pose"
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
    help="Directory to save training runs.",
)
@click.option(
    "--val_dir",
    type=Path,
    default=Path("val"),
    help="Directory to save validation runs.",
)
@click.option(
    "--device",
    default="-1",
    type=str,
    show_default=True,
    help="Device to be used by ultralytics. Example: -1, cuda, cpu, or [1,2].",
)
@click.option(
    "--do-tuning",
    is_flag=True,
    default=False,
    show_default=True,
    help="Run tune() before train().",
)
@click.option(
    "--use-tuned-hyperparameters/--no-use-tuned-hyperparameters",
    default=False,
    show_default=True,
    help="Load best_hyperparameters.yaml and pass to train().",
)
def main(
    *,
    hydra_model_name: list[HydraModelName],
    object_dataset_name: Path,
    pose_dataset_name: Path,
    segmentation_dataset_name: Path,
    assets_dir: Path,
    runs_dir: Path,
    val_dir: Path,
    device: str,
    do_tuning: bool,
    use_tuned_hyperparameters: bool,
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
    runs_dir = repo_root / runs_dir
    val_path = runs_dir / val_dir

    for hydra_model in flattened_hydra_model_names:
        model_path = val_path / str(hydra_model) / (str(hydra_model) + ".pt")

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

        best_params = {}
        tuned_hyperparameters_path = None

        if do_tuning:
            run_name = (
                str(hydra_model)
                + "~"
                + RandomWord().word(
                    word_min_length=4,
                    word_max_length=8,
                    include_categories=["nouns"],
                )
            )
            wandb.init(project="multi-task-yolo", name=run_name)

            config = TrainingConfig(
                data=data,
                project=runs_dir,
                name=Path("tune") / run_name,
                epochs=40,
                optimizer="AdamW",
                freeze=hydra_model.number_of_frozen_modules,
                device=device,
            )
            tuned_hyperparameters_path = do_hyperparameter_tuning(
                config, model_path
            )

            print(
                f"Loading best hyperparameters from: "
                f"{tuned_hyperparameters_path}"
            )
            with open(tuned_hyperparameters_path) as f:
                best_params = yaml.safe_load(f)
            print("Loaded params:", best_params)

        run_name = (
            str(hydra_model)
            + "~"
            + RandomWord().word(
                word_min_length=4,
                word_max_length=8,
                include_categories=["nouns"],
            )
        )
        wandb.init(project="multi-task-yolo", name=run_name)

        config = TrainingConfig(
            data=data,
            project=runs_dir,
            name=Path("train") / run_name,
            epochs=70,
            freeze=hydra_model.number_of_frozen_modules,
            device=device,
        )
        config_dict = config.to_dict()

        if use_tuned_hyperparameters:
            config_dict.update(best_params)

        yolo_model_wrapper = YOLO(model_path)

        yolo_model_wrapper.train(**config_dict)


if __name__ == "__main__":
    main()
