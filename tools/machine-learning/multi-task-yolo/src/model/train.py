from pathlib import Path

import click
import yaml
from ultralytics.models.yolo.model import YOLO

DEVICE_FORMAT_ERROR = "must be a comma-separated list of integers, e.g. 0,1"
DEVICE_EMPTY_ERROR = "must contain at least one device index, e.g. 0"


@click.command(
    context_settings={"help_option_names": ["-h", "--help"]},
    help="Finetuning a single task model.",
)
@click.option(
    "--repo-root",
    type=click.Path(path_type=Path),
    default=Path.cwd().resolve(),
    show_default=True,
    help="Repository root used to derive default runs paths.",
)
@click.option(
    "--project-tune-dir",
    type=click.Path(path_type=Path),
    default=None,
    help="Tune project directory. Defaults to <repo-root>/runs/tune.",
)
@click.option(
    "--project-train-dir",
    type=click.Path(path_type=Path),
    default=None,
    help="Train project directory. Defaults to <repo-root>/runs/train.",
)
@click.option(
    "--tuning-folder-name",
    default="yolo26m-tuning",
    show_default=True,
    help="Subfolder name for tuning outputs.",
)
@click.option(
    "--training-folder-name",
    default="yolo26m-tuned",
    show_default=True,
    help="Subfolder name for training outputs.",
)
@click.option(
    "--base-model-path",
    default="assets/yolo26m.pt",
    show_default=True,
    help="Base model checkpoint path.",
)
@click.option(
    "--data",
    "data_path",
    default="/opt/data/nao_coco_k1_data.yaml",
    show_default=True,
    help="Dataset yaml path.",
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
@click.option(
    "--dev-mode",
    is_flag=True,
    default=False,
    show_default=True,
    help="Use fast development train settings.",
)
@click.option(
    "--device",
    type=str,
    default="1",
    show_default=True,
    help="GPU device index(es) as comma-separated list, e.g. 0,1.",
)
def main(
    *,
    repo_root: Path,
    project_tune_dir: Path | None,
    project_train_dir: Path | None,
    tuning_folder_name: str,
    training_folder_name: str,
    base_model_path: str,
    data_path: str,
    do_tuning: bool,
    use_tuned_hyperparameters: bool,
    dev_mode: bool,
    device: str,
) -> None:
    repo_root = repo_root.resolve()
    project_tune_dir = (
        project_tune_dir.resolve()
        if project_tune_dir
        else (repo_root / "runs" / "tune")
    )
    project_train_dir = (
        project_train_dir.resolve()
        if project_train_dir
        else (repo_root / "runs" / "train")
    )

    try:
        devices = [int(d.strip()) for d in device.split(",") if d.strip()]
    except ValueError as exc:
        raise click.BadParameter(DEVICE_FORMAT_ERROR) from exc
    if not devices:
        raise click.BadParameter(DEVICE_EMPTY_ERROR)

    model = YOLO(base_model_path)
    best_params = {}

    if do_tuning:
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

        model.tune(
            iterations=50,
            data=data_path,
            project=project_tune_dir,
            name=tuning_folder_name,
            exist_ok=True,
            epochs=40,
            device=devices,
            optimizer="AdamW",
            space=search_space,
            plots=True,
            save=True,
            val=True,
            freeze=11,
        )
        use_tuned_hyperparameters = True

    if use_tuned_hyperparameters:
        yaml_path = (
            project_tune_dir / tuning_folder_name / "best_hyperparameters.yaml"
        )
        print(f"Loading best hyperparameters from: {yaml_path}")
        with open(yaml_path) as f:
            best_params = yaml.safe_load(f)
        print("Loaded params:", best_params)

    train_args = {
        "data": data_path,
        "project": project_train_dir,
        "name": training_folder_name,
        "exist_ok": True,
        "epochs": 70,
        "imgsz": 640,
        "device": devices,
        "batch": 32,
        "fraction": 1.0,
        "workers": 8,
        "freeze": 11,
        # resume=True,
    }

    if dev_mode:
        train_args.update(
            {
                "name": f"{training_folder_name}-dev",
                "epochs": 5,
                "imgsz": 512,
                "batch": 8,
                "fraction": 0.1,
                "workers": 4,
            }
        )

    train_args.update(best_params)

    model.train(**train_args)


if __name__ == "__main__":
    main()
