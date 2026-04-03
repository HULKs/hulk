import logging
import os
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any, cast

import cv2
import numpy as np
import numpy.typing as npt
import torch
from torch import nn
from ultralytics.data.augment import LetterBox
from ultralytics.utils.nms import non_max_suppression
from ultralytics.utils.ops import (
    scale_boxes,
    scale_coords,
)
from ultralytics.utils.plotting import Annotator, colors

from model.multi_task_yolo import Hydra

logger = logging.getLogger(__name__)

ImageArray = npt.NDArray[np.uint8]
Shape2D = tuple[int, int]
TaskOutputs = dict[str, torch.Tensor]
ClassNames = Mapping[int, str] | Sequence[str] | None


@dataclass(frozen=True)
class TaskMeta:
    nc: int
    kpt_shape: tuple[int, int]


class UnsupportedPredictionOutputError(TypeError):
    def __init__(self) -> None:
        super().__init__("Unsupported prediction output format")


class ImageLoadError(FileNotFoundError):
    def __init__(self) -> None:
        super().__init__("Image could not be loaded")


class InvalidModelOutputError(TypeError):
    def __init__(self) -> None:
        super().__init__("Model output must be a mapping")


class MultiTaskPredictor:
    def __init__(
        self,
        multi_task_model: nn.Module,
        device: str | None = None,
    ) -> None:
        self.device = device or ("cuda" if torch.cuda.is_available() else "cpu")
        self.model = multi_task_model.to(self.device)
        self.model.eval()
        self.preprocessor = LetterBox(new_shape=(640, 640))
        self.task_class_names = cast(
            dict[str, ClassNames],
            getattr(self.model, "head_class_names", {}),
        )

        self.task_meta: dict[str, TaskMeta] = {}
        heads = cast(
            dict[str, nn.ModuleList],
            getattr(self.model, "heads", {}),
        )
        for head_name, head in heads.items():
            head = head[-1]
            self.task_meta[head_name] = TaskMeta(
                nc=int(getattr(head, "nc", 80)),
                kpt_shape=self._parse_kpt_shape(
                    getattr(head, "kpt_shape", (17, 3))
                ),
            )

    @staticmethod
    def _parse_kpt_shape(raw_shape: Any) -> tuple[int, int]:
        if isinstance(raw_shape, Sequence) and len(raw_shape) >= 2:
            return int(raw_shape[0]), int(raw_shape[1])
        return 17, 3

    @staticmethod
    def _extract_prediction_tensor(raw_output: Any) -> torch.Tensor:
        if isinstance(raw_output, torch.Tensor):
            return raw_output
        if isinstance(raw_output, (list, tuple)) and len(raw_output):
            first = raw_output[0]
            if isinstance(first, torch.Tensor):
                return first
        raise UnsupportedPredictionOutputError

    @staticmethod
    def _is_end2end_decoded(pred_tensor: torch.Tensor) -> bool:
        return pred_tensor.ndim == 3 and pred_tensor.shape[-1] <= 512

    @staticmethod
    def _filter_end2end_predictions(
        pred_tensor: torch.Tensor,
        conf_thres: float,
    ) -> list[torch.Tensor]:
        filtered: list[torch.Tensor] = []
        for sample in pred_tensor:
            filtered.append(sample[sample[:, 4] > conf_thres])
        return filtered

    @staticmethod
    def _scale_pose_keypoints(
        pose_predictions: torch.Tensor,
        resized_shape: Shape2D,
        img_shape: tuple[int, int, int],
        kpt_shape: tuple[int, int],
    ) -> None:
        if len(pose_predictions) == 0:
            return

        n_kpt, kpt_dim = kpt_shape[0], kpt_shape[1]
        if n_kpt <= 0 or kpt_dim < 2:
            return

        expected_len = n_kpt * kpt_dim
        actual_len = pose_predictions.shape[1] - 6
        if actual_len != expected_len:
            return

        kpts = pose_predictions[:, 6:].view(-1, n_kpt, kpt_dim)
        kpts[..., :2] = scale_coords(resized_shape, kpts[..., :2], img_shape)
        pose_predictions[:, 6:] = kpts.view(len(pose_predictions), -1)

    def preprocess(
        self,
        img_path: str | Path,
    ) -> tuple[torch.Tensor, ImageArray, Shape2D]:
        image_bgr = cv2.imread(str(img_path))
        if image_bgr is None:
            raise ImageLoadError

        preprocessed = self.preprocessor(image=image_bgr)
        if isinstance(preprocessed, dict):
            letterboxed = cast(ImageArray, preprocessed["img"])
        else:
            letterboxed = cast(ImageArray, preprocessed)

        chw_rgb = letterboxed.transpose((2, 0, 1))[::-1]
        chw_rgb = np.ascontiguousarray(chw_rgb)
        img_tensor = torch.from_numpy(chw_rgb).to(self.device).float() / 255.0
        if len(img_tensor.shape) == 3:
            img_tensor = img_tensor[None]
        resized_shape = (int(letterboxed.shape[0]), int(letterboxed.shape[1]))
        return img_tensor, cast(ImageArray, image_bgr), resized_shape

    def predict(
        self,
        img_path: str | Path,
        conf_thres: float = 0.25,
        iou_thres: float = 0.45,
    ) -> tuple[TaskOutputs, ImageArray]:
        img_tensor, img_orig, resized_shape = self.preprocess(img_path)

        with torch.no_grad():
            raw_outputs = self.model(img_tensor)

        if not isinstance(raw_outputs, Mapping):
            raise InvalidModelOutputError

        results: TaskOutputs = {}

        if "detection" in raw_outputs:
            det_raw = self._extract_prediction_tensor(raw_outputs["detection"])
            if self._is_end2end_decoded(det_raw):
                det_preds = self._filter_end2end_predictions(
                    det_raw, conf_thres
                )
            else:
                nc_det = self.task_meta["detection"].nc
                det_preds = non_max_suppression(
                    det_raw, conf_thres, iou_thres, nc=nc_det
                )
            if len(det_preds[0]):
                det_preds[0][:, :4] = scale_boxes(
                    resized_shape, det_preds[0][:, :4], img_orig.shape
                )
            results["detection"] = det_preds[0]

        if "pose" in raw_outputs:
            pose_raw = self._extract_prediction_tensor(raw_outputs["pose"])
            if self._is_end2end_decoded(pose_raw):
                pose_preds = self._filter_end2end_predictions(
                    pose_raw, conf_thres
                )
            else:
                nc_pose = self.task_meta["pose"].nc
                pose_preds = non_max_suppression(
                    pose_raw, conf_thres, iou_thres, nc=nc_pose
                )

            if len(pose_preds[0]):
                pose_preds[0][:, :4] = scale_boxes(
                    resized_shape, pose_preds[0][:, :4], img_orig.shape
                )
                self._scale_pose_keypoints(
                    pose_preds[0],
                    resized_shape,
                    img_orig.shape,
                    self.task_meta["pose"].kpt_shape,
                )

            results["pose"] = pose_preds[0]

        for head_name, head_predictions in results.items():
            logger.info("%s: %d detections", head_name, len(head_predictions))

        return results, img_orig


