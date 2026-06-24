from __future__ import annotations

import json
import logging
from collections.abc import Mapping, Sequence
from dataclasses import asdict, dataclass, replace
from pathlib import Path
from typing import Any, cast

import click
import cv2
import numpy as np
from ultralytics.data.utils import (
    IMG_FORMATS,
    check_det_dataset,
    img2label_paths,
    polygon2mask,
)
from ultralytics.models.yolo.model import YOLO
from ultralytics.nn.tasks import DetectionModel

from model.hydra import get_backbone, set_backbone
from utils.model_naming import (
    HYDRA_MODEL_NAME_TYPE,
    HydraModelName,
    ModelName,
    TaskType,
)
from validation.validator import dataset_name_for_task

logger = logging.getLogger(__name__)

SUPPORTED_SPLIT = "val"
SOURCE_SPLIT = "source"
DEFAULT_IOU_THRESHOLD = 0.5
DEFAULT_OUTPUT_DIR = Path("runs/rendered-predictions")
COCO_SKELETON = (
    (15, 13),
    (13, 11),
    (16, 14),
    (14, 12),
    (11, 12),
    (5, 11),
    (6, 12),
    (5, 6),
    (5, 7),
    (6, 8),
    (7, 9),
    (8, 10),
    (1, 2),
    (0, 1),
    (0, 2),
    (1, 3),
    (2, 4),
    (3, 5),
    (4, 6),
)
CLASS_COLORS_BGR = (
    (56, 140, 255),
    (88, 214, 141),
    (255, 112, 67),
    (171, 71, 188),
    (38, 198, 218),
    (255, 202, 40),
    (239, 83, 80),
    (92, 107, 192),
    (102, 187, 106),
    (255, 138, 101),
)
LABEL_MIN_FONT_SCALE = 0.42
LABEL_MAX_FONT_SCALE = 0.62
LABEL_FONT_BOX_WIDTH_RATIO = 170
LABEL_PAD_X = 5
LABEL_PAD_Y = 4
LABEL_BACKGROUND_ALPHA = 0.65


@dataclass(frozen=True)
class GroundTruthLabel:
    cls: int
    box_xyxy: np.ndarray | None
    polygon_xy: np.ndarray | None


@dataclass(frozen=True)
class Prediction:
    cls: int
    conf: float
    box_xyxy: np.ndarray
    mask_xy: np.ndarray | None = None
    keypoints: np.ndarray | None = None
    iou: float = 0.0


@dataclass(frozen=True)
class RenderConfig:
    conf_threshold: float
    iou_threshold: float | None
    imgsz: int
    device: str
    batch: int
    num_images: int


class ModelPathNotFoundError(click.ClickException):
    def __init__(self, model_name: str, candidates: Sequence[Path]) -> None:
        checked = "\n".join(f"  - {path}" for path in candidates)
        super().__init__(
            f"No checkpoint found for '{model_name}'. Checked:\n{checked}",
        )


def resolve_asset_path(model_name: ModelName, assets_dir: Path) -> Path | None:
    path = assets_dir / model_name.name
    if path.exists():
        return path

    for suffix in (".pt", ".yaml"):
        suffixed_path = path.with_suffix(suffix)
        if suffixed_path.exists():
            return suffixed_path

    return None


def head_checkpoint_candidates(
    hydra_model_name: HydraModelName,
    head: ModelName,
    *,
    assets_dir: Path,
    runs_dir: Path,
    train_dir: Path,
) -> list[Path]:
    candidates: list[Path] = []
    asset_path = assets_dir / head.name
    candidates.extend(
        [
            asset_path,
            asset_path.with_suffix(".pt"),
            asset_path.with_suffix(".yaml"),
        ],
    )

    integrated_model_name = hydra_model_name.integrated_model_name(head)
    if integrated_model_name is not None:
        candidates.extend(
            [
                runs_dir
                / train_dir
                / integrated_model_name
                / "weights"
                / "best.pt",
                runs_dir
                / train_dir
                / integrated_model_name
                / "weights"
                / "last.pt",
                runs_dir
                / train_dir
                / "archive"
                / integrated_model_name
                / "weights"
                / "best.pt",
                runs_dir
                / train_dir
                / "archive"
                / integrated_model_name
                / "weights"
                / "last.pt",
            ],
        )

    return candidates


def resolve_head_path(
    hydra_model_name: HydraModelName,
    *,
    assets_dir: Path,
    runs_dir: Path,
    train_dir: Path,
) -> Path:
    head = hydra_model_name.heads[0]
    candidates = head_checkpoint_candidates(
        hydra_model_name,
        head,
        assets_dir=assets_dir,
        runs_dir=runs_dir,
        train_dir=train_dir,
    )
    for candidate in candidates:
        if candidate.exists():
            return candidate

    raise ModelPathNotFoundError(head.name, candidates)


def resolve_backbone_path(
    hydra_model_name: HydraModelName,
    *,
    assets_dir: Path,
) -> Path:
    path = resolve_asset_path(hydra_model_name.backbone, assets_dir)
    if path is not None:
        return path

    candidates = [
        assets_dir / hydra_model_name.backbone.name,
        (assets_dir / hydra_model_name.backbone.name).with_suffix(".pt"),
        (assets_dir / hydra_model_name.backbone.name).with_suffix(".yaml"),
    ]
    raise ModelPathNotFoundError(hydra_model_name.backbone.name, candidates)


def flatten_hydra_model_names(
    hydra_model_names: Sequence[HydraModelName],
) -> list[HydraModelName]:
    return [
        HydraModelName(
            backbone=hydra_model_name.backbone,
            heads=[head],
            number_of_frozen_modules=(
                hydra_model_name.number_of_frozen_modules
            ),
        )
        for hydra_model_name in hydra_model_names
        for head in hydra_model_name.heads
    ]


