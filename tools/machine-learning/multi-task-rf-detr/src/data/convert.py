"""
Convert nao_dataset (YOLO detection format) -> RF-DETR COCO layout.

Input  (Ultralytics YOLO):
    data/nao_dataset/
        images/train/*.png   labels/train/*.txt   (YOLO: class cx cy w h, normalized)
        images/val/*.png     labels/val/*.txt
        (class names from data/nao_data.yaml)

Output (RF-DETR auto-detected COCO):
    data/processed/coco_format/
        train/_annotations.coco.json + image hardlinks
        valid/_annotations.coco.json + image hardlinks   (YOLO "val" -> RF-DETR "valid")

COCO conventions mirror the proven Roboflow export from rtdetr-pcb-project:
  - categories are 0-indexed (id == YOLO class_id, no remap, no off-by-one)
  - bbox in absolute pixels [x_min, y_min, w, h]; area in px^2
  - empty-label images are KEPT as background examples (image entry, no annotations)

Usage:
    python src/convert_nao_to_rfdetr.py                 # full conversion
    python src/convert_nao_to_rfdetr.py --limit 50      # quick dry-run (50 imgs/split)
    python src/convert_nao_to_rfdetr.py --copy          # force copy instead of hardlink
"""
import argparse
import json
import os
import shutil
from pathlib import Path

import yaml
from PIL import Image

IMG_EXTS = {".png", ".jpg", ".jpeg", ".bmp"}


def load_class_names(yaml_path: Path) -> list:
    data = yaml.safe_load(yaml_path.read_text(encoding="utf-8"))
    names = data["names"]
    if isinstance(names, dict):  # {0: 'Ball', 1: 'GoalPost', ...}
        return [names[i] for i in sorted(names.keys())]
    return list(names)


def place_image(src: Path, dst: Path, use_copy: bool) -> None:
    if dst.exists():
        return
    if use_copy:
        shutil.copy2(src, dst)
        return
    try:
        os.link(src, dst)            # hardlink — NTFS-friendly, zero extra disk
    except OSError:
        shutil.copy2(src, dst)       # fallback (cross-volume, permissions, etc.)


def convert_split(img_dir: Path, lbl_dir: Path, out_dir: Path, names: list,
                  split_label: str, limit: int, use_copy: bool) -> dict:
    out_dir.mkdir(parents=True, exist_ok=True)
    images, annotations = [], []
    ann_id = 0
    n_background = 0
    n_clamped = 0

    img_files = sorted(p for p in img_dir.iterdir() if p.suffix.lower() in IMG_EXTS)
    if limit:
        img_files = img_files[:limit]

    for img_id, img_path in enumerate(img_files):
        with Image.open(img_path) as im:
            w, h = im.size
        images.append({
            "id": img_id, "license": 1, "file_name": img_path.name,
            "height": h, "width": w,
        })
        place_image(img_path, out_dir / img_path.name, use_copy)

        lbl_path = lbl_dir / (img_path.stem + ".txt")
        if not lbl_path.exists() or lbl_path.stat().st_size == 0:
            n_background += 1
            continue

        for line in lbl_path.read_text(encoding="utf-8").strip().splitlines():
            parts = line.split()
            if len(parts) != 5:
                continue
            cls = int(parts[0])
            cx, cy, bw, bh = map(float, parts[1:])
            # YOLO normalized center-form -> COCO absolute corner-form
            x = (cx - bw / 2.0) * w
            y = (cy - bh / 2.0) * h
            aw, ah = bw * w, bh * h
            # Defensive clamp to image bounds
            x0, y0 = max(0.0, x), max(0.0, y)
            if x0 != x or y0 != y:
                n_clamped += 1
            aw = min(aw, w - x0)
            ah = min(ah, h - y0)
            if aw <= 0 or ah <= 0:
                continue
            annotations.append({
                "id": ann_id, "image_id": img_id, "category_id": cls,
                "bbox": [round(x0, 2), round(y0, 2), round(aw, 2), round(ah, 2)],
                "area": round(aw * ah, 2), "segmentation": [], "iscrowd": 0,
            })
            ann_id += 1

    categories = [{"id": i, "name": n, "supercategory": "robocup"} for i, n in enumerate(names)]
    coco = {
        "info": {"description": f"NAO RoboCup dataset — {split_label}", "version": "1.0"},
        "licenses": [{"id": 1, "name": "unknown", "url": ""}],
        "images": images,
        "annotations": annotations,
        "categories": categories,
    }
    (out_dir / "_annotations.coco.json").write_text(
        json.dumps(coco, ensure_ascii=False), encoding="utf-8"
    )
    return {
        "split": split_label, "images": len(images), "annotations": len(annotations),
        "background": n_background, "clamped": n_clamped,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--src", default="data/nao_dataset", help="YOLO dataset root")
    parser.add_argument("--names", default="data/nao_data.yaml", help="class names yaml")
    parser.add_argument("--out", default="data/processed/coco_format", help="COCO output root")
    parser.add_argument("--limit", type=int, default=0, help="max images per split (0=all)")
    parser.add_argument("--copy", action="store_true", help="copy images instead of hardlink")
    args = parser.parse_args()

    src = Path(args.src)
    names = load_class_names(Path(args.names))
    print(f"Classes ({len(names)}): {names}")

    # YOLO split name -> RF-DETR split dir name
    split_map = {"train": "train", "val": "valid"}
    results = []
    for yolo_split, rfdetr_split in split_map.items():
        img_dir = src / "images" / yolo_split
        lbl_dir = src / "labels" / yolo_split
        if not img_dir.exists():
            print(f"SKIP {yolo_split}: {img_dir} not found")
            continue
        print(f"\nConverting {yolo_split} -> {rfdetr_split} ...")
        stats = convert_split(
            img_dir, lbl_dir, Path(args.out) / rfdetr_split,
            names, rfdetr_split, args.limit, args.copy,
        )
        results.append(stats)
        print(f"  images={stats['images']} annotations={stats['annotations']} "
              f"background={stats['background']} clamped={stats['clamped']}")

    print("\n" + "=" * 60)
    print("Conversion complete.")
    for r in results:
        print(f"  {r['split']:6s}: {r['images']:6d} imgs, {r['annotations']:7d} anns, "
              f"{r['background']:5d} background")
    print(f"Output: {Path(args.out).resolve()}")
    print("=" * 60)


if __name__ == "__main__":
    main()
