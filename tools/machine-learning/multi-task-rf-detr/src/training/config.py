"""
YAML configuration loader for RF-DETR fine-tuning.

Adapted from rtdetr-pcb-project/src/config_loader.py — restructured around
the rfdetr API (model.train(...) kwargs) instead of HuggingFace Trainer args.

Save as: src/config_loader.py
"""

import sys
import yaml
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class ModelConfig:
    variant: str = "RFDETRSmall"  # rfdetr.RFDETRSmall / RFDETRSegSmall / etc.
    pretrained: bool = True
    num_classes: int = 7
    class_names: list[str] = field(default_factory=list)
    warm_start_backbone: str | None = None
    # Pose-only fields (ignored for det/seg)
    num_keypoints: int | None = None
    keypoint_names: list[str] | None = None


@dataclass
class DataConfig:
    dataset_dir: str = "./data/processed/coco_format"


@dataclass
class TrainingConfig:
    output_dir: str = "./models/rf-detr-small-det"
    epochs: int = 50
    batch_size: int = 8
    grad_accum_steps: int = 2
    lr: float = 1e-4
    weight_decay: float = 1e-4
    resolution: int = 448
    early_stopping_patience: int = 10
    early_stopping_min_delta: float = 1e-3
    fp16: bool = True
    dataloader_num_workers: int = 0  # Windows: must be 0
    tensorboard: bool = True
    logging_dir: str = "./outputs/logs"
    save_total_limit: int = 3


@dataclass
class AugmentationConfig:
    enabled: bool = True
    horizontal_flip_p: float = 0.5
    rotate_limit: int = 10
    brightness_limit: float = 0.2
    contrast_limit: float = 0.2
    motion_blur_p: float = 0.2
    gaussian_noise_p: float = 0.2


@dataclass
class ExportConfig:
    output_path: str = "./exports/rf-detr-det-448.onnx"
    simplify: bool = True
    static_batch: bool = True
    parity_tolerance: float = 1e-3


@dataclass
class Config:
    model: ModelConfig = field(default_factory=ModelConfig)
    data: DataConfig = field(default_factory=DataConfig)
    training: TrainingConfig = field(default_factory=TrainingConfig)
    augmentation: AugmentationConfig = field(default_factory=AugmentationConfig)
    export: ExportConfig = field(default_factory=ExportConfig)


def load_config(config_path: str) -> Config:
    path = Path(config_path)
    if not path.exists():
        raise FileNotFoundError(f"Config not found: {path}")

    with open(path, encoding="utf-8") as f:
        raw = yaml.safe_load(f)

    config = Config()
    if "model" in raw:
        config.model = ModelConfig(**raw["model"])
    if "data" in raw:
        config.data = DataConfig(**raw["data"])
    if "training" in raw:
        td = raw["training"]
        if sys.platform == "win32" and td.get("dataloader_num_workers", 0) > 0:
            print("Windows detected: forcing dataloader_num_workers=0")
            td["dataloader_num_workers"] = 0
        config.training = TrainingConfig(**td)
    if "augmentation" in raw:
        config.augmentation = AugmentationConfig(**raw["augmentation"])
    if "export" in raw:
        config.export = ExportConfig(**raw["export"])

    _validate(config)
    return config


def _validate(config: Config) -> None:
    if config.training.resolution % 56 != 0:
        raise ValueError(
            f"resolution={config.training.resolution} must be divisible by 56 "
            f"(rfdetr DINOv2 patch-size constraint)"
        )
    if len(config.model.class_names) != config.model.num_classes:
        raise ValueError(
            f"num_classes={config.model.num_classes} but class_names has "
            f"{len(config.model.class_names)} entries"
        )


def print_config(config: Config) -> None:
    print("=" * 60)
    print("RF-DETR Configuration")
    print("=" * 60)
    print(f"Variant:        {config.model.variant}")
    print(f"Classes:        {config.model.num_classes} -> {config.model.class_names}")
    print(f"Resolution:     {config.training.resolution}")
    print(
        f"Effective batch:{config.training.batch_size * config.training.grad_accum_steps} "
        f"({config.training.batch_size} x {config.training.grad_accum_steps})"
    )
    print(f"LR:             {config.training.lr}")
    print(f"Epochs:         {config.training.epochs}")
    print(f"FP16:           {config.training.fp16}")
    print(f"Workers:        {config.training.dataloader_num_workers}")
    print(f"Dataset dir:    {config.data.dataset_dir}")
    print(f"Output dir:     {config.training.output_dir}")
    print(f"Export path:    {config.export.output_path}")
    print("=" * 60)


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("--config", default="config/detection.yaml")
    args = parser.parse_args()

    config = load_config(args.config)
    print_config(config)