def build_hydra_head_model(
    hydra_model_name: HydraModelName,
    *,
    assets_dir: Path,
    runs_dir: Path,
    train_dir: Path,
) -> YOLO:
    backbone_path = resolve_backbone_path(
        hydra_model_name,
        assets_dir=assets_dir,
    )
    head_path = resolve_head_path(
        hydra_model_name,
        assets_dir=assets_dir,
        runs_dir=runs_dir,
        train_dir=train_dir,
    )

    backbone_model = cast(DetectionModel, YOLO(backbone_path).model)
    head_yolo = YOLO(head_path)
    head_model = cast(DetectionModel, head_yolo.model)
    backbone = get_backbone(
        backbone_model,
        hydra_model_name.number_of_frozen_modules,
    )
    set_backbone(
        head_model,
        backbone,
        hydra_model_name.number_of_frozen_modules,
    )
    eval_model = getattr(head_yolo, "eval", None)
    if callable(eval_model):
        eval_model()
    return head_yolo


def dataset_path_for_task(
    *,
    data: Path | None,
    task_type: TaskType,
    object_dataset_name: Path,
    pose_dataset_name: Path,
    segmentation_dataset_name: Path,
    assets_dir: Path,
) -> Path:
    if data is not None:
        return data

    dataset_name = dataset_name_for_task(
        task_type,
        object_dataset_name=object_dataset_name,
        pose_dataset_name=pose_dataset_name,
        segmentation_dataset_name=segmentation_dataset_name,
    )
    return assets_dir / "datasets" / dataset_name


def collect_image_files(image_paths: str | Sequence[str]) -> list[Path]:
    paths = [image_paths] if isinstance(image_paths, str) else list(image_paths)
    image_files: list[Path] = []

    for raw_path in paths:
        path = Path(raw_path)
        if path.is_dir():
            image_files.extend(
                sorted(
                    file_path
                    for file_path in path.rglob("*")
                    if is_image_file(file_path)
                )
            )
            continue

        if path.is_file() and is_image_file(path):
            image_files.append(path)
            continue

        if path.is_file():
            image_files.extend(image_paths_from_list_file(path))
            continue

        raise FileNotFoundError(path)

    return sorted(image_files)


def is_image_file(path: Path) -> bool:
    return path.suffix[1:].lower() in IMG_FORMATS


def image_paths_from_list_file(path: Path) -> list[Path]:
    image_files: list[Path] = []
    parent = path.parent
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        candidate = Path(stripped)
        if stripped.startswith("./"):
            candidate = parent / stripped[2:]
        elif not candidate.is_absolute():
            candidate = parent / candidate
        if is_image_file(candidate):
            image_files.append(candidate)
    return image_files


def sample_images(images: Sequence[Path], num_images: int) -> list[Path]:
    if num_images == -1 or num_images >= len(images):
        return list(images)
    if num_images <= 0:
        raise click.BadParameter(  # noqa: TRY003
            "--num-images must be -1 or a positive integer",
        )
    if num_images == 1:
        return [images[0]]

    step = (len(images) - 1) / (num_images - 1)
    indices = [round(index * step) for index in range(num_images)]
    return [images[index] for index in indices]


def load_val_images(
    data_path: Path,
    *,
    num_images: int,
) -> tuple[dict[str, Any], list[Path]]:
    try:
        dataset = check_det_dataset(str(data_path), autodownload=False)
    except FileNotFoundError as exc:
        raise click.ClickException(  # noqa: TRY003
            f"Could not load validation images from '{data_path}': {exc}",
        ) from exc
    image_files = collect_image_files(cast(str | Sequence[str], dataset["val"]))
    return dataset, sample_images(image_files, num_images)


def load_source_images(
    source_image_dir: Path,
    *,
    num_images: int,
) -> list[Path]:
    if not source_image_dir.is_dir():
        raise click.ClickException(  # noqa: TRY003
            f"Source image path is not a directory: {source_image_dir}",
        )

    image_files = collect_image_files(str(source_image_dir))
    if not image_files:
        raise click.ClickException(  # noqa: TRY003
            f"No images found under source image directory: {source_image_dir}",
        )
    return sample_images(image_files, num_images)


def parse_ground_truth_labels(
    label_path: Path,
    *,
    image_shape: tuple[int, int],
    task_type: TaskType,
) -> list[GroundTruthLabel]:
    if not label_path.exists():
        return []

    labels: list[GroundTruthLabel] = []
    for line in label_path.read_text(encoding="utf-8").splitlines():
        values = parse_label_line(line)
        if values is None:
            continue

        cls = int(values[0])
        if task_type == TaskType.SEGMENTATION and len(values) > 5:
            polygon_xy = normalized_polygon_to_pixels(
                values[1:],
                image_shape,
            )
            labels.append(
                GroundTruthLabel(
                    cls=cls,
                    box_xyxy=polygon_to_box(polygon_xy),
                    polygon_xy=polygon_xy,
                )
            )
            continue

        box_xyxy = normalized_xywh_to_xyxy(values[1:5], image_shape)
        labels.append(
            GroundTruthLabel(
                cls=cls,
                box_xyxy=box_xyxy,
                polygon_xy=box_to_polygon(box_xyxy)
                if task_type == TaskType.SEGMENTATION
                else None,
            )
        )

    return labels


def parse_label_line(line: str) -> list[float] | None:
    stripped = line.strip()
    if not stripped:
        return None

    values = [float(value) for value in stripped.split()]
    if len(values) < 5:
        return None
    return values


