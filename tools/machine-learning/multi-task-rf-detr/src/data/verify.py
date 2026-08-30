"""Verify the converted RF-DETR COCO dataset.

uv run -m data.verify --config assets/detection.yaml
uv run -m data.verify --config assets/segmentation.yaml --check-masks
"""

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

from training.config import Config, load_config


def verify_split(
    split_dir: Path,
    split_name: str,
    expected_class_names: list,
    check_masks: bool = False,
    check_keypoints: bool = False,
):
    print(f"\n[{split_name}] {split_dir}")
    stats = {"valid": True, "warnings": []}

    ann_path = split_dir / "_annotations.coco.json"
    if not split_dir.exists() or not ann_path.exists():
        print("  ERROR: split dir or _annotations.coco.json missing")
        return False, stats
    try:
        coco = json.loads(ann_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        print(f"  ERROR: invalid JSON ({exc})")
        return False, stats

    for key in ("images", "annotations", "categories"):
        if key not in coco:
            print(f"  ERROR: missing key {key}")
            return False, stats

    images, annotations, categories = coco["images"], coco["annotations"], coco["categories"]
    coco_names = [c["name"] for c in sorted(categories, key=lambda x: x["id"])]
    if expected_class_names and coco_names != expected_class_names:
        print(f"  WARN: class mismatch: COCO {coco_names} vs config {expected_class_names}")
    else:
        print(f"  OK: {len(categories)} classes match config")

    missing = [i["file_name"] for i in images if not (split_dir / i["file_name"]).exists()]
    print(
        f"  {'WARN' if missing else 'OK'}: {len(missing)} missing image files"
        if missing
        else f"  OK: {len(images)} images exist"
    )

    per_cat, bad_boxes, bad_masks, bad_kpts = Counter(), 0, 0, 0
    for ann in annotations:
        per_cat[ann.get("category_id")] += 1
        bbox = ann.get("bbox", [])
        if len(bbox) != 4 or bbox[2] <= 0 or bbox[3] <= 0 or bbox[0] < 0 or bbox[1] < 0:
            bad_boxes += 1
        if check_masks and not ann.get("segmentation"):
            bad_masks += 1
        if check_keypoints and len(ann.get("keypoints", [])) % 3 != 0:
            bad_kpts += 1

    if bad_boxes:
        print(f"  WARN: {bad_boxes} invalid bboxes")
    if check_masks and bad_masks:
        print(f"  WARN: {bad_masks} annotations missing segmentation")
    if check_keypoints and bad_kpts:
        print(f"  WARN: {bad_kpts} annotations with malformed keypoints")

    id_to_name = {c["id"]: c["name"] for c in categories}
    print("  Class distribution:")
    for cat_id, count in sorted(per_cat.items()):
        print(f"    {cat_id}: {id_to_name.get(cat_id, '?')} = {count}")
    return True, stats


def verify_dataset(
    config: Config, check_masks: bool = False, check_keypoints: bool = False
) -> bool:
    print("=" * 60)
    print("RF-DETR Dataset Verification")
    print("=" * 60)
    dataset_dir = Path(config.data.dataset_dir)
    if not dataset_dir.exists():
        print(f"FATAL: dataset_dir not found: {dataset_dir}")
        return False
    print(f"Expected classes: {config.model.class_names}")

    all_valid = True
    for split in ("train", "valid"):
        ok, _ = verify_split(
            dataset_dir / split,
            split,
            config.model.class_names,
            check_masks=check_masks,
            check_keypoints=check_keypoints,
        )
        all_valid = all_valid and ok
    if (dataset_dir / "test").exists():
        verify_split(
            dataset_dir / "test",
            "test",
            config.model.class_names,
            check_masks=check_masks,
            check_keypoints=check_keypoints,
        )

    print("\n" + "=" * 60)
    print("PASSED" if all_valid else "FAILED")
    print("=" * 60)
    return all_valid


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", default="assets/detection.yaml")
    parser.add_argument("--check-masks", action="store_true")
    parser.add_argument("--check-keypoints", action="store_true")
    args = parser.parse_args()
    config = load_config(args.config)
    ok = verify_dataset(config, check_masks=args.check_masks, check_keypoints=args.check_keypoints)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
