import torch
import torch.nn as nn
from ultralytics import YOLO

from multi_task_predictor import (
    MultiTaskPredictor,
    visualize_multi_task_predictions,
)

# ==========================================
# 1. Your Dynamic Helper Functions
# ==========================================


def get_head_parent_module_index(yaml: dict) -> int:
    """Returns the index of the last layer of the backbone."""
    return len(yaml.get("backbone", [])) - 1


def get_foundation_modulelist(model) -> nn.ModuleList:
    """Extracts the backbone as an nn.ModuleList dynamically."""
    # Note: Using .model.model to get the internal nn.Module architecture
    split_idx = get_head_parent_module_index(model.model.yaml)
    return nn.ModuleList(list(model.model.model.children())[: split_idx + 1])


def get_head_modulelist(model) -> nn.ModuleList:
    """Extracts the neck + task head dynamically."""
    split_idx = get_head_parent_module_index(model.model.yaml)
    return nn.ModuleList(list(model.model.model.children())[split_idx + 1 :])


# ==========================================
# 2. The Dynamic Multi-Task Module
# ==========================================


class DynamicMultiTaskYOLO(nn.Module):
    def __init__(self, foundation_path: str, task_dict: dict):
        super().__init__()

        # 1. Load the foundation model
        print(f"Loading foundation from: {foundation_path}")
        foundation_yolo = YOLO(foundation_path)

        # Store the exact split index dynamically using your helper
        self.split_idx = get_head_parent_module_index(
            foundation_yolo.model.yaml
        )

        # Extract the shared backbone dynamically
        self.shared_backbone = get_foundation_modulelist(foundation_yolo)
        self.save_backbone = foundation_yolo.model.save  # e.g., [4, 6, 9]

        # 2. Extract the Neck + Head for each task dynamically
        self.task_branches = nn.ModuleDict()
        self.branch_saves = {}
        self.task_class_names = {}

        for task_name, model_path in task_dict.items():
            print(f"Extracting {task_name} head from: {model_path}")
            task_yolo = YOLO(model_path)

            # Use helper to extract layers AFTER the backbone
            self.task_branches[task_name] = get_head_modulelist(task_yolo)
            self.branch_saves[task_name] = task_yolo.model.save
            self.task_class_names[task_name] = getattr(
                task_yolo.model, "names", {}
            )

    def forward(self, x):
        # --- PHASE 1: Shared Backbone ---
        y_shared = []  # Store outputs for skip connections

        for i, m in enumerate(self.shared_backbone):
            if m.f != -1:
                x = (
                    y_shared[m.f]
                    if isinstance(m.f, int)
                    else [x if j == -1 else y_shared[j] for j in m.f]
                )
            x = m(x)
            y_shared.append(x if i in self.save_backbone else None)

        # --- PHASE 2: Task-Specific Necks & Heads ---
        outputs = {}

        for task_name, branch in self.task_branches.items():
            # Create a localized copy of the saved backbone features
            y_branch = list(y_shared)
            branch_x = x  # Start with the final output of the backbone

            # Use the dynamic split index to offset the layer numbers properly
            for i, m in enumerate(branch):
                layer_idx = i + self.split_idx + 1

                if m.f != -1:
                    branch_x = (
                        y_branch[m.f]
                        if isinstance(m.f, int)
                        else [branch_x if j == -1 else y_branch[j] for j in m.f]
                    )

                branch_x = m(branch_x)

                # Maintain the skip-connection list for this specific branch
                while len(y_branch) <= layer_idx:
                    y_branch.append(None)
                y_branch[layer_idx] = (
                    branch_x
                    if layer_idx in self.branch_saves[task_name]
                    else None
                )

            outputs[task_name] = branch_x

        return outputs


# ==========================================
# 3. Execution
# ==========================================
def main():
    # Define your specific tasks
    tasks = {"detection": "yolo26m.pt", "pose": "yolo26m-pose.pt"}

    # Build the dynamic unified model
    # It handles all the routing internally based on your helper parsing
    multi_model = DynamicMultiTaskYOLO(
        foundation_path="yolo26m.pt", task_dict=tasks
    )

    predictor = MultiTaskPredictor(multi_model)

    predictions, original_image = predictor.predict(
        "validation/2173162b-cd77-4dec-923b-e28eafd3297c.png"
    )

    pose_shape = predictor.task_meta["pose"]["kpt_shape"]
    detection_class_names = predictor.task_class_names.get("detection")
    visualize_multi_task_predictions(
        original_image,
        predictions,
        kpt_shape=pose_shape,
        save_path="validation/output/test.png",
        detection_class_names=detection_class_names,
    )
    multi_model.eval()


if __name__ == "__main__":
    main()
