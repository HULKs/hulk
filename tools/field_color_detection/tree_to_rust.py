from pathlib import Path

import click
import joblib
import lightgbm as lgb
import numpy as np

features = [
    "center",
    "right",
    "top",
    "left",
    "bottom",
]


def sigmoid(x: float) -> float:
    return 1 / (1 + np.exp(-x))


def generate_if_else(tree: dict) -> str:
    if "split_index" not in tree:
        probability = sigmoid(tree["leaf_value"])
        return f"return {probability};"

    feature = tree["split_feature"]
    threshold = tree["threshold"] / 255
    decision_type = tree["decision_type"]
    left = tree["left_child"]
    right = tree["right_child"]
    left_branch = generate_if_else(left)
    right_branch = generate_if_else(right)
    return (
        f"if features.{features[feature]} {decision_type} {threshold} {{\n"
        f"{left_branch}\n"
        "} else {\n"
        f"{right_branch}\n"
        "}"
    )


@click.command()
@click.argument("model-path", type=Path)
@click.argument("output", type=Path)
def main(model_path: Path, output: Path) -> None:
    model: lgb.LGBMClassifier = joblib.load(model_path)
    model_dict = model.booster_.dump_model()
    tree = model_dict["tree_info"][0]["tree_structure"]

    code = (
"""/*
    *********************************** GENERATED CODE ***********************************

    This code was generated from a decision tree model in Python.

    The tool to generate this Rust code can be found here:
        /tools/field_color_detection/tree_to_rust.py
    The input is a joblib file of a LGBMClassifier model and the output is this Rust file.

    **************************************************************************************
*/\n\n"""
    )
    code += "pub struct Features {\n"
    for feature in features:
        code += f"pub {feature}: f32,\n"
    code += "}\n\n"
    code += "#[allow(clippy::collapsible_else_if)]\n"
    code += "pub fn predict(features: &Features) -> f32 {\n"
    code += generate_if_else(tree)
    code += "\n}"

    with open(output, "w") as f:
        f.write(code)


if __name__ == "__main__":
    main()
