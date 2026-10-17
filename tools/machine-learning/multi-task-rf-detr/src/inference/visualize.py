"""
Run the trained RF-DETR detector on images and save annotated overlays.

    uv run -m inference.visualize --config assets/detection.yaml
    uv run -m inference.visualize --config assets/detection.yaml --num 24 --threshold 0.4

Qualitative companion to the numeric metrics: lets you SEE detection quality per class
(e.g. Ball/Robot strong, L/T/X line-spots weak). Uses the same best-checkpoint fallback as
export (best_total -> best_ema -> best_regular). Overlays go to outputs/predictions/ (gitignored).
"""

import argparse
import sys
from pathlib import Path

from training.config import load_config


def find_checkpoint(cfg, override):
    if override:
        return override
    out = Path(cfg.training.output_dir)
    for name in (
        "checkpoint_best_total.pth",
        "checkpoint_best_ema.pth",
        "checkpoint_best_regular.pth",
    ):
        if (out / name).exists():
            return str(out / name)
    return None


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--config", default="assets/detection.yaml")
    ap.add_argument("--checkpoint", default=None)
    ap.add_argument("--images", default=None, help="image dir; default = <dataset_dir>/valid")
    ap.add_argument("--num", type=int, default=12)
    ap.add_argument("--threshold", type=float, default=0.5)
    ap.add_argument("--out", default="outputs/predictions")
    args = ap.parse_args()

    cfg = load_config(args.config)
    ckpt = find_checkpoint(cfg, args.checkpoint)
    if not ckpt:
        print(f"ERROR: no checkpoint found in {cfg.training.output_dir}")
        sys.exit(1)

    img_dir = Path(args.images) if args.images else Path(cfg.data.dataset_dir) / "valid"
    imgs = sorted(list(img_dir.glob("*.png")) + list(img_dir.glob("*.jpg")))[: args.num]
    if not imgs:
        print(f"ERROR: no images in {img_dir}")
        sys.exit(1)
    outdir = Path(args.out)
    outdir.mkdir(parents=True, exist_ok=True)

    import numpy as np
    import rfdetr
    import supervision as sv
    from PIL import Image

    print(f"Loading {cfg.model.variant} from {ckpt} ...")
    model = getattr(rfdetr, cfg.model.variant)(
        pretrain_weights=ckpt, resolution=cfg.training.resolution
    )
    names = cfg.model.class_names

    # supervision renamed the box annotator across versions — support both.
    box_cls = getattr(sv, "BoxAnnotator", None) or getattr(sv, "BoundingBoxAnnotator")
    box_annotator = box_cls()
    label_annotator = sv.LabelAnnotator()

    total = 0
    per_class: dict[str, int] = {}
    for p in imgs:
        im = Image.open(p).convert("RGB")
        det = model.predict(im, threshold=args.threshold)
        total += len(det)
        labels = []
        for cid, conf in zip(det.class_id, det.confidence):
            nm = names[int(cid)] if 0 <= int(cid) < len(names) else str(cid)
            labels.append(f"{nm} {conf:.2f}")
            per_class[nm] = per_class.get(nm, 0) + 1
        scene = np.array(im).copy()
        scene = box_annotator.annotate(scene=scene, detections=det)
        scene = label_annotator.annotate(scene=scene, detections=det, labels=labels)
        Image.fromarray(scene).save(outdir / f"pred_{p.stem}.png")
        print(f"  {p.name}: {len(det)} detections")

    print(f"\nSaved {len(imgs)} overlays to {outdir}")
    print(f"Total detections: {total} | per class: {dict(sorted(per_class.items()))}")


if __name__ == "__main__":
    main()