def normalized_xywh_to_xyxy(
    values: Sequence[float],
    image_shape: tuple[int, int],
) -> np.ndarray:
    height, width = image_shape
    x_center, y_center, box_width, box_height = values
    x1 = (x_center - box_width / 2) * width
    y1 = (y_center - box_height / 2) * height
    x2 = (x_center + box_width / 2) * width
    y2 = (y_center + box_height / 2) * height
    return clip_box(np.array([x1, y1, x2, y2], dtype=np.float32), image_shape)


def normalized_polygon_to_pixels(
    values: Sequence[float],
    image_shape: tuple[int, int],
) -> np.ndarray:
    height, width = image_shape
    coords = np.array(values, dtype=np.float32).reshape(-1, 2)
    coords[:, 0] = np.clip(coords[:, 0] * width, 0, width - 1)
    coords[:, 1] = np.clip(coords[:, 1] * height, 0, height - 1)
    return coords


def clip_box(box_xyxy: np.ndarray, image_shape: tuple[int, int]) -> np.ndarray:
    height, width = image_shape
    box = box_xyxy.astype(np.float32, copy=True)
    box[[0, 2]] = np.clip(box[[0, 2]], 0, width - 1)
    box[[1, 3]] = np.clip(box[[1, 3]], 0, height - 1)
    return box


def box_to_polygon(box_xyxy: np.ndarray) -> np.ndarray:
    x1, y1, x2, y2 = box_xyxy
    return np.array(
        [[x1, y1], [x2, y1], [x2, y2], [x1, y2]],
        dtype=np.float32,
    )


def polygon_to_box(polygon_xy: np.ndarray) -> np.ndarray:
    x1 = np.min(polygon_xy[:, 0])
    y1 = np.min(polygon_xy[:, 1])
    x2 = np.max(polygon_xy[:, 0])
    y2 = np.max(polygon_xy[:, 1])
    return np.array([x1, y1, x2, y2], dtype=np.float32)


def predictions_from_result(result: Any) -> list[Prediction]:
    boxes = getattr(result, "boxes", None)
    if boxes is None:
        return []

    boxes_data = to_numpy(getattr(boxes, "data", boxes))
    if boxes_data.size == 0:
        return []

    boxes_data = np.atleast_2d(boxes_data)
    masks = masks_from_result(result, len(boxes_data))
    keypoints = keypoints_from_result(result, len(boxes_data))

    predictions: list[Prediction] = []
    for index, row in enumerate(boxes_data):
        predictions.append(
            Prediction(
                cls=int(row[-1]),
                conf=float(row[-2]),
                box_xyxy=row[:4].astype(np.float32),
                mask_xy=masks[index],
                keypoints=keypoints[index],
            )
        )
    return predictions


def masks_from_result(
    result: Any,
    prediction_count: int,
) -> list[np.ndarray | None]:
    masks = getattr(result, "masks", None)
    mask_polygons = getattr(masks, "xy", None)
    if mask_polygons is None:
        return [None] * prediction_count

    return [
        np.asarray(mask_polygons[index], dtype=np.float32)
        if index < len(mask_polygons)
        else None
        for index in range(prediction_count)
    ]


def keypoints_from_result(
    result: Any,
    prediction_count: int,
) -> list[np.ndarray | None]:
    keypoints = getattr(result, "keypoints", None)
    if keypoints is None:
        return [None] * prediction_count

    data = to_numpy(getattr(keypoints, "data", keypoints))
    if data.size == 0:
        return [None] * prediction_count
    data = np.asarray(data, dtype=np.float32)
    return [
        data[index] if index < len(data) else None
        for index in range(prediction_count)
    ]


def to_numpy(value: Any) -> np.ndarray:
    tensor = value
    detach = getattr(tensor, "detach", None)
    if callable(detach):
        tensor = detach()
    cpu = getattr(tensor, "cpu", None)
    if callable(cpu):
        tensor = cpu()
    numpy = getattr(tensor, "numpy", None)
    if callable(numpy):
        return np.asarray(numpy())
    return np.asarray(tensor)


def filter_predictions(
    predictions: Sequence[Prediction],
    labels: Sequence[GroundTruthLabel],
    *,
    task_type: TaskType,
    image_shape: tuple[int, int],
    conf_threshold: float,
    iou_threshold: float,
) -> list[Prediction]:
    filtered: list[Prediction] = []
    for prediction in predictions:
        if prediction.conf < conf_threshold:
            continue

        if iou_threshold <= 0:
            filtered.append(replace(prediction, iou=1.0))
            continue

        best_iou = best_label_iou(
            prediction,
            labels,
            task_type=task_type,
            image_shape=image_shape,
        )
        if best_iou >= iou_threshold:
            filtered.append(replace(prediction, iou=best_iou))

    return filtered


def best_label_iou(
    prediction: Prediction,
    labels: Sequence[GroundTruthLabel],
    *,
    task_type: TaskType,
    image_shape: tuple[int, int],
) -> float:
    same_class_labels = [
        label for label in labels if label.cls == prediction.cls
    ]
    if not same_class_labels:
        return 0.0

    if task_type == TaskType.SEGMENTATION:
        return best_mask_iou(prediction, same_class_labels, image_shape)

    bbox_ious = (
        bbox_iou(prediction.box_xyxy, label.box_xyxy)
        for label in same_class_labels
        if label.box_xyxy is not None
    )
    return max(bbox_ious, default=0.0)


