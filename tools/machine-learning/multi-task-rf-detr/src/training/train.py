"""Phase 1 - RF-DETR detection training on the converted NAO COCO dataset.

    uv run -m training.train --config assets/detection.yaml
    uv run -m training.train --config assets/detection.yaml --dry-run   # no GPU/rfdetr needed

num_classes is auto-detected by rfdetr from the COCO categories.
"""

import argparse
import sys
from pathlib import Path

from training.config import load_config, print_config


def build_train_kwargs(config) -> dict:
    """Map our YAML config onto rfdetr's model.train(...) kwargs.

    Single place that touches the rfdetr training signature; all names are
    verified against rfdetr 1.7.1's TrainConfig.
    """
    t = config.training
    return {
        "dataset_dir": config.data.dataset_dir,
        "epochs": t.epochs,
        "batch_size": t.batch_size,
        "grad_accum_steps": t.grad_accum_steps,
        "lr": t.lr,
        "weight_decay": t.weight_decay,
        "resolution": t.resolution,
        "output_dir": t.output_dir,
        "early_stopping": True,
        "early_stopping_patience": t.early_stopping_patience,
        "early_stopping_min_delta": t.early_stopping_min_delta,
        "tensorboard": t.tensorboard,
        "num_workers": t.dataloader_num_workers,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", default="assets/detection.yaml")
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print resolved config + kwargs, then exit (no rfdetr/GPU).",
    )
    parser.add_argument("--resume", default=None, help="Checkpoint path to resume from.")
    args = parser.parse_args()

    config = load_config(args.config)
    print_config(config)

    ds = Path(config.data.dataset_dir)
    for split in ("train", "valid"):
        ann = ds / split / "_annotations.coco.json"
        if not ann.exists():
            print(f"ERROR: missing {ann}. Run `uv run -m data.convert` first.")
            sys.exit(1)

    train_kwargs = build_train_kwargs(config)
    if args.resume:
        train_kwargs["resume"] = args.resume

    print("\nResolved rfdetr train() kwargs:")
    for k, v in train_kwargs.items():
        print(f"  {k:26s} = {v}")

    if args.dry_run:
        print("\n[dry-run] Not importing rfdetr / not training.")
        return

    import rfdetr
    import torch

    if not torch.cuda.is_available():
        print("WARNING: CUDA not available - training will be very slow on CPU.")

    print(f"\nInstantiating {config.model.variant} (downloads weights on first run)...")
    model = getattr(rfdetr, config.model.variant)()

    print("Starting training...\n")
    model.train(**train_kwargs)
    print(f"\nDone. Checkpoints in {config.training.output_dir}")
    print(f"TensorBoard: tensorboard --logdir {config.training.output_dir}")


if __name__ == "__main__":
    main()