def visualize_multi_task_predictions(
    original_image: ImageArray,
    predictions: Mapping[str, torch.Tensor],
    kpt_shape: tuple[int, int] | None = None,
    save_path: str | Path = "unified_output.jpg",
    detection_class_names: ClassNames = None,
) -> ImageArray:
    annotator = Annotator(original_image.copy(), line_width=2, font_size=10)

    if "detection" in predictions and len(predictions["detection"]) > 0:
        for det in predictions["detection"]:
            x1, y1, x2, y2, conf, cls_id = det[:6].tolist()
            cls_id = int(cls_id)
            cls_name = str(cls_id)
            if isinstance(detection_class_names, dict):
                cls_name = detection_class_names.get(cls_id, cls_name)
            elif (
                isinstance(detection_class_names, Sequence)
                and not isinstance(detection_class_names, str)
                and 0 <= cls_id < len(detection_class_names)
            ):
                cls_name = detection_class_names[cls_id]
            label = f"{cls_name} {conf:.2f}"
            annotator.box_label(
                [x1, y1, x2, y2], label, color=colors(cls_id, bgr=True)
            )

    if "pose" in predictions and len(predictions["pose"]) > 0:
        for pose in predictions["pose"]:
            box = pose[:4].tolist()
            conf = pose[4].item()

            label = f"Person {conf:.2f}"
            annotator.box_label(box, label, color=(0, 255, 0))

            kpts_flat = pose[6:]
            actual_len = len(kpts_flat)

            if kpt_shape is not None and (
                kpt_shape[0] * kpt_shape[1] == actual_len
            ):
                kpts = kpts_flat.view(*kpt_shape).cpu().numpy()
                annotator.kpts(kpts, shape=original_image.shape, kpt_line=True)
            else:
                if actual_len % 3 == 0:
                    kpts = kpts_flat.view(-1, 3).cpu().numpy()
                    annotator.kpts(
                        kpts, shape=original_image.shape, kpt_line=True
                    )
                elif actual_len % 2 == 0:
                    kpts = kpts_flat.view(-1, 2).cpu().numpy()
                    annotator.kpts(
                        kpts, shape=original_image.shape, kpt_line=True
                    )

    annotated = annotator.result()
    annotated_image = cast(ImageArray, np.asarray(annotated))
    cv2.imwrite(str(save_path), annotated_image)
    return annotated_image


def main() -> None:
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s | %(levelname)s | %(message)s",
    )

    tasks = {
        "detection": "src/validation/assets/yolo26m-tuned.pt",
        "pose": "src/validation/assets/yolo26m-pose.pt",
    }

    multi_task_model = Hydra(
        foundation_path="src/validation/assets/yolo26m-tuned.pt",
        task_dict=tasks,
    )

    predictor = MultiTaskPredictor(multi_task_model)

    predictions, original_image = predictor.predict(
        "src/validation/assets/2173162b-cd77-4dec-923b-e28eafd3297c.png",
        conf_thres=0.1,
    )

    pose_shape = predictor.task_meta["pose"].kpt_shape
    detection_class_names = predictor.task_class_names.get("detection")

    os.makedirs("src/validation/output", exist_ok=True)
    visualize_multi_task_predictions(
        original_image,
        predictions,
        kpt_shape=pose_shape,
        save_path="src/validation/output/test.png",
        detection_class_names=detection_class_names,
    )


if __name__ == "__main__":
    main()
