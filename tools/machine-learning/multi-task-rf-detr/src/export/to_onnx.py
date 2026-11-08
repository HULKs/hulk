"""Phase 4 - export a trained RF-DETR checkpoint to ONNX for robot-side hand-off.

    uv run -m export.to_onnx --config assets/detection.yaml

Static batch (batch_size=1, dynamic_batch=False) avoids the known DETR/TensorRT
dynamic-shape failures; the robot only ever infers single frames.
"""

import argparse
import shutil
import sys
from pathlib import Path

from training.config import load_config


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--config", default="assets/detection.yaml")
    ap.add_argument(
        "--checkpoint",
        default=None,
        help="default: best_total, else best_ema, else best_regular in output_dir",
    )
    args = ap.parse_args()

    cfg = load_config(args.config)
    if args.checkpoint:
        ckpt = args.checkpoint
    else:
        # Prefer best_total (written at a normal end-of-training); fall back to best_ema /
        # best_regular, which are saved live and survive an interrupted/early-stopped run.
        out_dir = Path(cfg.training.output_dir)
        candidates = [
            "checkpoint_best_total.pth",
            "checkpoint_best_ema.pth",
            "checkpoint_best_regular.pth",
        ]
        ckpt = next((str(out_dir / c) for c in candidates if (out_dir / c).exists()), None)
        if ckpt is None:
            print(f"ERROR: no best checkpoint in {out_dir} (looked for {candidates})")
            sys.exit(1)
    if not Path(ckpt).exists():
        print(f"ERROR: checkpoint not found: {ckpt}")
        sys.exit(1)
    print(f"Using checkpoint: {ckpt}")

    out_path = Path(cfg.export.output_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    stage_dir = out_path.parent / "_export_stage"

    print(
        f"Variant: {cfg.model.variant} | ckpt: {ckpt} | res: {cfg.training.resolution} "
        f"| static_batch: {cfg.export.static_batch} | target: {out_path}"
    )

    import rfdetr

    model = getattr(rfdetr, cfg.model.variant)(
        pretrain_weights=ckpt,
        resolution=cfg.training.resolution,
    )
    produced = Path(
        model.export(
            output_dir=str(stage_dir),
            simplify=cfg.export.simplify,
            batch_size=1,
            dynamic_batch=not cfg.export.static_batch,
            opset_version=17,
        )
    )

    if out_path.exists():
        out_path.unlink()
    shutil.move(str(produced), str(out_path))
    shutil.rmtree(stage_dir, ignore_errors=True)

    import onnx

    onnx.checker.check_model(str(out_path))
    print(f"\nExported + structurally valid: {out_path} ({out_path.stat().st_size / 1e6:.1f} MB)")
    print("Next: uv run -m export.validate --config", args.config)


if __name__ == "__main__":
    main()
