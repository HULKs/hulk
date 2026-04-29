import argparse
import logging
from pathlib import Path
from typing import cast

from ultralytics.models.yolo.model import YOLO
from ultralytics.nn.tasks import DetectionModel

from model.hydra import get_backbone, set_backbone


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run Ultralytics validation for Hydra heads"
    )
    parser.add_argument(
        "--backbone",
        type=Path,
        default="assets/yolo26m.pt",
        help="Path to the model used for the backbone",
    )
    parser.add_argument(
        "--head",
        type=Path,
        default="assets/yolo26m-pose.pt",
        help="Path to the model checkpoint used for the head",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default="assets/yolo26m-pose.pt",
        help="Path where the combined model should be saved",
    )
    args = parser.parse_args()

    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s | %(levelname)s | %(message)s",
    )

    backbone_model = cast(DetectionModel, YOLO(args.backbone).model)
    head_model = YOLO(args.head)
    backbone = get_backbone(backbone_model)
    set_backbone(head_model.model, backbone)

    head_model.save(args.output)


if __name__ == "__main__":
    main()