def best_mask_iou(
    prediction: Prediction,
    labels: Sequence[GroundTruthLabel],
    image_shape: tuple[int, int],
) -> float:
    if prediction.mask_xy is None or len(prediction.mask_xy) < 3:
        return 0.0

    prediction_mask = polygon_to_binary_mask(prediction.mask_xy, image_shape)
    best_iou = 0.0
    for label in labels:
        if label.polygon_xy is None or len(label.polygon_xy) < 3:
            continue
        label_mask = polygon_to_binary_mask(label.polygon_xy, image_shape)
        best_iou = max(best_iou, binary_mask_iou(prediction_mask, label_mask))
    return best_iou


def bbox_iou(box_a: np.ndarray, box_b: np.ndarray | None) -> float:
    if box_b is None:
        return 0.0

    x1 = max(float(box_a[0]), float(box_b[0]))
    y1 = max(float(box_a[1]), float(box_b[1]))
    x2 = min(float(box_a[2]), float(box_b[2]))
    y2 = min(float(box_a[3]), float(box_b[3]))
    intersection = max(0.0, x2 - x1) * max(0.0, y2 - y1)
    if intersection == 0:
        return 0.0

    area_a = max(0.0, float(box_a[2] - box_a[0])) * max(
        0.0,
        float(box_a[3] - box_a[1]),
    )
    area_b = max(0.0, float(box_b[2] - box_b[0])) * max(
        0.0,
        float(box_b[3] - box_b[1]),
    )
    union = area_a + area_b - intersection
    return intersection / union if union > 0 else 0.0


def polygon_to_binary_mask(
    polygon_xy: np.ndarray,
    image_shape: tuple[int, int],
) -> np.ndarray:
    polygon = polygon_xy.reshape(-1, 2).astype(np.int32)
    return polygon2mask(image_shape, [polygon]).astype(bool)


def binary_mask_iou(mask_a: np.ndarray, mask_b: np.ndarray) -> float:
    intersection = np.logical_and(mask_a, mask_b).sum()
    union = np.logical_or(mask_a, mask_b).sum()
    return float(intersection / union) if union else 0.0


def result_image(result: Any) -> np.ndarray:
    image = getattr(result, "orig_img", None)
    if image is not None:
        return np.asarray(image).copy()

    path = getattr(result, "path", None)
    if path is None:
        raise ValueError("Result has no orig_img or path")  # noqa: TRY003

    loaded = cv2.imread(str(path))
    if loaded is None:
        raise FileNotFoundError(path)
    return loaded


def result_shape(result: Any) -> tuple[int, int]:
    shape = getattr(result, "orig_shape", None)
    if shape is not None:
        return cast(tuple[int, int], tuple(shape[:2]))
    image = result_image(result)
    return cast(tuple[int, int], image.shape[:2])


def render_predictions(
    image: np.ndarray,
    predictions: Sequence[Prediction],
    *,
    names: Mapping[int, str] | Sequence[str] | None,
    task_type: TaskType,
) -> np.ndarray:
    rendered = ensure_bgr_image(image)
    ordered_predictions = sorted(
        predictions,
        key=prediction_area,
        reverse=True,
    )

    if task_type == TaskType.SEGMENTATION:
        for prediction in ordered_predictions:
            if prediction.mask_xy is not None:
                draw_mask(
                    rendered,
                    prediction.mask_xy,
                    class_color(prediction.cls),
                )

    if task_type == TaskType.POSE:
        for prediction in ordered_predictions:
            draw_pose(rendered, prediction, class_color(prediction.cls))

    for prediction in ordered_predictions:
        color = class_color(prediction.cls)
        draw_rounded_box(rendered, prediction.box_xyxy, color)
        draw_box_label(rendered, prediction, names, color)

    return rendered


def prediction_area(prediction: Prediction) -> float:
    x1, y1, x2, y2 = prediction.box_xyxy
    return max(0.0, float(x2 - x1)) * max(0.0, float(y2 - y1))


def ensure_bgr_image(image: np.ndarray) -> np.ndarray:
    if image.ndim == 2:
        return cv2.cvtColor(image, cv2.COLOR_GRAY2BGR)
    if image.shape[2] == 4:
        return cv2.cvtColor(image, cv2.COLOR_BGRA2BGR)
    return image.copy()


def class_color(cls: int) -> tuple[int, int, int]:
    return CLASS_COLORS_BGR[cls % len(CLASS_COLORS_BGR)]


def draw_mask(
    image: np.ndarray,
    polygon_xy: np.ndarray,
    color: tuple[int, int, int],
) -> None:
    if len(polygon_xy) < 3:
        return
    overlay = image.copy()
    polygon = polygon_xy.reshape(-1, 2).round().astype(np.int32)
    cv2.fillPoly(overlay, [polygon], color=color, lineType=cv2.LINE_AA)
    cv2.addWeighted(overlay, 0.28, image, 0.72, 0, dst=image)


def draw_pose(
    image: np.ndarray,
    prediction: Prediction,
    color: tuple[int, int, int],
) -> None:
    keypoints = prediction.keypoints
    if keypoints is None or len(keypoints) == 0:
        return

    visible = keypoint_visibility(keypoints)
    for start, end in COCO_SKELETON:
        if start >= len(keypoints) or end >= len(keypoints):
            continue
        if not visible[start] or not visible[end]:
            continue
        cv2.line(
            image,
            keypoint_point(keypoints[start]),
            keypoint_point(keypoints[end]),
            color,
            2,
            lineType=cv2.LINE_AA,
        )

    for index, point in enumerate(keypoints):
        if not visible[index]:
            continue
        cv2.circle(
            image,
            keypoint_point(point),
            4,
            (255, 255, 255),
            -1,
            lineType=cv2.LINE_AA,
        )
        cv2.circle(
            image,
            keypoint_point(point),
            3,
            color,
            -1,
            lineType=cv2.LINE_AA,
        )


