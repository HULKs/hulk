import cv2
import numpy as np
import torch
from ultralytics.data.augment import LetterBox
from ultralytics.utils.nms import non_max_suppression
from ultralytics.utils.ops import (
    scale_boxes,
    scale_coords,
)
from ultralytics.utils.plotting import Annotator, colors


class MultiTaskPredictor:
    def __init__(
        self,
        multi_task_model,
        device="cuda" if torch.cuda.is_available() else "cpu",
    ):
        self.device = device
        self.model = multi_task_model.to(self.device)
        self.model.eval()
        self.preprocessor = LetterBox(new_shape=(640, 640))
        self.task_class_names = getattr(self.model, "task_class_names", {})

        # --- NEW: Dynamically extract task properties ---
        self.task_meta = {}
        for task_name, branch in self.model.task_branches.items():
            head = branch[-1]  # The final module is the Head
            self.task_meta[task_name] = {
                "nc": getattr(head, "nc", 80),
                "kpt_shape": getattr(head, "kpt_shape", [17, 3]),
            }
            print(
                f"Meta extracted for {task_name} -> nc: {self.task_meta[task_name]['nc']}"
            )

    @staticmethod
    def _extract_prediction_tensor(raw_output):
        """Get the primary prediction tensor from nested Ultralytics outputs."""
        if isinstance(raw_output, torch.Tensor):
            return raw_output
        if isinstance(raw_output, (list, tuple)) and len(raw_output):
            first = raw_output[0]
            if isinstance(first, torch.Tensor):
                return first
        raise TypeError(
            f"Unsupported prediction output type: {type(raw_output)}"
        )

    @staticmethod
    def _is_end2end_decoded(pred_tensor):
        # End-to-end outputs are typically [batch, max_det, dims] (e.g. [1, 300, 57]).
        return pred_tensor.ndim == 3 and pred_tensor.shape[-1] <= 512

    @staticmethod
    def _filter_end2end_predictions(pred_tensor, conf_thres):
        filtered = []
        for sample in pred_tensor:
            filtered.append(sample[sample[:, 4] > conf_thres])
        return filtered

    @staticmethod
    def _scale_pose_keypoints(
        pose_predictions, resized_shape, img_shape, kpt_shape
    ):
        if len(pose_predictions) == 0:
            return

        n_kpt, kpt_dim = int(kpt_shape[0]), int(kpt_shape[1])
        if n_kpt <= 0 or kpt_dim < 2:
            return

        expected_len = n_kpt * kpt_dim
        actual_len = pose_predictions.shape[1] - 6
        if actual_len != expected_len:
            return

        kpts = pose_predictions[:, 6:].view(-1, n_kpt, kpt_dim)
        kpts[..., :2] = scale_coords(resized_shape, kpts[..., :2], img_shape)
        pose_predictions[:, 6:] = kpts.view(len(pose_predictions), -1)

    def preprocess(self, img_path):
        img0 = cv2.imread(img_path)
        img = self.preprocessor(image=img0)
        img = img.transpose((2, 0, 1))[::-1]
        img = np.ascontiguousarray(img)
        img_tensor = torch.from_numpy(img).to(self.device).float() / 255.0
        if len(img_tensor.shape) == 3:
            img_tensor = img_tensor[None]
        return img_tensor, img0, img.shape[1:]

    def predict(self, img_path, conf_thres=0.25, iou_thres=0.45):
        img_tensor, img_orig, resized_shape = self.preprocess(img_path)

        with torch.no_grad():
            raw_outputs = self.model(img_tensor)

        results = {}

        if "detection" in raw_outputs:
            det_raw = self._extract_prediction_tensor(raw_outputs["detection"])
            if self._is_end2end_decoded(det_raw):
                det_preds = self._filter_end2end_predictions(
                    det_raw, conf_thres
                )
            else:
                nc_det = self.task_meta["detection"]["nc"]
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
                nc_pose = self.task_meta["pose"]["nc"]
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
                    self.task_meta["pose"]["kpt_shape"],
                )

            results["pose"] = pose_preds[0]

        return results, img_orig


def visualize_multi_task_predictions(
    original_image,
    predictions,
    kpt_shape=None,
    save_path="unified_output.jpg",
    detection_class_names=None,
):
    annotator = Annotator(original_image.copy(), line_width=2, font_size=10)

    # 1. Detection
    if "detection" in predictions and len(predictions["detection"]) > 0:
        for det in predictions["detection"]:
            x1, y1, x2, y2, conf, cls_id = det[:6].tolist()
            cls_id = int(cls_id)
            cls_name = str(cls_id)
            if isinstance(detection_class_names, dict):
                cls_name = detection_class_names.get(cls_id, cls_name)
            elif isinstance(detection_class_names, list):
                if 0 <= cls_id < len(detection_class_names):
                    cls_name = detection_class_names[cls_id]
            label = f"{cls_name} {conf:.2f}"
            annotator.box_label(
                [x1, y1, x2, y2], label, color=colors(cls_id, True)
            )

    # 2. Pose
    if "pose" in predictions and len(predictions["pose"]) > 0:
        for pose in predictions["pose"]:
            box = pose[:4].tolist()
            conf = pose[4].item()
            cls_id = int(pose[5].item())

            label = f"Person {conf:.2f}"
            annotator.box_label(box, label, color=(0, 255, 0))

            # --- THE UNBREAKABLE KEYPOINT EXTRACTOR ---
            kpts_flat = pose[6:]
            actual_len = len(kpts_flat)

            # Check if the user passed a shape that perfectly matches
            if kpt_shape is not None and (
                kpt_shape[0] * kpt_shape[1] == actual_len
            ):
                kpts = kpts_flat.view(*kpt_shape).cpu().numpy()
                annotator.kpts(kpts, shape=original_image.shape, kpt_line=True)
            else:
                # Auto-detect shape based on tensor length to prevent crashes
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
                else:
                    # If it's a completely foreign shape (like 295), skip drawing lines but don't crash
                    print(
                        f"⚠️ Warning: Model output {actual_len} keypoint elements (not divisible by 2 or 3). Skipping skeleton lines."
                    )

    annotated_image = annotator.result()
    cv2.imwrite(save_path, annotated_image)
    print(f"🎨 Success! Annotated image saved to: {save_path}")

    return annotated_image
