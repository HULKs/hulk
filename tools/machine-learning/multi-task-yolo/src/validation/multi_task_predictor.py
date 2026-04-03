import logging
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any, cast

import cv2
import numpy as np
import numpy.typing as npt
import torch
from torch import nn
from ultralytics.engine.results import Results
from ultralytics.models.yolo.detect.predict import DetectionPredictor
from ultralytics.models.yolo.pose.predict import PosePredictor
from ultralytics.nn.autobackend import check_class_names

from model.multi_task_yolo import Hydra

logger = logging.getLogger(__name__)

ImageArray = npt.NDArray[np.uint8]
Shape2D = tuple[int, int]
TaskResults = dict[str, Results]
ClassNames = Mapping[int, str] | Sequence[str] | None


@dataclass(frozen=True)
class TaskMeta:
    names: dict[int, str]
    kpt_shape: tuple[int, int] | None = None


@dataclass
class PredictorModelProxy:
    names: dict[int, str]
    kpt_shape: tuple[int, int] | None = None
    fp16: bool = False
    stride: int = 32
    format: str = "pt"
    dynamic: bool = False
    end2end: bool = True


class ImageLoadError(FileNotFoundError):
    def __init__(self) -> None:
        super().__init__("Image could not be loaded")


class InvalidModelOutputError(TypeError):
    def __init__(self) -> None:
        super().__init__("Model output must be a mapping")


class MissingTaskPredictorError(RuntimeError):
    def __init__(self) -> None:
        super().__init__(
            "No supported task predictors available for preprocessing"
        )


class MultiTaskPredictor:
    def __init__(
        self,
        multi_task_model: nn.Module,
        device: str | None = None,
        imgsz: Shape2D = (640, 640),
    ) -> None:
        self.device = torch.device(
            device or ("cuda" if torch.cuda.is_available() else "cpu")
        )
        self.model = multi_task_model.to(self.device)
        self.model.eval()
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
            raw_kpt_shape = getattr(head, "kpt_shape", None)
            self.task_meta[head_name] = TaskMeta(
                names=self._normalize_class_names(
                    self.task_class_names.get(head_name)
                ),
                kpt_shape=(
                    self._parse_kpt_shape(raw_kpt_shape)
                    if raw_kpt_shape is not None
                    else None
                ),
            )

        self.task_predictors = self._build_task_predictors(imgsz)
        self.preprocess_predictor = self._select_preprocess_predictor()

    @staticmethod
    def _parse_kpt_shape(raw_shape: Any) -> tuple[int, int]:
        if isinstance(raw_shape, Sequence) and len(raw_shape) >= 2:
            return int(raw_shape[0]), int(raw_shape[1])
        return 17, 3

    @staticmethod
    def _normalize_class_names(class_names: ClassNames) -> dict[int, str]:
        if isinstance(class_names, Mapping):
            return check_class_names(dict(class_names))
        if isinstance(class_names, Sequence) and not isinstance(
            class_names, str
        ):
            return check_class_names(list(class_names))
        return {}

    def _configure_predictor(
        self,
        predictor: DetectionPredictor | PosePredictor,
        model_proxy: PredictorModelProxy,
        imgsz: Shape2D,
    ) -> None:
        runtime_predictor = cast(Any, predictor)
        runtime_predictor.model = model_proxy
        runtime_predictor.device = self.device
        runtime_predictor.imgsz = list(imgsz)

    def _build_task_predictors(
        self,
        imgsz: Shape2D,
    ) -> dict[str, DetectionPredictor | PosePredictor]:
        task_predictors: dict[str, DetectionPredictor | PosePredictor] = {}

        detection_meta = self.task_meta.get("detection")
        if detection_meta is not None:
            detection_predictor = DetectionPredictor(
                overrides={"imgsz": list(imgsz), "task": "detect"}
            )
            self._configure_predictor(
                predictor=detection_predictor,
                model_proxy=PredictorModelProxy(names=detection_meta.names),
                imgsz=imgsz,
            )
            task_predictors["detection"] = detection_predictor

        pose_meta = self.task_meta.get("pose")
        if pose_meta is not None:
            pose_predictor = PosePredictor(
                overrides={"imgsz": list(imgsz), "task": "pose"}
            )
            self._configure_predictor(
                predictor=pose_predictor,
                model_proxy=PredictorModelProxy(
                    names=pose_meta.names,
                    kpt_shape=pose_meta.kpt_shape or (17, 3),
                    end2end=True,
                ),
                imgsz=imgsz,
            )
            task_predictors["pose"] = pose_predictor

        return task_predictors

    def _select_preprocess_predictor(
        self,
    ) -> DetectionPredictor | PosePredictor:
        detection_predictor = self.task_predictors.get("detection")
        if detection_predictor is not None:
            return detection_predictor

        pose_predictor = self.task_predictors.get("pose")
        if pose_predictor is not None:
            return pose_predictor

        raise MissingTaskPredictorError

    def preprocess(
        self,
        img_path: str | Path,
    ) -> tuple[torch.Tensor, ImageArray]:
        image_bgr = cv2.imread(str(img_path))
        if image_bgr is None:
            raise ImageLoadError

        img_tensor = self.preprocess_predictor.preprocess([image_bgr])
        return img_tensor, cast(ImageArray, image_bgr)

    def predict(
        self,
        img_path: str | Path,
        conf_thres: float = 0.25,
        iou_thres: float = 0.45,
    ) -> tuple[TaskResults, ImageArray]:
        img_tensor, img_orig = self.preprocess(img_path)

        with torch.no_grad():
            raw_outputs = self.model(img_tensor)

        if not isinstance(raw_outputs, Mapping):
            raise InvalidModelOutputError

        results: TaskResults = {}

        for task_name, raw_output in raw_outputs.items():
            task_predictor = self.task_predictors.get(task_name)
            if task_predictor is None:
                logger.warning("Skipping unsupported task head: %s", task_name)
                continue

            task_predictor.args.conf = conf_thres
            task_predictor.args.iou = iou_thres
            task_predictor.batch = ([str(img_path)], None, None)

            task_results = task_predictor.postprocess(
                raw_output,
                img_tensor,
                [img_orig],
            )
            if not task_results:
                continue
            result = task_results[0]

            results[task_name] = result
            logger.info("%s: %s", task_name, result.verbose().strip())

        return results, img_orig


def visualize_multi_task_predictions(
    original_image: ImageArray,
    predictions: Mapping[str, Results],
    save_path: str | Path = "unified_output.jpg",
) -> ImageArray:
    annotated_image = original_image.copy()

    if "detection" in predictions:
        annotated_image = predictions["detection"].plot(img=annotated_image)

    if "pose" in predictions:
        annotated_image = predictions["pose"].plot(
            img=annotated_image,
            color_mode="instance",
            kpt_line=True,
        )

    cv2.imwrite(str(save_path), annotated_image)
    return cast(ImageArray, np.asarray(annotated_image))


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

    output_dir = Path("src/validation/output")
    output_dir.mkdir(parents=True, exist_ok=True)
    visualize_multi_task_predictions(
        original_image,
        predictions,
        save_path=output_dir / "test.png",
    )


if __name__ == "__main__":
    main()