def keypoint_visibility(keypoints: np.ndarray) -> np.ndarray:
    if keypoints.shape[1] >= 3:
        return keypoints[:, 2] > 0.25
    return (keypoints[:, 0] > 0) & (keypoints[:, 1] > 0)


def keypoint_point(point: np.ndarray) -> tuple[int, int]:
    return round(float(point[0])), round(float(point[1]))


def draw_rounded_box(
    image: np.ndarray,
    box_xyxy: np.ndarray,
    color: tuple[int, int, int],
) -> None:
    height, width = image.shape[:2]
    x1, y1, x2, y2 = clipped_int_box(box_xyxy, width, height)
    if x2 <= x1 or y2 <= y1:
        return

    thickness = max(2, round(min(height, width) / 350))
    radius = min(10, max(3, (x2 - x1) // 8), max(3, (y2 - y1) // 8))
    cv2.line(
        image,
        (x1 + radius, y1),
        (x2 - radius, y1),
        color,
        thickness,
        lineType=cv2.LINE_AA,
    )
    cv2.line(
        image,
        (x1 + radius, y2),
        (x2 - radius, y2),
        color,
        thickness,
        lineType=cv2.LINE_AA,
    )
    cv2.line(
        image,
        (x1, y1 + radius),
        (x1, y2 - radius),
        color,
        thickness,
        lineType=cv2.LINE_AA,
    )
    cv2.line(
        image,
        (x2, y1 + radius),
        (x2, y2 - radius),
        color,
        thickness,
        lineType=cv2.LINE_AA,
    )
    cv2.ellipse(
        image,
        (x1 + radius, y1 + radius),
        (radius, radius),
        180,
        0,
        90,
        color,
        thickness,
        lineType=cv2.LINE_AA,
    )
    cv2.ellipse(
        image,
        (x2 - radius, y1 + radius),
        (radius, radius),
        270,
        0,
        90,
        color,
        thickness,
        lineType=cv2.LINE_AA,
    )
    cv2.ellipse(
        image,
        (x2 - radius, y2 - radius),
        (radius, radius),
        0,
        0,
        90,
        color,
        thickness,
        lineType=cv2.LINE_AA,
    )
    cv2.ellipse(
        image,
        (x1 + radius, y2 - radius),
        (radius, radius),
        90,
        0,
        90,
        color,
        thickness,
        lineType=cv2.LINE_AA,
    )


def draw_box_label(
    image: np.ndarray,
    prediction: Prediction,
    names: Mapping[int, str] | Sequence[str] | None,
    color: tuple[int, int, int],
) -> None:
    height, width = image.shape[:2]
    x1, y1, x2, y2 = clipped_int_box(prediction.box_xyxy, width, height)
    if x2 <= x1 or y2 <= y1:
        return

    text = f"{class_name(names, prediction.cls)} {prediction.conf:.2f}"
    box_width = max(1, x2 - x1)
    preferred_scale = label_font_scale(box_width)
    font_scale, text_size, baseline, text = fit_label(
        text,
        max_width=max(1, width - LABEL_PAD_X * 2),
        preferred_scale=preferred_scale,
    )
    text_width, text_height = text_size
    label_width = min(width, text_width + LABEL_PAD_X * 2)
    label_height = min(
        height,
        text_height + baseline + LABEL_PAD_Y * 2,
    )
    label_x1, label_y1 = label_position(
        box=(x1, y1, x2, y2),
        label_size=(label_width, label_height),
        image_size=(height, width),
    )
    label_x2 = label_x1 + label_width
    label_y2 = label_y1 + label_height

    draw_translucent_rect(
        image,
        rect=(label_x1, label_y1, label_x2, label_y2),
        color=color,
        alpha=LABEL_BACKGROUND_ALPHA,
    )
    text_origin = (
        label_x1 + LABEL_PAD_X,
        label_y1 + LABEL_PAD_Y + text_height,
    )
    cv2.putText(
        image,
        text,
        text_origin,
        cv2.FONT_HERSHEY_SIMPLEX,
        font_scale,
        contrast_text_color(color),
        1,
        lineType=cv2.LINE_AA,
    )


def draw_translucent_rect(
    image: np.ndarray,
    *,
    rect: tuple[int, int, int, int],
    color: tuple[int, int, int],
    alpha: float,
) -> None:
    x1, y1, x2, y2 = rect
    roi = image[y1:y2, x1:x2]
    if roi.size == 0:
        return

    overlay = np.full_like(roi, color)
    cv2.addWeighted(overlay, alpha, roi, 1 - alpha, 0, dst=roi)


def label_position(
    *,
    box: tuple[int, int, int, int],
    label_size: tuple[int, int],
    image_size: tuple[int, int],
) -> tuple[int, int]:
    x1, y1, x2, y2 = box
    label_width, label_height = label_size
    image_height, image_width = image_size
    box_width = max(1, x2 - x1)
    box_height = max(1, y2 - y1)

    if label_width <= box_width and label_height <= box_height:
        return x1, y1

    max_x = max(0, image_width - label_width)
    max_y = max(0, image_height - label_height)
    candidates = [
        (clamp_int(x1, 0, max_x), y1 - label_height),
        (clamp_int(x1, 0, max_x), y2),
        (x2, clamp_int(y1, 0, max_y)),
        (x1 - label_width, clamp_int(y1, 0, max_y)),
    ]

    for candidate_x, candidate_y in candidates:
        candidate = (
            candidate_x,
            candidate_y,
            candidate_x + label_width,
            candidate_y + label_height,
        )
        if is_in_image(candidate, image_width, image_height) and not overlaps(
            candidate,
            box,
        ):
            return candidate_x, candidate_y

    fallback_x, fallback_y = candidates[0]
    return clamp_int(fallback_x, 0, max_x), clamp_int(fallback_y, 0, max_y)


def clamp_int(value: int, lower: int, upper: int) -> int:
    return max(lower, min(value, upper))


def is_in_image(
    rect: tuple[int, int, int, int],
    image_width: int,
    image_height: int,
) -> bool:
    x1, y1, x2, y2 = rect
    return x1 >= 0 and y1 >= 0 and x2 <= image_width and y2 <= image_height


def overlaps(
    rect_a: tuple[int, int, int, int],
    rect_b: tuple[int, int, int, int],
) -> bool:
    ax1, ay1, ax2, ay2 = rect_a
    bx1, by1, bx2, by2 = rect_b
    return ax1 < bx2 and ax2 > bx1 and ay1 < by2 and ay2 > by1


def label_font_scale(box_width: int) -> float:
    return float(
        np.clip(
            box_width / LABEL_FONT_BOX_WIDTH_RATIO,
            LABEL_MIN_FONT_SCALE,
            LABEL_MAX_FONT_SCALE,
        )
    )


def fit_label(
    text: str,
    *,
    max_width: int,
    preferred_scale: float,
) -> tuple[float, tuple[int, int], int, str]:
    font_scale = preferred_scale
    thickness = 1
    text_size, baseline = cv2.getTextSize(
        text,
        cv2.FONT_HERSHEY_SIMPLEX,
        font_scale,
        thickness,
    )
    while text_size[0] > max_width and font_scale > LABEL_MIN_FONT_SCALE:
        font_scale -= 0.05
        text_size, baseline = cv2.getTextSize(
            text,
            cv2.FONT_HERSHEY_SIMPLEX,
            font_scale,
            thickness,
        )

    while text_size[0] > max_width and len(text) > 4:
        text = f"{text[:-4]}..."
        text_size, baseline = cv2.getTextSize(
            text,
            cv2.FONT_HERSHEY_SIMPLEX,
            font_scale,
            thickness,
        )

    return font_scale, text_size, baseline, text


def clipped_int_box(
    box_xyxy: np.ndarray,
    width: int,
    height: int,
) -> tuple[int, int, int, int]:
    x1, y1, x2, y2 = box_xyxy
    return (
        int(np.clip(round(float(x1)), 0, width - 1)),
        int(np.clip(round(float(y1)), 0, height - 1)),
        int(np.clip(round(float(x2)), 0, width - 1)),
        int(np.clip(round(float(y2)), 0, height - 1)),
    )


def class_name(
    names: Mapping[int, str] | Sequence[str] | None,
    cls: int,
) -> str:
    if isinstance(names, Mapping):
        return names.get(cls, str(cls))
    if names is not None and 0 <= cls < len(names):
        return names[cls]
    return str(cls)


def contrast_text_color(color: tuple[int, int, int]) -> tuple[int, int, int]:
    blue, green, red = color
    luminance = 0.114 * blue + 0.587 * green + 0.299 * red
    return (0, 0, 0) if luminance > 150 else (255, 255, 255)


def relative_image_path(image_path: Path, dataset_root: Path) -> Path:
    try:
        relative_path = image_path.resolve().relative_to(dataset_root.resolve())
    except ValueError:
        return Path(image_path.name)
    return flatten_dataset_image_path(relative_path)


def relative_source_image_path(image_path: Path, source_root: Path) -> Path:
    try:
        return image_path.resolve().relative_to(source_root.resolve())
    except ValueError:
        return Path(image_path.name)


def flatten_dataset_image_path(relative_path: Path) -> Path:
    parts = relative_path.parts
    for index, part in enumerate(parts):
        if part != "images":
            continue
        if index + 1 >= len(parts) or parts[index + 1] != SUPPORTED_SPLIT:
            continue
        return Path(*parts[:index], *parts[index + 2 :])
    return relative_path


def result_names(result: Any, model: YOLO) -> Mapping[int, str] | Sequence[str]:
    names = getattr(result, "names", None)
    if names is not None:
        return cast(Mapping[int, str] | Sequence[str], names)
    return cast(Mapping[int, str] | Sequence[str], getattr(model, "names", {}))


def save_image(path: Path, image: np.ndarray) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    success = cv2.imwrite(str(path), image)
    if not success:
        raise OSError(path)


def render_hydra_model(
    hydra_model_name: HydraModelName,
    *,
    data: Path | None,
    object_dataset_name: Path,
    pose_dataset_name: Path,
    segmentation_dataset_name: Path,
    assets_dir: Path,
    runs_dir: Path,
    train_dir: Path,
    output_dir: Path,
    config: RenderConfig,
) -> dict[str, Any]:
    head = hydra_model_name.heads[0]
    task_type = head.task_type()
    dataset_path = dataset_path_for_task(
        data=data,
        task_type=task_type,
        object_dataset_name=object_dataset_name,
        pose_dataset_name=pose_dataset_name,
        segmentation_dataset_name=segmentation_dataset_name,
        assets_dir=assets_dir,
    )
    dataset, image_paths = load_val_images(
        dataset_path,
        num_images=config.num_images,
    )
    label_paths = [
        Path(label_path)
        for label_path in img2label_paths([str(path) for path in image_paths])
    ]
    model = build_hydra_head_model(
        hydra_model_name,
        assets_dir=assets_dir,
        runs_dir=runs_dir,
        train_dir=train_dir,
    )
    model_output_dir = output_dir / str(hydra_model_name)

    prediction_count = 0
    kept_prediction_count = 0
    saved_images: list[str] = []
    results = model.predict(
        source=[str(path) for path in image_paths],
        stream=True,
        conf=config.conf_threshold,
        imgsz=config.imgsz,
        device=config.device,
        batch=config.batch,
        verbose=False,
    )

    dataset_root = Path(dataset["path"])
    for image_path, label_path, result in zip(
        image_paths,
        label_paths,
        results,
        strict=False,
    ):
        image_shape = result_shape(result)
        labels = parse_ground_truth_labels(
            label_path,
            image_shape=image_shape,
            task_type=task_type,
        )
        predictions = predictions_from_result(result)
        filtered = filter_predictions(
            predictions,
            labels,
            task_type=task_type,
            image_shape=image_shape,
            conf_threshold=config.conf_threshold,
            iou_threshold=cast(float, config.iou_threshold),
        )
        prediction_count += len(predictions)
        kept_prediction_count += len(filtered)

        rendered = render_predictions(
            result_image(result),
            filtered,
            names=result_names(result, model),
            task_type=task_type,
        )
        output_path = (
            model_output_dir
            / SUPPORTED_SPLIT
            / relative_image_path(image_path, dataset_root)
        )
        save_image(output_path, rendered)
        saved_images.append(str(output_path))

    manifest = {
        "hydra_model_name": str(hydra_model_name),
        "head_model_name": str(head),
        "task": str(task_type),
        "dataset": str(dataset_path),
        "dataset_root": str(dataset_root),
        "split": SUPPORTED_SPLIT,
        "config": asdict(config),
        "runs_dir": str(runs_dir),
        "train_dir": str(train_dir),
        "image_count": len(image_paths),
        "prediction_count": prediction_count,
        "kept_prediction_count": kept_prediction_count,
        "saved_images": saved_images,
    }
    manifest_path = model_output_dir / "manifest.json"
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(
        json.dumps(manifest, indent=2),
        encoding="utf-8",
    )
    logger.info("Saved manifest to %s", manifest_path)
    return manifest


def render_hydra_source_model(
    hydra_model_name: HydraModelName,
    *,
    source_image_dir: Path,
    assets_dir: Path,
    runs_dir: Path,
    train_dir: Path,
    output_dir: Path,
    config: RenderConfig,
) -> dict[str, Any]:
    head = hydra_model_name.heads[0]
    task_type = head.task_type()
    image_paths = load_source_images(
        source_image_dir,
        num_images=config.num_images,
    )
    model = build_hydra_head_model(
        hydra_model_name,
        assets_dir=assets_dir,
        runs_dir=runs_dir,
        train_dir=train_dir,
    )
    model_output_dir = output_dir / str(hydra_model_name)

    prediction_count = 0
    kept_prediction_count = 0
    saved_images: list[str] = []
    results = model.predict(
        source=[str(path) for path in image_paths],
        stream=True,
        conf=config.conf_threshold,
        imgsz=config.imgsz,
        device=config.device,
        batch=config.batch,
        verbose=False,
    )

    for image_path, result in zip(image_paths, results, strict=False):
        predictions = predictions_from_result(result)
        filtered = [
            prediction
            for prediction in predictions
            if prediction.conf >= config.conf_threshold
        ]
        prediction_count += len(predictions)
        kept_prediction_count += len(filtered)

        rendered = render_predictions(
            result_image(result),
            filtered,
            names=result_names(result, model),
            task_type=task_type,
        )
        output_path = (
            model_output_dir
            / SOURCE_SPLIT
            / relative_source_image_path(image_path, source_image_dir)
        )
        save_image(output_path, rendered)
        saved_images.append(str(output_path))

    manifest = {
        "hydra_model_name": str(hydra_model_name),
        "head_model_name": str(head),
        "task": str(task_type),
        "mode": SOURCE_SPLIT,
        "source_image_dir": str(source_image_dir),
        "split": SOURCE_SPLIT,
        "config": asdict(config),
        "runs_dir": str(runs_dir),
        "train_dir": str(train_dir),
        "image_count": len(image_paths),
        "prediction_count": prediction_count,
        "kept_prediction_count": kept_prediction_count,
        "saved_images": saved_images,
    }
    manifest_path = model_output_dir / "manifest.json"
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(
        json.dumps(manifest, indent=2),
        encoding="utf-8",
    )
    logger.info("Saved manifest to %s", manifest_path)
    return manifest


def validate_cli_config(
    *,
    data: Path | None,
    source_image_dir: Path | None,
    conf_threshold: float,
    iou_threshold: float | None,
    imgsz: int,
    batch: int,
    num_images: int,
) -> None:
    if data is not None and source_image_dir is not None:
        raise click.BadParameter(  # noqa: TRY003
            "--data and --source-image-dir are mutually exclusive",
        )
    if source_image_dir is not None and iou_threshold is not None:
        raise click.BadParameter(  # noqa: TRY003
            "--source-image-dir and --iou-threshold are mutually exclusive",
        )
    if not 0 <= conf_threshold <= 1:
        raise click.BadParameter(  # noqa: TRY003
            "--conf-threshold must be between 0 and 1",
        )
    if iou_threshold is not None and not 0 <= iou_threshold <= 1:
        raise click.BadParameter(  # noqa: TRY003
            "--iou-threshold must be between 0 and 1",
        )
    if imgsz <= 0:
        raise click.BadParameter("--imgsz must be > 0")  # noqa: TRY003
    if batch <= 0:
        raise click.BadParameter("--batch must be > 0")  # noqa: TRY003
    if num_images != -1 and num_images <= 0:
        raise click.BadParameter(  # noqa: TRY003
            "--num-images must be -1 or a positive integer",
        )


@click.command(
    context_settings={"help_option_names": ["-h", "--help"]},
    help=(
        "Render predictions for Hydra YOLO heads on dataset or source images."
    ),
)
@click.option(
    "--hydra-model-name",
    "--hydra_model_name",
    "hydra_model_names",
    multiple=True,
    required=True,
    type=HYDRA_MODEL_NAME_TYPE,
    help=(
        "Hydra model name using the given naming convention. "
        "Example: yolo26m=f11+yolo26m+yolo26m-pose"
    ),
)
@click.option(
    "--data",
    type=click.Path(path_type=Path),
    default=None,
    help=(
        "Optional dataset YAML or dataset directory used for every head. "
        "If omitted, task-specific dataset names under --assets-dir are used."
    ),
)
@click.option(
    "--source-image-dir",
    "--source_image_dir",
    type=click.Path(path_type=Path),
    default=None,
    help=(
        "Optional directory of unlabeled source images. All Hydra heads use "
        "the same sampled images. Mutually exclusive with --data and "
        "--iou-threshold."
    ),
)
@click.option(
    "--object-dataset-name",
    "--object_dataset_name",
    type=click.Path(path_type=Path),
    default=Path("coco.yaml"),
    show_default=True,
    help="Object detection dataset name relative to <assets-dir>/datasets.",
)
@click.option(
    "--pose-dataset-name",
    "--pose_dataset_name",
    type=click.Path(path_type=Path),
    default=Path("coco-pose.yaml"),
    show_default=True,
    help="Pose dataset name relative to <assets-dir>/datasets.",
)
@click.option(
    "--segmentation-dataset-name",
    "--segmentation_dataset_name",
    type=click.Path(path_type=Path),
    default=Path("coco.yaml"),
    show_default=True,
    help="Segmentation dataset name relative to <assets-dir>/datasets.",
)
@click.option(
    "--assets-dir",
    "--assets_dir",
    type=click.Path(path_type=Path),
    default=Path("assets"),
    show_default=True,
    help="Directory containing model and dataset assets.",
)
@click.option(
    "--runs-dir",
    "--runs_dir",
    type=click.Path(path_type=Path),
    default=Path("runs"),
    show_default=True,
    help="Directory containing training and validation runs.",
)
@click.option(
    "--train-dir",
    "--train_dir",
    type=click.Path(path_type=Path),
    default=Path("train"),
    show_default=True,
    help="Training run directory relative to --runs-dir.",
)
@click.option(
    "--output-dir",
    "--output_dir",
    type=click.Path(path_type=Path),
    default=DEFAULT_OUTPUT_DIR,
    show_default=True,
    help="Directory where rendered images and manifests are written.",
)
@click.option(
    "--conf-threshold",
    type=float,
    default=0.25,
    show_default=True,
    help="Minimum model confidence before label IOU filtering.",
)
@click.option(
    "--iou-threshold",
    type=float,
    default=None,
    help=(
        "Minimum same-class ground-truth IOU in dataset mode. "
        f"Defaults to {DEFAULT_IOU_THRESHOLD}. Mutually exclusive with "
        "--source-image-dir."
    ),
)
@click.option(
    "--imgsz",
    type=int,
    default=640,
    show_default=True,
    help="Prediction image size.",
)
@click.option(
    "--device",
    default="-1",
    show_default=True,
    help="Ultralytics device argument, e.g. -1, cpu, cuda, or 0.",
)
@click.option(
    "--batch",
    type=int,
    default=1,
    show_default=True,
    help="Prediction batch size.",
)
@click.option(
    "--num-images",
    "--num_images",
    type=int,
    default=-1,
    show_default=True,
    help="Number of images to label. -1 labels all selected images.",
)
def main(
    *,
    hydra_model_names: tuple[HydraModelName, ...],
    data: Path | None,
    source_image_dir: Path | None,
    object_dataset_name: Path,
    pose_dataset_name: Path,
    segmentation_dataset_name: Path,
    assets_dir: Path,
    runs_dir: Path,
    train_dir: Path,
    output_dir: Path,
    conf_threshold: float,
    iou_threshold: float,
    imgsz: int,
    device: str,
    batch: int,
    num_images: int,
) -> None:
    validate_cli_config(
        data=data,
        source_image_dir=source_image_dir,
        conf_threshold=conf_threshold,
        iou_threshold=iou_threshold,
        imgsz=imgsz,
        batch=batch,
        num_images=num_images,
    )
    effective_iou_threshold = (
        None
        if source_image_dir is not None
        else (
            DEFAULT_IOU_THRESHOLD
            if iou_threshold is None
            else iou_threshold
        )
    )
    config = RenderConfig(
        conf_threshold=conf_threshold,
        iou_threshold=effective_iou_threshold,
        imgsz=imgsz,
        device=device,
        batch=batch,
        num_images=num_images,
    )

    flattened_hydra_model_names = flatten_hydra_model_names(hydra_model_names)
    if source_image_dir is not None:
        manifests = [
            render_hydra_source_model(
                hydra_model_name,
                source_image_dir=source_image_dir,
                assets_dir=assets_dir,
                runs_dir=runs_dir,
                train_dir=train_dir,
                output_dir=output_dir,
                config=config,
            )
            for hydra_model_name in flattened_hydra_model_names
        ]
    else:
        manifests = [
            render_hydra_model(
                hydra_model_name,
                data=data,
                object_dataset_name=object_dataset_name,
                pose_dataset_name=pose_dataset_name,
                segmentation_dataset_name=segmentation_dataset_name,
                assets_dir=assets_dir,
                runs_dir=runs_dir,
                train_dir=train_dir,
                output_dir=output_dir,
                config=config,
            )
            for hydra_model_name in flattened_hydra_model_names
        ]
    click.echo(f"Rendered {len(manifests)} Hydra head(s) to {output_dir}")


if __name__ == "__main__":
    main()
