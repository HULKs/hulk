from typing import Any, cast

import torch
import torch.nn as nn
from ultralytics import YOLO


def get_head_parent_module_index(yaml_config: dict[str, Any]) -> int:
    """Returns the index of the last layer of the backbone."""
    return len(yaml_config.get("backbone", [])) - 1


def get_foundation_module_list(model: Any) -> nn.ModuleList:
    """Extracts the backbone as an nn.ModuleList dynamically."""
    model_root = cast(Any, model.model)
    split_idx = get_head_parent_module_index(
        cast(dict[str, Any], model_root.yaml)
    )
    return nn.ModuleList(list(model_root.model.children())[: split_idx + 1])


def get_head_modulelist(model: Any) -> nn.ModuleList:
    """Extracts the neck + task head dynamically."""
    model_root = cast(Any, model.model)
    split_idx = get_head_parent_module_index(
        cast(dict[str, Any], model_root.yaml)
    )
    return nn.ModuleList(list(model_root.model.children())[split_idx + 1 :])


class MultiTaskYOLO(nn.Module):
    def __init__(
        self,
        foundation_path: str,
        task_dict: dict[str, str],
    ) -> None:
        super().__init__()

        print(f"Loading foundation from: {foundation_path}")
        foundation_yolo = YOLO(foundation_path)
        foundation_root = cast(Any, foundation_yolo.model)

        self.split_idx = get_head_parent_module_index(
            cast(dict[str, Any], foundation_root.yaml)
        )

        self.shared_backbone = get_foundation_module_list(foundation_yolo)
        self.save_backbone = cast(list[int], foundation_root.save)

        self.task_branches = nn.ModuleDict()
        self.branch_saves = {}
        self.task_class_names = {}

        for task_name, model_path in task_dict.items():
            print(f"Extracting {task_name} head from: {model_path}")
            task_yolo = YOLO(model_path)
            task_root = cast(Any, task_yolo.model)

            self.task_branches[task_name] = get_head_modulelist(task_yolo)
            self.branch_saves[task_name] = cast(list[int], task_root.save)
            self.task_class_names[task_name] = getattr(task_root, "names", {})

    def forward(self, x: torch.Tensor) -> dict[str, torch.Tensor]:
        y_shared: list[torch.Tensor | None] = []
        backbone_input: Any = x

        for i, m in enumerate(self.shared_backbone):
            from_index = cast(Any, getattr(m, "f", -1))
            if from_index != -1:
                backbone_input = (
                    y_shared[from_index]
                    if isinstance(from_index, int)
                    else [
                        backbone_input if j == -1 else y_shared[j]
                        for j in cast(list[int], from_index)
                    ]
                )
            backbone_input = m(backbone_input)
            y_shared.append(backbone_input if i in self.save_backbone else None)

        outputs: dict[str, torch.Tensor] = {}

        for task_name in self.task_branches:
            branch = cast(nn.ModuleList, self.task_branches[task_name])
            y_branch = list(y_shared)
            branch_output: Any = backbone_input

            for i, m in enumerate(branch):
                layer_idx = i + self.split_idx + 1

                from_index = cast(Any, getattr(m, "f", -1))
                if from_index != -1:
                    branch_output = (
                        y_branch[from_index]
                        if isinstance(from_index, int)
                        else [
                            branch_output if j == -1 else y_branch[j]
                            for j in cast(list[int], from_index)
                        ]
                    )

                branch_output = m(branch_output)

                while len(y_branch) <= layer_idx:
                    y_branch.append(None)
                y_branch[layer_idx] = (
                    branch_output
                    if layer_idx in self.branch_saves[task_name]
                    else None
                )

            outputs[task_name] = cast(torch.Tensor, branch_output)

        return outputs
